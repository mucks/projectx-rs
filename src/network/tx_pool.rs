use crate::{
    core::{Transaction, TxHasher},
    types::Hash,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub struct TxMapSorter<'a> {
    transactions: Vec<&'a Transaction>,
}

impl<'a> TxMapSorter<'a> {
    pub fn new(map: &'a HashMap<Hash, Transaction>) -> TxMapSorter<'a> {
        let mut transactions = Vec::new();
        for tx in map.values() {
            transactions.push(tx);
        }
        let mut s = TxMapSorter { transactions };
        s.sort();
        s
    }

    pub fn sort(&mut self) {
        self.transactions.sort_by_key(|a| a.first_seen());
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }
    pub fn less(&self, i: usize, j: usize) -> bool {
        self.transactions[i].first_seen() < self.transactions[j].first_seen()
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        self.transactions.swap(i, j);
    }
}

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
    pub fn transactions(&self) -> Vec<&Transaction> {
        let s = TxMapSorter::new(&self.transactions);
        s.transactions
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

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

    #[test]
    fn test_sort_transaction() -> Result<()> {
        let mut p = TxPool::new();
        let tx_len: usize = 1000;

        for i in 0..tx_len {
            let mut tx = Transaction::new(i.to_le_bytes().to_vec());
            tx.set_first_seen((i * thread_rng().gen_range(1..1000)) as u64);
            p.add(tx)?;
        }
        assert_eq!(tx_len, p.len());

        let transactions = p.transactions();
        for i in 0..tx_len - 1 {
            assert!(transactions[i].first_seen() <= transactions[i + 1].first_seen());
        }

        Ok(())
    }
}
