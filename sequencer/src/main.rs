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

    // Virtual clock starts at the first tx's arrival time
    let mut sim_clock_ms = pending[0].arrival_ms;
    let mut next_pending = 0;

    // Main sequencer loop: advance sim_clock one batch window per iteration
    let verbose = std::env::var("VERBOSE").unwrap_or_default() == "1";
    let mut tick = 0u64;

    loop {
        // Admit all txs that have arrived by sim_clock_ms
        let admitted_before = mempool.get_all().len();
        while next_pending < pending.len() && pending[next_pending].arrival_ms <= sim_clock_ms {
            mempool.add_tx(pending[next_pending].clone());
            next_pending += 1;
        }
        let txs_admitted = mempool.get_all().len() - admitted_before;

        if !mempool.is_empty() {
            let txs = mempool.get_all();
            let txs_in_window = txs.len();
            let ordered_txs = ordering_policy.order(txs, sim_clock_ms);

            executor.execute_batch(&ordered_txs)?;

            let batches = batcher.create_batches(ordered_txs)?;
            let blobs_this_tick = batches.len();

            if verbose {
                println!(
                    "tick={} admitted={} window_total={} blobs={}",
                    tick, txs_admitted, txs_in_window, blobs_this_tick
                );
            }

            for (blob_index, batch) in batches.iter().enumerate() {
                let blob_close_time_ms = sim_clock_ms + (blob_index as u64 * config.blob_submission_delay_ms);
                metrics.record_batch(batch, blob_close_time_ms, config.max_blob_size);

                match blob_sender.send_blob(&batch).await {
                    Ok((tx_hash, inclusion_ms)) => {
                        tracing::info!("Blob sent: {}", tx_hash);
                        metrics.record_inclusion(inclusion_ms);
                    }
                    Err(e) => tracing::error!("Failed to send blob: {}", e),
                }

                let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
                mempool.remove_txs(&tx_ids);
            }
        }

        if next_pending >= pending.len() && mempool.is_empty() {
            break;
        }

        tick += 1;
        sim_clock_ms += config.batch_timeout_ms;
    }

    // Print final metrics
    metrics.print_summary(config.max_blob_size);

    Ok(())
}
