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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    version: u32,
    data_hash: Hash,
    pub prev_block_hash: Hash,
    timestamp: u64,
    pub height: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub header: Header,
    transactions: Vec<Transaction>,
    validator: Option<PublicKey>,
    signature: Option<Signature>,
    // Cached version of the header hash
    hash: Hash,
}

impl Header {
    pub fn bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }
}

impl Block {
    pub fn new(h: Header, txs: Vec<Transaction>) -> Self {
        Self {
            header: h,
            transactions: txs,
            validator: None,
            signature: None,
            hash: Hash::default(),
        }
    }

    pub fn hash(&mut self, hasher: Box<dyn Hasher<Header>>) -> Hash {
        if self.hash.is_zero() {
            self.hash = hasher
                .hash(&self.header)
                .map_err(|err| panic!("block.rs hashing failed: {err}"))
                .unwrap();
        }
        self.hash
    }

    pub fn sign(&mut self, private_key: &PrivateKey) -> Result<()> {
        let sig = private_key.sign(&self.header.bytes()?);

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

        if !sig.verify(&self.header.bytes()?, pub_key) {
            return Err(anyhow!("block has invalid signature"));
        }

        for tx in &self.transactions {
            tx.verify()?;
        }

        Ok(())
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
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

    pub fn random(height: u32, prev_block_hash: Hash) -> Block {
        let header = Header {
            version: 1,
            data_hash: Hash::random(),
            prev_block_hash,
            timestamp: std::time::Instant::now().elapsed().as_secs(),
            height,
        };

        Block::new(header, vec![])
    }

    pub fn random_with_signature(height: u32, prev_block_hash: Hash) -> Result<Block> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(height, prev_block_hash);
        let tx = Transaction::random_with_signature();
        b.add_transaction(tx);
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
        let mut block = Block::random(0, Hash::default());
        let hash = block.hash(Box::new(BlockHasher));
        println!("hash: {hash}");
    }

    #[test]
    fn test_sign_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0, Hash::default());
        b.sign(&private_key)?;
        assert!(b.signature.is_some());

        Ok(())
    }
    #[test]
    fn test_verify_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0, Hash::default());
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
