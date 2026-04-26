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
        let mut combined_data = Vec::new();
        for tx in txs {
            combined_data.extend_from_slice(&tx.data);
        }
        let compressed = zstd::encode_all(&combined_data[..], 3)?;
        Ok(compressed.len())
    }
}
