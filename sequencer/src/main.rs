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

            let ordered_txs = ordering_policy.order(mempool.get_all(), sim_clock_ms, submission_count as usize);
            let batches = batcher.create_batches(ordered_txs)?;

            if capacity_fired {
                // Capacity trigger: submit only the FIRST blob, leave the rest in the mempool.
                // The timer keeps running — remaining txs continue accumulating wait time.
                let batch = &batches[0];

                if verbose {
                    println!(
                        "submission={} trigger=capacity window={:.1}s mempool_kb={:.1} submitting=1/{} blobs",
                        submission_count,
                        actual_window_ms as f64 / 1000.0,
                        mempool_size as f64 / 1024.0,
                        batches.len(),
                    );
                }

                metrics.record_ordering_latencies(&batch.txs, sim_clock_ms);
                executor.execute_batch(&batch.txs)?;
                metrics.record_batch(batch, config.max_blob_size);

                match blob_sender.send_blob(batch).await {
                    Ok((tx_hash, _)) => tracing::info!("Blob sent: {}", tx_hash),
                    Err(e) => tracing::error!("Failed to send blob: {}", e),
                }

                let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
                mempool.remove_txs(&tx_ids);

                // Snap to next 6s L1 slot boundary for simulated inclusion time.
                let slot_ms: u64 = 6_000; // devnet uses 6s slots (network_params.yaml: seconds_per_slot: 6)
                let t_inclusion = ((sim_clock_ms + slot_ms) / slot_ms) * slot_ms;
                metrics.record_inclusion(t_inclusion - sim_clock_ms);
                sim_clock_ms = t_inclusion;

                // Admit txs that arrived while the blob was being submitted.
                while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
                    let tx = pending[next_pending].clone();
                    if batch_start_ms.is_none() {
                        batch_start_ms = Some(tx.arrival_ms);
                    }
                    mempool.add_tx(tx);
                    next_pending += 1;
                }
                // Do NOT reset batch_start_ms — timer continues for remaining txs.

            } else {
                // Time trigger: mempool didn't fill a full blob in time.
                // Submit all remaining txs as a partial blob, then reset.
                if verbose {
                    println!(
                        "submission={} trigger=time window={:.1}s mempool_kb={:.1} blobs={}",
                        submission_count,
                        actual_window_ms as f64 / 1000.0,
                        mempool_size as f64 / 1024.0,
                        batches.len(),
                    );
                }

                let all_txs: Vec<_> = batches.iter().flat_map(|b| b.txs.iter().cloned()).collect();
                metrics.record_ordering_latencies(&all_txs, sim_clock_ms);
                executor.execute_batch(&all_txs)?;

                // All blobs submitted at the same logical moment land in the same slot.
                let slot_ms: u64 = 6_000; // devnet uses 6s slots (network_params.yaml: seconds_per_slot: 6)
                let t_inclusion = ((sim_clock_ms + slot_ms) / slot_ms) * slot_ms;
                let inclusion_latency_ms = t_inclusion - sim_clock_ms;

                for batch in batches.iter() {
                    metrics.record_batch(batch, config.max_blob_size);

                    match blob_sender.send_blob(batch).await {
                        Ok((tx_hash, _)) => tracing::info!("Blob sent: {}", tx_hash),
                        Err(e) => tracing::error!("Failed to send blob: {}", e),
                    }

                    let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
                    mempool.remove_txs(&tx_ids);
                    metrics.record_inclusion(inclusion_latency_ms);
                }

                sim_clock_ms = t_inclusion;
                // Reset timer, then admit txs that arrived during the slot window.
                batch_start_ms = None;
                while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
                    let tx = pending[next_pending].clone();
                    if batch_start_ms.is_none() {
                        batch_start_ms = Some(tx.arrival_ms);
                    }
                    mempool.add_tx(tx);
                    next_pending += 1;
                }
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
