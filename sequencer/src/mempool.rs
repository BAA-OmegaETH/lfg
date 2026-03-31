use crate::types::UserTx;
use std::collections::VecDeque;

pub struct Mempool {
    txs: VecDeque<UserTx>,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            txs: VecDeque::new(),
        }
    }

    pub fn add_tx(&mut self, tx: UserTx) {
        self.txs.push_back(tx);
    }

    pub fn get_all(&self) -> Vec<UserTx> {
        self.txs.iter().cloned().collect()
    }

    pub fn remove_txs(&mut self, tx_ids: &[u64]) {
        self.txs.retain(|tx| !tx_ids.contains(&tx.tx_id));
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}
