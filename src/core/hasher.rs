use std::io::Read;

use super::block::Header;
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
        let hash = Hash::from_bytes(Sha256::digest(&bytes).as_slice());
        Ok(hash)
    }
}
