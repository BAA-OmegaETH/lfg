use crate::config::SequencerConfig;
use crate::types::UserTx;

pub trait OrderingPolicy {
    fn order(&self, txs: Vec<UserTx>) -> Vec<UserTx>;
}

pub struct FcfsOrdering;

impl OrderingPolicy for FcfsOrdering {
    fn order(&self, mut txs: Vec<UserTx>) -> Vec<UserTx> {
        txs.sort_by_key(|tx| tx.arrival_ms);
        txs
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

    fn calculate_score(&self, tx: &UserTx, current_time_ms: u64, current_batch_size: usize) -> f64 {
        let wait_time = (current_time_ms - tx.arrival_ms) as f64;
        let wait_score = wait_time / 1000.0; // normalize to seconds

        // Simple compression heuristic
        let compress_score = match tx.tx_type.as_str() {
            "transfer" => 0.9,
            "swap" => 0.7,
            "mint" => 0.5,
            _ => 0.6,
        };

        // Fit score: how well it fits current batch
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
    fn order(&self, txs: Vec<UserTx>) -> Vec<UserTx> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut ordered_txs = Vec::with_capacity(txs.len());
        let mut unprocessed_txs = txs;
        let mut current_batch_size = 0;

        while !unprocessed_txs.is_empty() {
            // Find the best transaction strictly among those that fit in the current batch.
            let best_fit_index = unprocessed_txs
                .iter()
                .enumerate()
                .filter(|(_, tx)| tx.payload_size + current_batch_size <= self.max_blob_size)
                .max_by(|(_, a), (_, b)| {
                    let score_a = self.calculate_score(a, current_time, current_batch_size);
                    let score_b = self.calculate_score(b, current_time, current_batch_size);
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);

            if let Some(index) = best_fit_index {
                // A fitting tx was found. Add it to the batch.
                let best_tx = unprocessed_txs.remove(index);
                current_batch_size += best_tx.payload_size;
                ordered_txs.push(best_tx);
            } else {
                // No remaining tx fits in the current batch. Start a new batch.
                current_batch_size = 0;

                // Find the best tx for an *empty* batch
                let best_for_new_batch_index = unprocessed_txs
                    .iter()
                    .enumerate()
                    .filter(|(_, tx)| tx.payload_size <= self.max_blob_size) 
                    .max_by(|(_, a), (_, b)| {
                        let score_a = self.calculate_score(a, current_time, 0);
                        let score_b = self.calculate_score(b, current_time, 0);
                        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i);

                if let Some(index) = best_for_new_batch_index {
                    let best_tx = unprocessed_txs.remove(index);
                    current_batch_size += best_tx.payload_size;
                    ordered_txs.push(best_tx);
                } else {
                    // Oversized transactions. Append to end.
                    ordered_txs.append(&mut unprocessed_txs);
                    break;
                }
            }
        }

        ordered_txs
    }
}

pub fn create_ordering_policy(config: &SequencerConfig) -> Box<dyn OrderingPolicy> {
    match config.ordering_policy.as_str() {
        "des" => Box::new(DesOrdering::new(config)),
        _ => Box::new(FcfsOrdering),
    }
}