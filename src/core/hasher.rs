use super::block::Block;
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

impl Hasher<Block> for BlockHasher {
    fn hash(&self, block: &Block) -> Result<Hash> {
        let bytes = block.header_bytes()?;
        let hash = Hash::from_bytes(Sha256::digest(&bytes).as_slice());
        Ok(hash)
    }
}
