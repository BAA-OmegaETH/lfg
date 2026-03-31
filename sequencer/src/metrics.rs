use crate::types::{Batch, Metrics, UserTx};

pub struct MetricsCollector {
    total_txs: usize,
    total_blobs: usize,
    total_fill_rate: f64,
    latencies: Vec<u64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_txs: 0,
            total_blobs: 0,
            total_fill_rate: 0.0,
            latencies: Vec::new(),
        }
    }

    pub fn record_batch(&mut self, batch: &Batch, max_blob_size: usize) {
        self.total_txs += batch.txs.len();
        self.total_blobs += 1;

        let fill_rate = batch.compressed_size as f64 / max_blob_size as f64;
        self.total_fill_rate += fill_rate;

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        for tx in &batch.txs {
            let latency = current_time - tx.arrival_ms;
            self.latencies.push(latency);
        }
    }

    pub fn get_metrics(&self) -> Metrics {
        let avg_fill_rate = if self.total_blobs > 0 {
            self.total_fill_rate / self.total_blobs as f64
        } else {
            0.0
        };

        let avg_latency_ms = if !self.latencies.is_empty() {
            self.latencies.iter().sum::<u64>() as f64 / self.latencies.len() as f64
        } else {
            0.0
        };

        let max_latency_ms = *self.latencies.iter().max().unwrap_or(&0);

        Metrics {
            total_txs: self.total_txs,
            total_blobs: self.total_blobs,
            avg_fill_rate,
            avg_latency_ms,
            max_latency_ms,
        }
    }

    pub fn print_summary(&self) {
        let metrics = self.get_metrics();
        println!("\n=== Metrics Summary ===");
        println!("Total Txs: {}", metrics.total_txs);
        println!("Total Blobs: {}", metrics.total_blobs);
        println!("Avg Fill Rate: {:.2}%", metrics.avg_fill_rate * 100.0);
        println!("Avg Latency: {:.2}ms", metrics.avg_latency_ms);
        println!("Max Latency: {}ms", metrics.max_latency_ms);
    }
}
