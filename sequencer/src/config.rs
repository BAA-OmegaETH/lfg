use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerConfig {
    /// Ordering policy: "fcfs" or "des"
    pub ordering_policy: String,

    /// DES parameters (alpha, beta, gamma)
    pub des_alpha: f64,
    pub des_beta: f64,
    pub des_gamma: f64,

    /// Blob size in bytes (~128KB)
    pub max_blob_size: usize,

    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,

    /// RPC endpoint for Ethereum devnet
    pub eth_rpc_url: String,

    /// Private key for blob submission
    pub sender_private_key: String,
}

impl Default for SequencerConfig {
    fn default() -> Self {
        Self {
            ordering_policy: std::env::var("ORDERING_POLICY").unwrap_or_else(|_| "fcfs".to_string()),
            des_alpha: 0.33,
            des_beta: 0.33,
            des_gamma: 0.34,
            max_blob_size: 128 * 1024, // 128KB
            batch_timeout_ms: 60_000, // 60 seconds instead of 1 second
            eth_rpc_url: "http://localhost:8545".to_string(),
            sender_private_key: String::new(),
        }
    }
}
