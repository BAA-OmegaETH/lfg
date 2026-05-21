use crate::types::{Batch, Metrics};

pub struct MetricsCollector {
    total_txs: usize,
    total_blobs: usize,
    total_uncompressed_size: usize,
    total_compressed_size: usize,
    per_blob_ratios: Vec<f64>,
    inclusion_latencies_ms: Vec<u64>,
    ordering_latencies_ms: Vec<u64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_txs: 0,
            total_blobs: 0,
            total_uncompressed_size: 0,
            total_compressed_size: 0,
            per_blob_ratios: Vec::new(),
            inclusion_latencies_ms: Vec::new(),
            ordering_latencies_ms: Vec::new(),
        }
    }

    pub fn record_batch(&mut self, batch: &Batch, _max_blob_size: usize) {
        self.total_txs += batch.txs.len();
        self.total_blobs += 1;

        self.total_uncompressed_size += batch.total_size;
        self.total_compressed_size += batch.compressed_size;

        if batch.compressed_size > 0 {
            let ratio = batch.total_size as f64 / batch.compressed_size as f64;
            self.per_blob_ratios.push(ratio);
        }
    }

    pub fn record_inclusion(&mut self, inclusion_latency_ms: u64) {
        if inclusion_latency_ms > 0 {
            self.inclusion_latencies_ms.push(inclusion_latency_ms);
        }
    }

    /// Records ordering latency for each tx: (seal_time_ms) - (tx.arrival_ms).
    /// seal_time_ms is sim_clock_ms at the moment the trigger fired and the ordering ran.
    pub fn record_ordering_latencies(
        &mut self,
        txs: &[crate::types::UserTx],
        seal_time_ms: u64,
    ) {
        for tx in txs.iter() {
            let latency = seal_time_ms.saturating_sub(tx.arrival_ms);
            self.ordering_latencies_ms.push(latency);
        }
    }

    pub fn get_metrics(&self, max_blob_size: usize) -> Metrics {
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

        if !self.ordering_latencies_ms.is_empty() {
            let n = self.ordering_latencies_ms.len();
            let avg = self.ordering_latencies_ms.iter().sum::<u64>() as f64 / n as f64;
            let mut sorted = self.ordering_latencies_ms.clone();
            sorted.sort_unstable();
            let p95 = sorted[(n as f64 * 0.95) as usize].min(*sorted.last().unwrap());
            let max = *sorted.last().unwrap();
            println!("\n--- Ordering Latency (arrival → selected into blob) ---");
            println!("  Avg: {:.0}ms  P95: {}ms  Max: {}ms", avg, p95, max);
            println!("  (measures how long each tx waited before the ordering algo selected it)");
        }

        if !self.inclusion_latencies_ms.is_empty() {
            let n = self.inclusion_latencies_ms.len();
            let avg = self.inclusion_latencies_ms.iter().sum::<u64>() as f64 / n as f64;
            let mut sorted = self.inclusion_latencies_ms.clone();
            sorted.sort_unstable();
            let p50 = sorted[n / 2];
            let max = *sorted.last().unwrap();
            println!("\n--- Blob Inclusion Latency (on-chain) ---");
            println!("  Blobs submitted: {}", n);
            println!("  Avg: {:.0}ms  P50: {}ms  Max: {}ms", avg, p50, max);
        }

        // Per-blob compression breakdown
        if !self.per_blob_ratios.is_empty() {
            let n = self.per_blob_ratios.len() as f64;
            let mean = self.per_blob_ratios.iter().sum::<f64>() / n;
            let variance = self.per_blob_ratios.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
            let stddev = variance.sqrt();
            let min = self.per_blob_ratios.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = self.per_blob_ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            // Histogram: <1.0, 1.0-1.5, 1.5-2.0, 2.0-3.0, >3.0
            let buckets = [
                ("<1.0:1",   self.per_blob_ratios.iter().filter(|&&r| r < 1.0).count()),
                ("1.0-1.5", self.per_blob_ratios.iter().filter(|&&r| r >= 1.0 && r < 1.5).count()),
                ("1.5-2.0", self.per_blob_ratios.iter().filter(|&&r| r >= 1.5 && r < 2.0).count()),
                ("2.0-3.0", self.per_blob_ratios.iter().filter(|&&r| r >= 2.0 && r < 3.0).count()),
                (">3.0:1",  self.per_blob_ratios.iter().filter(|&&r| r >= 3.0).count()),
            ];

            println!("\n--- Per-Blob Compression Distribution ---");
            println!("  Min: {:.2}:1  Max: {:.2}:1  StdDev: {:.3}", min, max, stddev);
            for (label, count) in &buckets {
                let pct = *count as f64 / n * 100.0;
                println!("  {:8}  {:4} blobs ({:5.1}%)", label, count, pct);
            }
        }
    }
}
