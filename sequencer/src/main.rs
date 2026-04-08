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
    let blob_sender = blob_sender::BlobSender::new(config.eth_rpc_url.clone());
    let mut metrics = MetricsCollector::new();

    // Load transactions from the realistic dataset
    tracing::info!("Loading transactions from dataset...");
    
    // We will test the 'mixed' scenario first
    let contents = std::fs::read_to_string("../mixed.csv")
        .expect("Failed to read CSV file. Make sure mixed.csv exists in the root folder.");
    
    for line in contents.lines().skip(1) { // Skip the CSV header row
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() == 4 {
            let tx = UserTx::new(
                parts[0].parse().unwrap(), // tx_id
                parts[1].parse().unwrap(), // payload_size
                parts[2].to_string(),      // tx_type
                parts[3].parse().unwrap(), // arrival_ms
            );
            mempool.add_tx(tx);
        }
    }

    tracing::info!("Processing {} transactions", mempool.len());

    // Main sequencer loop
    while !mempool.is_empty() {
        // Get transactions from mempool
        let txs = mempool.get_all();

        // Order transactions
        let ordered_txs = ordering_policy.order(txs);

        // Execute transactions
        executor.execute_batch(&ordered_txs)?;

        // Create batches
        let batches = batcher.create_batches(ordered_txs.clone())?;

        // Send blobs and collect metrics
        for batch in batches {
            metrics.record_batch(&batch, config.max_blob_size);

            match blob_sender.send_blob(&batch).await {
                Ok(tx_hash) => tracing::info!("Blob sent: {}", tx_hash),
                Err(e) => tracing::error!("Failed to send blob: {}", e),
            }

            // Remove processed txs
            let tx_ids: Vec<u64> = batch.txs.iter().map(|tx| tx.tx_id).collect();
            mempool.remove_txs(&tx_ids);
        }

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(config.batch_timeout_ms)).await;
    }

    // Print final metrics
    metrics.print_summary(config.max_blob_size);

    Ok(())
}
