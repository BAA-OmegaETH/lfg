use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTx {
    pub tx_id: u64,
    pub arrival_ms: u64,
    pub payload_size: usize,
    pub tx_type: String,
    pub gas_bid: u64,
    pub data: Vec<u8>,
}

impl UserTx {
    pub fn new(tx_id: u64, payload_size: usize, tx_type: String, gas_bid: u64) -> Self {
        let arrival_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            tx_id,
            arrival_ms,
            payload_size,
            tx_type,
            gas_bid,
            data: vec![0u8; payload_size],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Batch {
    pub txs: Vec<UserTx>,
    pub total_size: usize,
    pub compressed_size: usize,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub total_txs: usize,
    pub total_blobs: usize,
    pub avg_fill_rate: f64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: u64,
}
