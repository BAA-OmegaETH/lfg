mod batcher;
mod blob_sender;
mod config;
mod executor;
mod mempool;
mod metrics;
mod ordering;
mod types;

use anyhow::Result;
use config::SequencerConfig;
use mempool::Mempool;
use metrics::MetricsCollector;
use ordering::create_ordering_policy;
use types::UserTx;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting OmegaETH Sequencer");

    // Load configuration
    let config = SequencerConfig::default();
    tracing::info!("Ordering policy: {}", config.ordering_policy);

    // Initialize components
    let mut mempool = Mempool::new();
    let ordering_policy = create_ordering_policy(&config);
    let mut executor = executor::Executor::new();
    let batcher = batcher::Batcher::new(&config);
    let blob_sender = blob_sender::BlobSender::new(
        std::env::var("ETH_RPC_URL").unwrap_or_else(|_| config.eth_rpc_url.clone()),
        std::env::var("SENDER_PRIVATE_KEY").unwrap_or_else(|_| config.sender_private_key.clone()),
    );
    let mut metrics = MetricsCollector::new();

    // Load transactions into a pending queue sorted by arrival time
    tracing::info!("Loading transactions from dataset...");

    let dataset = std::env::var("DATASET").unwrap_or_else(|_| "../mixed.csv".to_string());
    tracing::info!("Dataset: {}", dataset);
    let contents = std::fs::read_to_string(&dataset)
        .unwrap_or_else(|_| panic!("Failed to read dataset file: {}", dataset));

    let mut pending: Vec<UserTx> = contents
        .lines()
        .skip(1)
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 6 {
                Some(UserTx::new(
                    parts[0].parse().ok()?,
                    parts[1].parse().ok()?,
                    parts[2].to_string(),
                    parts[3].parse().ok()?,
                    parts[4].to_string(),
                    parts[5].parse().ok()?,
                ))
            } else {
                None
            }
        })
        .collect();

    pending.sort_by_key(|tx| tx.arrival_ms);

    if pending.is_empty() {
        tracing::warn!("No transactions loaded. Exiting.");
        return Ok(());
    }

    tracing::info!("Loaded {} transactions", pending.len());

    let mut next_pending = 0usize;
    let mut sim_clock_ms;

    // batch_start_ms: when the first tx of the current batch arrived (starts the time trigger)
    let mut batch_start_ms: Option<u64> = None;

    let verbose = std::env::var("VERBOSE").unwrap_or_default() == "1";
    let dry_run = std::env::var("DRY_RUN").unwrap_or_default() == "1";
    let ordering_delay_ms: u64 = std::env::var("ORDERING_DELAY_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut submission_count = 0u64;

    // Dual-trigger event-driven loop.
    // Two competing triggers determine when blobs are submitted:
    //   Capacity trigger: mempool accumulated size >= one blob (128KB) → submit immediately
    //   Time trigger:     batch_timeout_ms elapsed since first tx arrived → submit whatever is there
    loop {
        // Find the time of the next event: whichever comes first —
        // the next tx arrival or the time trigger deadline.
        let next_arrival_ms = pending.get(next_pending).map(|tx| tx.arrival_ms);
        let time_trigger_ms = batch_start_ms.map(|s| s + config.batch_timeout_ms);

        let next_event_ms = match (next_arrival_ms, time_trigger_ms) {
            (Some(a), Some(t)) => a.min(t),
            (Some(a), None)    => a,
            (None,    Some(t)) => t,
            (None,    None)    => break,
        };
        sim_clock_ms = next_event_ms;

        // Admit all txs that have arrived by sim_clock_ms.
        // Start the time trigger when the first tx enters an empty batch.
        while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
            let tx = pending[next_pending].clone();
            if batch_start_ms.is_none() {
                batch_start_ms = Some(tx.arrival_ms);
            }
            mempool.add_tx(tx);
            next_pending += 1;
        }

        // Evaluate both triggers.
        let mempool_size: usize = mempool.get_all().iter().map(|tx| tx.payload_size).sum();
        let capacity_fired = mempool_size >= config.max_blob_size;
        let time_fired     = time_trigger_ms.map(|t| sim_clock_ms >= t).unwrap_or(false);

        if (capacity_fired || time_fired) && !mempool.is_empty() {
            let batch_start = batch_start_ms.unwrap_or(sim_clock_ms);
            let actual_window_ms = sim_clock_ms.saturating_sub(batch_start).max(1);

            // Optional deliberate delay: advance sim_clock before ordering to
            // accumulate more txs in the mempool, giving DES a larger pool to score.
            if ordering_delay_ms > 0 {
                sim_clock_ms += ordering_delay_ms;
                while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
                    let tx = pending[next_pending].clone();
                    if batch_start_ms.is_none() {
                        batch_start_ms = Some(tx.arrival_ms);
                    }
                    mempool.add_tx(tx);
                    next_pending += 1;
                }
            }

            let t0 = std::time::Instant::now();
            let ordered_txs = ordering_policy.order(mempool.get_all(), sim_clock_ms, submission_count as usize);
            let algo_elapsed_ms = t0.elapsed().as_micros() as u64 / 1000;
            sim_clock_ms += algo_elapsed_ms;

            // Admit txs that arrived during the ordering algorithm's execution
            while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
                let tx = pending[next_pending].clone();
                if batch_start_ms.is_none() {
                    batch_start_ms = Some(tx.arrival_ms);
                }
                mempool.add_tx(tx);
                next_pending += 1;
            }

            let batches = batcher.create_batches(ordered_txs)?;

            if capacity_fired {
                // Capacity trigger: submit only the FIRST blob, leave the rest in the mempool.
                // The timer keeps running — remaining txs continue accumulating wait time.
                let batch = &batches[0];

                if verbose {
                    let fill_pct = batch.total_size as f64 / config.max_blob_size as f64 * 100.0;
                    let comp_pct = batch.compressed_size as f64 / config.max_blob_size as f64 * 100.0;
                    println!(
                        "blob={:3} trigger=capacity txs={:4} uncomp={:6.1}KB ({:5.1}%) comp={:6.1}KB ({:5.1}%) window={:.1}s",
                        submission_count,
                        batch.txs.len(),
                        batch.total_size as f64 / 1024.0,
                        fill_pct,
                        batch.compressed_size as f64 / 1024.0,
                        comp_pct,
                        actual_window_ms as f64 / 1000.0,
                    );
                }

                metrics.record_ordering_latencies(&batch.txs, sim_clock_ms);
                executor.execute_batch(&batch.txs)?;
                metrics.record_batch(batch, config.max_blob_size);

                if !dry_run {
                    match blob_sender.send_blob(batch).await {
                        Ok((tx_hash, _)) => tracing::info!("Blob sent: {}", tx_hash),
                        Err(e) => tracing::error!("Failed to send blob: {}", e),
                    }
                }

                let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
                mempool.remove_txs(&tx_ids);

                // Record inclusion latency via slot boundary math, but do NOT advance
                // sim_clock_ms — the ordering clock is driven purely by dataset arrival
                // times so that ordering latency reflects mempool wait time only.
                let slot_ms: u64 = 6_000; // devnet uses 6s slots (network_params.yaml: seconds_per_slot: 6)
                let t_inclusion = ((sim_clock_ms + slot_ms) / slot_ms) * slot_ms;
                metrics.record_inclusion(t_inclusion - sim_clock_ms);
                // Do NOT reset batch_start_ms — timer continues for remaining txs.

            } else {
                // Time trigger: mempool didn't fill a full blob in time.
                // Submit all remaining txs as a partial blob, then reset.
                let all_txs: Vec<_> = batches.iter().flat_map(|b| b.txs.iter().cloned()).collect();
                metrics.record_ordering_latencies(&all_txs, sim_clock_ms);
                executor.execute_batch(&all_txs)?;

                // Record inclusion latency via slot boundary math, but do NOT advance
                // sim_clock_ms — ordering clock stays driven by dataset arrival times.
                let slot_ms: u64 = 6_000; // devnet uses 6s slots (network_params.yaml: seconds_per_slot: 6)
                let t_inclusion = ((sim_clock_ms + slot_ms) / slot_ms) * slot_ms;
                let inclusion_latency_ms = t_inclusion - sim_clock_ms;

                for batch in batches.iter() {
                    if verbose {
                        let fill_pct = batch.total_size as f64 / config.max_blob_size as f64 * 100.0;
                        let comp_pct = batch.compressed_size as f64 / config.max_blob_size as f64 * 100.0;
                        println!(
                            "blob={:3} trigger=time    txs={:4} uncomp={:6.1}KB ({:5.1}%) comp={:6.1}KB ({:5.1}%) window={:.1}s",
                            submission_count,
                            batch.txs.len(),
                            batch.total_size as f64 / 1024.0,
                            fill_pct,
                            batch.compressed_size as f64 / 1024.0,
                            comp_pct,
                            actual_window_ms as f64 / 1000.0,
                        );
                    }
                    metrics.record_batch(batch, config.max_blob_size);

                    if !dry_run {
                        match blob_sender.send_blob(batch).await {
                            Ok((tx_hash, _)) => tracing::info!("Blob sent: {}", tx_hash),
                            Err(e) => tracing::error!("Failed to send blob: {}", e),
                        }
                    }

                    let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
                    mempool.remove_txs(&tx_ids);
                    metrics.record_inclusion(inclusion_latency_ms);
                }

                // Reset timer — restarts when the next tx arrives.
                batch_start_ms = None;
            }

            submission_count += 1;
        }

        if next_pending >= pending.len() && mempool.is_empty() {
            break;
        }
    }

    // Print final metrics
    metrics.print_summary(config.max_blob_size);

    Ok(())
}
