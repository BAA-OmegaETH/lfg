use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTx {
    pub tx_id: u64,
    pub arrival_ms: u64,
    pub payload_size: usize,
    pub tx_type: String,
    pub gas_bid: u64,
    pub data: Vec<u8>,
    pub from: String,
    pub nonce: u64,
}

impl UserTx {
    pub fn new(tx_id: u64, payload_size: usize, tx_type: String, arrival_ms: u64, from: String, nonce: u64) -> Self {
        let data = Self::generate_typed_data(tx_id, arrival_ms, payload_size, &tx_type);
        Self { tx_id, arrival_ms, payload_size, tx_type, gas_bid: 0, data, from, nonce }
    }

    fn generate_typed_data(tx_id: u64, arrival_ms: u64, payload_size: usize, tx_type: &str) -> Vec<u8> {
        // Zero-byte density models real Ethereum ABI calldata compressibility per tx type.
        // Higher zero density → more compressible → matches compress_score ordering.
        //   transfer: address(12B zeros+20B) + uint256(28B zeros+4B) → ~75% zeros → 4-6:1 zstd
        //   mint:     similar ABI structure, slightly more varied        → ~60% zeros → 3-4:1 zstd
        //   swap:     path arrays + multi-param → complex, fewer zeros  → ~35% zeros → 2-3:1 zstd
        //   other:    unstructured calldata                              → ~15% zeros → ~1.5:1 zstd
        let zero_density: f64 = match tx_type {
            "transfer" => 0.75,
            "mint"     => 0.60,
            "swap"     => 0.35,
            _          => 0.15,
        };

        let mut rng_state = tx_id.wrapping_add(arrival_ms).wrapping_add(1);
        let mut next_random = || -> u8 {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 17;
            rng_state ^= rng_state << 5;
            (rng_state % 256) as u8
        };

        let mut data = Vec::with_capacity(payload_size);

        // First 4 bytes: function selector (always non-zero)
        for _ in 0..4.min(payload_size) {
            let b = next_random();
            data.push(if b == 0 { 1 } else { b });
        }

        // Remaining bytes: type-correlated zero density
        while data.len() < payload_size {
            // Use 256 thresholds to avoid float ops in the hot path
            let threshold = (zero_density * 256.0) as u8;
            let probe = next_random();
            if probe < threshold {
                data.push(0);
            } else {
                let b = next_random();
                data.push(if b == 0 { 1 } else { b }); // ensure non-zero
            }
        }

        data
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
    pub avg_compression_ratio: f64,
}