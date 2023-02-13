use crate::{
    core::{Transaction, TxHasher},
    types::Hash,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct TxPool {
    transactions: HashMap<Hash, Transaction>,
}

impl TxPool {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    // Add a transaction to the pool, the caller is responsible for checking if the transaction already exists
    pub fn add(&mut self, mut tx: Transaction) -> Result<()> {
        let hash = tx.hash(Box::new(TxHasher {}))?;

        if let Some(tx_mut) = self.transactions.get_mut(&hash) {
            *tx_mut = tx;
        } else {
            self.transactions.insert(hash, tx);
        }

        Ok(())
    }

    pub fn has(&self, hash: &Hash) -> bool {
        self.transactions.contains_key(hash)
    }

    pub fn flush(&mut self) {
        self.transactions = HashMap::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_pool() {
        let p = TxPool::new();
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn test_tx_pool_add_tx() -> Result<()> {
        let mut p = TxPool::new();
        let tx = Transaction::new(b"foo".to_vec());
        p.add(tx)?;
        assert_eq!(p.len(), 1);
        let tx = Transaction::new(b"foo".to_vec());
        p.add(tx)?;
        assert_eq!(p.len(), 1);

        p.flush();
        assert_eq!(p.len(), 0);

        Ok(())
    }
}
