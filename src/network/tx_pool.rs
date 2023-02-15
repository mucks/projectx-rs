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
    all: HashMap<Hash, Transaction>,
    pending: HashMap<Hash, Transaction>,
    max_length: usize,
}

impl TxPool {
    pub fn new(max_length: usize) -> Self {
        Self {
            all: HashMap::new(),
            pending: HashMap::new(),
            max_length,
        }
    }
    pub fn len(&self) -> usize {
        self.all.len()
    }
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear()
    }

    // Add a transaction to the pool, the caller is responsible for checking if the transaction already exists
    pub fn add(&mut self, mut tx: Transaction) -> Result<()> {
        if tx.hash.is_none() {
            tx.calculate_and_cache_hash(Box::new(TxHasher))?;
        }

        if self.all.len() == self.max_length {
            let oldest_hash = self
                .all()
                .first()
                .ok_or_else(|| anyhow!("could not find first block in all transactions"))?
                .hash();
            self.all.remove(&oldest_hash);
        }

        let tx_hash = tx.hash();

        if !self.has(&tx_hash) {
            self.all.insert(tx_hash, tx.clone());
            self.pending.insert(tx_hash, tx);
        }

        Ok(())
    }

    pub fn has(&self, hash: &Hash) -> bool {
        self.all.contains_key(hash)
    }

    pub fn flush(&mut self) {
        self.all = HashMap::new();
    }
    pub fn pending(&self) -> Vec<&Transaction> {
        let s = TxMapSorter::new(&self.pending);
        s.transactions
    }

    pub fn all(&self) -> Vec<&Transaction> {
        let s = TxMapSorter::new(&self.all);
        s.transactions
    }

    pub fn pending_cloned(&self) -> Vec<Transaction> {
        let s = TxMapSorter::new(&self.pending);
        s.transactions.into_iter().cloned().collect()
    }

    //TODO: fix cause very inefficient
    pub fn all_cloned(&self) -> Vec<Transaction> {
        let s = TxMapSorter::new(&self.all);
        s.transactions.into_iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn test_tx_pool() {
        let p = TxPool::new(10);
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn test_tx_pool_add_tx() -> Result<()> {
        let mut p = TxPool::new(11);
        let n = 10;

        for i in 1..n {
            let tx = Transaction::random_with_signature();

            p.add(tx.clone())?;
            p.add(tx)?;

            assert_eq!(i, p.pending_count());
            assert_eq!(i, p.pending.len());
            assert_eq!(i, p.all.len());
        }

        Ok(())
    }

    #[test]
    fn test_sort_transaction() -> Result<()> {
        let tx_len: usize = 1000;
        let mut p = TxPool::new(tx_len);

        for i in 0..tx_len {
            let mut tx = Transaction::new(i.to_le_bytes().to_vec());
            tx.set_first_seen((i * thread_rng().gen_range(1..1000)) as u128);
            p.add(tx)?;
        }
        assert_eq!(tx_len, p.len());

        let transactions = p.all();
        for i in 0..tx_len - 1 {
            assert!(transactions[i].first_seen() <= transactions[i + 1].first_seen());
        }

        Ok(())
    }
}
