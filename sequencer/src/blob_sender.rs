use std::time::{SystemTime, UNIX_EPOCH};

use alloy::{
    eips::eip4844::builder::{SidecarBuilder, SimpleCoder},
    network::{EthereumWallet, TransactionBuilder, TransactionBuilder4844},
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};

use crate::types::Batch;

pub struct BlobSender {
    rpc_url: String,
    private_key: String,
    enabled: bool,
}

impl BlobSender {
    pub fn new(rpc_url: String, private_key: String) -> Self {
        let enabled = !private_key.is_empty() && !rpc_url.is_empty();
        if !enabled {
            tracing::warn!("BlobSender disabled: set SENDER_PRIVATE_KEY and ETH_RPC_URL to enable real blob submission");
        }
        Self { rpc_url, private_key, enabled }
    }

    /// Send batch as a real EIP-4844 blob transaction.
    /// Returns (tx_hash, inclusion_latency_ms).
    pub async fn send_blob(&self, batch: &Batch) -> Result<(String, u64)> {
        if !self.enabled {
            let fake_hash = format!("0x{:064x}", batch.txs[0].tx_id);
            return Ok((fake_hash, 0));
        }

        // Concatenate all tx data into a single byte stream for the blob
        let mut data: Vec<u8> = Vec::new();
        for tx in &batch.txs {
            data.extend_from_slice(&tx.data);
        }

        // Connect to devnet
        let signer: PrivateKeySigner = self.private_key.parse()
            .context("Invalid SENDER_PRIVATE_KEY")?;
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.rpc_url.parse().context("Invalid ETH_RPC_URL")?);

        // Encode raw bytes into EIP-4844 blob field elements via SimpleCoder.
        // SimpleCoder masks each 32-byte chunk to stay within the BLS12-381
        // field prime, which is the only constraint on blob byte content.
        let sidecar: SidecarBuilder<SimpleCoder> = SidecarBuilder::from_slice(&data);
        let sidecar = sidecar.build().context("Failed to build blob sidecar")?;

        let tx = TransactionRequest::default()
            .with_to(Address::ZERO)
            .with_blob_sidecar(sidecar);

        let sent_at_ms = now_ms();

        let pending = provider.send_transaction(tx).await
            .context("Failed to send blob transaction")?;
        let tx_hash = format!("{:?}", pending.tx_hash());

        tracing::info!("Blob tx submitted: {}", tx_hash);

        let receipt = pending.get_receipt().await
            .context("Failed to get blob tx receipt")?;

        let inclusion_latency_ms = now_ms() - sent_at_ms;

        tracing::info!(
            "Blob included: hash={} block={} latency={}ms",
            tx_hash,
            receipt.block_number.unwrap_or(0),
            inclusion_latency_ms,
        );

        Ok((tx_hash, inclusion_latency_ms))
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
