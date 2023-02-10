use crate::{
    crypto::{PublicKey, Signature},
    types::Hash,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{
    encoding::{Decoder, Encoder},
    hasher::Hasher,
    transaction::Transaction,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Header {
    version: u32,
    data_hash: Hash,
    prev_block_hash: Hash,
    timestamp: u64,
    height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    header: Header,
    txs: Vec<Transaction>,
    validator: Option<PublicKey>,
    signature: Option<Signature>,
    // Cached version of the header hash
    hash: Hash,
}

impl Block {
    pub fn new(h: Header, txs: Vec<Transaction>) -> Self {
        Self {
            header: h,
            txs,
            validator: None,
            signature: None,
            hash: Hash::default(),
        }
    }

    pub fn hash(&mut self, hasher: Box<dyn Hasher<Self>>) -> Hash {
        if self.hash.is_zero() {
            self.hash = hasher
                .hash(self)
                .map_err(|err| panic!("block.rs hashing failed: {err}"))
                .unwrap();
        }
        self.hash
    }

    pub fn encode(
        &mut self,
        w: Box<dyn std::io::Write>,
        enc: Box<dyn Encoder<Self>>,
    ) -> Result<()> {
        enc.encode(w, self)
    }

    pub fn decode(&mut self, r: Box<dyn std::io::Read>, dec: Box<dyn Decoder<Self>>) -> Result<()> {
        dec.decode(r, self)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::hasher::BlockHasher;

    use super::*;

    #[test]
    fn test_hash_block() {
        let mut block = Block::new(Header::default(), vec![]);
        let mut hasher = BlockHasher;
        let hash = block.hash(Box::new(hasher));
        assert_eq!(hash, Hash::default());
    }
}
