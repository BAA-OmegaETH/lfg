use crate::types::Batch;
use anyhow::Result;

pub struct BlobSender {
    rpc_url: String,
}

impl BlobSender {
    pub fn new(rpc_url: String) -> Self {
        Self { rpc_url }
    }

    pub async fn send_blob(&self, batch: &Batch) -> Result<String> {
        // TODO: Integrate alloy to send actual blob transaction
        tracing::info!(
            "Sending blob with {} txs, compressed_size={}",
            batch.txs.len(),
            batch.compressed_size
        );

        // Placeholder: return mock tx hash
        Ok(format!("0x{:064x}", batch.txs[0].tx_id))
    }
}
