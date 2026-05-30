use std::collections::HashMap;

use crate::config::SequencerConfig;
use crate::types::UserTx;

pub trait OrderingPolicy {
    fn order(&self, txs: Vec<UserTx>, sim_clock_ms: u64, blob_offset: usize) -> Vec<UserTx>;
}

/// Groups txs by sender and sorts each sender's queue by nonce.
/// This enforces the hard constraint that a sender's txs must execute in nonce order.
fn build_sender_queues(txs: Vec<UserTx>) -> HashMap<String, Vec<UserTx>> {
    let mut queues: HashMap<String, Vec<UserTx>> = HashMap::new();
    for tx in txs {
        queues.entry(tx.from.clone()).or_default().push(tx);
    }
    for queue in queues.values_mut() {
        queue.sort_by_key(|tx| tx.nonce);
    }
    queues
}

pub struct FcfsOrdering;

impl OrderingPolicy for FcfsOrdering {
    fn order(&self, txs: Vec<UserTx>, _sim_clock_ms: u64, _blob_offset: usize) -> Vec<UserTx> {
        let mut queues = build_sender_queues(txs);
        let mut result = Vec::new();

        while queues.values().any(|q| !q.is_empty()) {
            // Pick the head-of-line tx with the earliest arrival_ms across all senders
            let best_sender = queues
                .iter()
                .filter(|(_, q)| !q.is_empty())
                .min_by_key(|(_, q)| q[0].arrival_ms)
                .map(|(s, _)| s.clone())
                .unwrap();

            let tx = queues.get_mut(&best_sender).unwrap().remove(0);
            result.push(tx);
        }

        result
    }
}

pub struct DesOrdering {
    alpha: f64,
    beta: f64,
    gamma: f64,
    max_blob_size: usize,
}

impl DesOrdering {
    pub fn new(config: &SequencerConfig) -> Self {
        Self {
            alpha: config.des_alpha,
            beta: config.des_beta,
            gamma: config.des_gamma,
            max_blob_size: config.max_blob_size,
        }
    }

    fn print_tx_scores(&self, tx: &UserTx, sim_clock_ms: u64, current_batch_size: usize, blob_index: usize) {
        let wait_time = sim_clock_ms.saturating_sub(tx.arrival_ms) as f64;
        let lambda = 0.0001_f64;
        let wait_score = 1.0 - (-lambda * wait_time).exp();
        let compress_score = (1.0 - (tx.payload_size as f64 / 10_000.0)).clamp(0.1, 1.0);
        let remaining = self.max_blob_size.saturating_sub(current_batch_size);
        let fit_score = if tx.payload_size <= remaining && remaining > 0 {
            tx.payload_size as f64 / remaining as f64
        } else {
            0.0
        };
        let des = self.alpha * wait_score + self.beta * compress_score + self.gamma * fit_score;
        println!(
            "blob={:3} tx={:6} type={:8} size={:5}B  wait={:.3} compress={:.3} fit={:.3} des={:.4}",
            blob_index, tx.tx_id, tx.tx_type, tx.payload_size,
            wait_score, compress_score, fit_score, des,
        );
    }

    fn calculate_score(&self, tx: &UserTx, sim_clock_ms: u64, current_batch_size: usize) -> f64 {
        let wait_time = sim_clock_ms.saturating_sub(tx.arrival_ms) as f64;
        // Exponential aging: score climbs quickly out of zero without ever hard-clamping.
        // lambda=0.0001 → 4s wait ≈ 0.33, 10s ≈ 0.63, 60s ≈ 1.00
        let lambda = 0.0001_f64;
        let wait_score = 1.0 - (-lambda * wait_time).exp();

        // Compress score: smaller txs have simpler ABI structure (more zero-padding) and
        // compress better. 10,000 bytes is chosen as the practical upper bound for calldata
        // in this dataset; score is clamped to [0.1, 1.0].
        let compress_score = (1.0 - (tx.payload_size as f64 / 10_000.0)).clamp(0.1, 1.0);

        let remaining_space = self.max_blob_size.saturating_sub(current_batch_size);
        let fit_score = if tx.payload_size <= remaining_space && remaining_space > 0 {
            tx.payload_size as f64 / remaining_space as f64
        } else {
            0.0
        };

        self.alpha * wait_score + self.beta * compress_score + self.gamma * fit_score
    }
}

impl OrderingPolicy for DesOrdering {
    fn order(&self, txs: Vec<UserTx>, sim_clock_ms: u64, blob_offset: usize) -> Vec<UserTx> {
        let verbose = std::env::var("DES_VERBOSE").unwrap_or_default() == "1";
        let mut queues = build_sender_queues(txs);
        let mut result = Vec::new();
        let mut current_batch_size = 0;
        let mut blob_index = blob_offset;

        while queues.values().any(|q| !q.is_empty()) {
            // Among head-of-line txs that fit in the current batch, pick the highest score
            let best_fitting = queues
                .iter()
                .filter(|(_, q)| !q.is_empty())
                // .filter(|(_, q)| q[0].payload_size + current_batch_size <= self.max_blob_size)
                .max_by(|(_, qa), (_, qb)| {
                    let score_a = self.calculate_score(&qa[0], sim_clock_ms, current_batch_size);
                    let score_b = self.calculate_score(&qb[0], sim_clock_ms, current_batch_size);
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(s, _)| s.clone());

            if let Some(sender) = best_fitting {
                let tx = queues.get_mut(&sender).unwrap().remove(0);
                if verbose {
                    self.print_tx_scores(&tx, sim_clock_ms, current_batch_size, blob_index);
                }
                current_batch_size += tx.payload_size;
                result.push(tx);
            } else {
                // Nothing fits in the current batch — start a new one
                current_batch_size = 0;
                blob_index += 1;

                let best_for_new = queues
                    .iter()
                    .filter(|(_, q)| !q.is_empty())
                    // .filter(|(_, q)| q[0].payload_size <= self.max_blob_size)
                    .max_by(|(_, qa), (_, qb)| {
                        let score_a = self.calculate_score(&qa[0], sim_clock_ms, 0);
                        let score_b = self.calculate_score(&qb[0], sim_clock_ms, 0);
                        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(s, _)| s.clone());

                if let Some(sender) = best_for_new {
                    let tx = queues.get_mut(&sender).unwrap().remove(0);
                    if verbose {
                        self.print_tx_scores(&tx, sim_clock_ms, 0, blob_index);
                    }
                    current_batch_size += tx.payload_size;
                    result.push(tx);
                } else {
                    // All remaining txs are oversized — append and exit
                    for queue in queues.values_mut() {
                        result.append(queue);
                    }
                    break;
                }
            }
        }

        result
    }
}

pub fn create_ordering_policy(config: &SequencerConfig) -> Box<dyn OrderingPolicy> {
    match config.ordering_policy.as_str() {
        "des" => Box::new(DesOrdering::new(config)),
        _ => Box::new(FcfsOrdering),
    }
}
