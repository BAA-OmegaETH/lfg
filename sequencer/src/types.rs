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
    pub fn new(tx_id: u64, payload_size: usize, tx_type: String, arrival_ms: u64) -> Self {
        let mut data = Vec::with_capacity(payload_size);
        
        // Seed a simple pseudo-random generator for deterministic payloads
        let mut rng_state = tx_id.wrapping_add(arrival_ms).wrapping_add(1);
        let mut next_random = || -> u8 {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 17;
            rng_state ^= rng_state << 5;
            (rng_state % 256) as u8
        };

        // 1. Simulate an Ethereum Function Selector (First 4 bytes)
        for _ in 0..4.min(payload_size) {
            data.push(next_random());
        }

        // 2. Build the rest of the payload using 32-byte ABI encoded words
        while data.len() < payload_size {
            let remaining = payload_size - data.len();
            if remaining >= 32 {
                // Flip a coin: Is this an Address or a Uint256 amount?
                if next_random() % 2 == 0 {
                    // Simulate an Address (12 bytes of zero padding + 20 bytes of random address)
                    for _ in 0..12 { data.push(0); }
                    for _ in 0..20 { data.push(next_random()); }
                } else {
                    // Simulate a Uint256 Token Amount (28 bytes of zero padding + 4 bytes of random value)
                    for _ in 0..28 { data.push(0); }
                    for _ in 0..4 { data.push(next_random()); }
                }
            } else {
                // Fill the remaining tail with random bytes (simulating a signature or other data)
                data.push(next_random());
            }
        }

        Self {
            tx_id,
            arrival_ms,
            payload_size,
            tx_type,
            gas_bid: 0, // gas_bid is not used in this simulation phase
            data,
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
    pub avg_uncompressed_fill_rate: f64,
    pub avg_compressed_fill_rate: f64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub avg_compression_ratio: f64,
}