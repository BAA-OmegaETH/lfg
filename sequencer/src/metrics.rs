use crate::types::{Batch, Metrics};

pub struct MetricsCollector {
    total_txs: usize,
    total_blobs: usize,
    latencies: Vec<u64>,
    total_uncompressed_size: usize,
    total_compressed_size: usize,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_txs: 0,
            total_blobs: 0,
            latencies: Vec::new(),
            total_uncompressed_size: 0,
            total_compressed_size: 0,
        }
    }

    pub fn record_batch(&mut self, batch: &Batch, batch_close_time_ms: u64, _max_blob_size: usize) {
        self.total_txs += batch.txs.len();
        self.total_blobs += 1;

        self.total_uncompressed_size += batch.total_size;
        self.total_compressed_size += batch.compressed_size;

        for tx in &batch.txs {
            let latency = batch_close_time_ms.saturating_sub(tx.arrival_ms);
            self.latencies.push(latency);
        }
    }

    // We use &mut self here to allow sorting the latencies vector in-place
    pub fn get_metrics(&mut self, max_blob_size: usize) -> Metrics {
        let total_possible_uncompressed_size = (self.total_blobs * max_blob_size) as f64;
        
        let avg_uncompressed_fill_rate = if total_possible_uncompressed_size > 0.0 {
            self.total_uncompressed_size as f64 / total_possible_uncompressed_size
        } else {
            0.0
        };

        let avg_compressed_fill_rate = if total_possible_uncompressed_size > 0.0 {
            self.total_compressed_size as f64 / total_possible_uncompressed_size
        } else {
            0.0
        };

        let avg_latency_ms = if !self.latencies.is_empty() {
            self.latencies.iter().sum::<u64>() as f64 / self.latencies.len() as f64
        } else {
            0.0
        };

        let max_latency_ms = *self.latencies.iter().max().unwrap_or(&0);

        let (p95_latency_ms, p99_latency_ms) = if !self.latencies.is_empty() {
            self.latencies.sort_unstable();
            let p95_index = (self.latencies.len() as f64 * 0.95).floor() as usize;
            let p99_index = (self.latencies.len() as f64 * 0.99).floor() as usize;
            (
                self.latencies.get(p95_index.min(self.latencies.len() - 1)).cloned().unwrap_or(max_latency_ms),
                self.latencies.get(p99_index.min(self.latencies.len() - 1)).cloned().unwrap_or(max_latency_ms),
            )
        } else {
            (0, 0)
        };

        let avg_compression_ratio = if self.total_compressed_size > 0 {
            self.total_uncompressed_size as f64 / self.total_compressed_size as f64
        } else {
            0.0
        };

        Metrics {
            total_txs: self.total_txs,
            total_blobs: self.total_blobs,
            avg_uncompressed_fill_rate,
            avg_compressed_fill_rate,
            avg_latency_ms,
            max_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            avg_compression_ratio,
        }
    }

    pub fn print_summary(&mut self, max_blob_size: usize) {
        let metrics = self.get_metrics(max_blob_size);
        println!("\n=== Metrics Summary ===");
        println!("Total Txs: {}", metrics.total_txs);
        println!("Total Blobs: {}", metrics.total_blobs);
        println!("Avg Uncompressed Fill Rate: {:.2}%", metrics.avg_uncompressed_fill_rate * 100.0);
        println!("Avg Compressed Fill Rate: {:.2}%", metrics.avg_compressed_fill_rate * 100.0);
        println!("Avg Compression Ratio: {:.2}:1", metrics.avg_compression_ratio);
        println!("Avg Latency: {:.2}ms", metrics.avg_latency_ms);
        println!("P95 Latency: {}ms", metrics.p95_latency_ms);
        println!("P99 Latency: {}ms", metrics.p99_latency_ms);
        println!("Max Latency: {}ms", metrics.max_latency_ms);
    }
}