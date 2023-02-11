use crate::{
    crypto::{PrivateKey, PublicKey, Signature},
    types::Hash,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::{
    encoding::{Decoder, Encoder},
    hasher::Hasher,
    transaction::Transaction,
};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Header {
    version: u32,
    data_hash: Hash,
    prev_block_hash: Hash,
    timestamp: u64,
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub header: Header,
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

    pub fn sign(&mut self, private_key: &PrivateKey) -> Result<()> {
        let sig = private_key.sign(&self.header_bytes()?);

        self.validator = Some(private_key.public_key());
        self.signature = Some(sig);

        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        let sig = self
            .signature
            .as_ref()
            .ok_or_else(|| anyhow!("block has no signature"))?;
        let pub_key = self
            .validator
            .as_ref()
            .ok_or_else(|| anyhow!("block has no validator (public_key)"))?;

        if !sig.verify(&self.header_bytes()?, pub_key) {
            return Err(anyhow!("block has invalid signature"));
        }
        Ok(())
    }

    pub fn header_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self.header)?)
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

    pub fn random(height: u32) -> Block {
        let mut txs = vec![];
        let header = Header {
            version: 1,
            data_hash: Hash::random(),
            prev_block_hash: Hash::random(),
            timestamp: std::time::Instant::now().elapsed().as_secs(),
            height,
        };
        let tx = Transaction {
            data: vec![0; 32],
            public_key: None,
            signature: None,
        };
        txs.push(tx);

        Block::new(header, txs)
    }

    pub fn random_with_signature(height: u32) -> Result<Block> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(height);
        b.sign(&private_key)?;
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hasher::BlockHasher;
    use anyhow::Result;

    #[test]
    fn test_hash_block() {
        let mut block = Block::random(0);
        let hash = block.hash(Box::new(BlockHasher));
        println!("hash: {hash}");
    }

    #[test]
    fn test_sign_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0);
        b.sign(&private_key)?;
        assert!(b.signature.is_some());

        Ok(())
    }
    #[test]
    fn test_verify_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0);
        b.sign(&private_key)?;
        b.verify()?;

        // changing the data should make the public key invalid
        b.header.height = 100;
        assert!(b.verify().is_err());
        b.header.height = 0;

        // changing the public key should make the signature invalid
        let other_private_key = PrivateKey::generate();
        b.validator = Some(other_private_key.public_key());
        assert!(b.verify().is_err());

        Ok(())
    }
}
