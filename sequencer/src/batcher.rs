use crate::config::SequencerConfig;
use crate::types::{Batch, UserTx};
use anyhow::Result;

pub struct Batcher {
    max_blob_size: usize,
}

impl Batcher {
    pub fn new(config: &SequencerConfig) -> Self {
        Self {
            max_blob_size: config.max_blob_size,
        }
    }

    pub fn create_batches(&self, txs: Vec<UserTx>) -> Result<Vec<Batch>> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for tx in txs {
            if current_size + tx.payload_size > self.max_blob_size && !current_batch.is_empty() {
                // Compress and create batch
                let compressed_size = self.compress_batch(&current_batch)?;
                batches.push(Batch {
                    txs: current_batch.clone(),
                    total_size: current_size,
                    compressed_size,
                });

                current_batch.clear();
                current_size = 0;
            }

            current_batch.push(tx.clone());
            current_size += tx.payload_size;
        }

        // Handle remaining txs
        if !current_batch.is_empty() {
            let compressed_size = self.compress_batch(&current_batch)?;
            batches.push(Batch {
                txs: current_batch,
                total_size: current_size,
                compressed_size,
            });
        }

        Ok(batches)
    }

    fn compress_batch(&self, txs: &[UserTx]) -> Result<usize> {
        // Serialize txs
        let serialized = serde_json::to_vec(txs)?;

        // Compress using zstd
        let compressed = zstd::encode_all(&serialized[..], 3)?;

        Ok(compressed.len())
    }
}
