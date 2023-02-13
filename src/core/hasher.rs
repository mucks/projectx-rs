use std::io::Read;

use super::{block::Header, Transaction};
use crate::types::Hash;
use anyhow::Result;
use sha2::{Digest, Sha256};

pub trait Hasher<T>
where
    T: Sized,
{
    fn hash(&self, t: &T) -> Result<Hash>;
}

pub struct BlockHasher;

impl Hasher<Header> for BlockHasher {
    fn hash(&self, header: &Header) -> Result<Hash> {
        let bytes = header.bytes()?;
        let hash = Hash::from_bytes(Sha256::digest(bytes).as_slice());
        Ok(hash)
    }
}

pub struct TxHasher;

impl Hasher<Transaction> for TxHasher {
    fn hash(&self, tx: &Transaction) -> Result<Hash> {
        let bytes = tx.data.clone();
        let hash = Hash::from_bytes(Sha256::digest(bytes).as_slice());
        Ok(hash)
    }
}
