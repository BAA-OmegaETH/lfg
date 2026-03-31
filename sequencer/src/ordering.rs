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
}

impl DesOrdering {
    pub fn new(config: &SequencerConfig) -> Self {
        Self {
            alpha: config.des_alpha,
            beta: config.des_beta,
            gamma: config.des_gamma,
        }
    }

    fn calculate_score(&self, tx: &UserTx, current_time_ms: u64, current_batch_size: usize, max_blob_size: usize) -> f64 {
        let wait_time = (current_time_ms - tx.arrival_ms) as f64;
        let wait_score = wait_time / 1000.0; // normalize to seconds

        // Simple compression heuristic (tx_type based)
        let compress_score = match tx.tx_type.as_str() {
            "transfer" => 0.9,
            "swap" => 0.7,
            "mint" => 0.5,
            _ => 0.6,
        };

        // Fit score: how well it fits current batch
        let remaining_space = max_blob_size.saturating_sub(current_batch_size);
        let fit_score = if tx.payload_size <= remaining_space {
            1.0 - (tx.payload_size as f64 / remaining_space as f64)
        } else {
            0.0
        };

        self.alpha * wait_score + self.beta * compress_score + self.gamma * fit_score
    }
}

impl OrderingPolicy for DesOrdering {
    fn order(&self, mut txs: Vec<UserTx>) -> Vec<UserTx> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        txs.sort_by(|a, b| {
            let score_a = self.calculate_score(a, current_time, 0, 128 * 1024);
            let score_b = self.calculate_score(b, current_time, 0, 128 * 1024);
            score_b.partial_cmp(&score_a).unwrap()
        });

        txs
    }
}

pub fn create_ordering_policy(config: &SequencerConfig) -> Box<dyn OrderingPolicy> {
    match config.ordering_policy.as_str() {
        "des" => Box::new(DesOrdering::new(config)),
        _ => Box::new(FcfsOrdering),
    }
}
