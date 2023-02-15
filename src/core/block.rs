use std::time::Instant;

use crate::{
    crypto::{PrivateKey, PublicKey, Signature},
    types::Hash,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    encoding::{Decoder, Encoder},
    hasher::Hasher,
    transaction::Transaction,
    BincodeEncoder, BlockHasher,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    version: u32,
    data_hash: Hash,
    pub prev_block_hash: Hash,
    timestamp: u128,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
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
    pub fn new(h: Header, txx: Vec<Transaction>) -> Self {
        Self {
            header: h,
            transactions: txx,
            validator: None,
            signature: None,
            hash: Hash::default(),
        }
    }

    pub fn from_prev_header(ph: Header, txx: Vec<Transaction>) -> Result<Self> {
        let data_hash = calculate_data_hash(&txx)?;

        let header = Header {
            version: ph.version,
            data_hash,
            prev_block_hash: BlockHasher {}.hash(&ph)?,
            timestamp: Instant::now().elapsed().as_nanos(),
            height: ph.height + 1,
        };

        Ok(Self::new(header, txx))
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

    pub fn verify(&mut self) -> Result<()> {
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

        let data_hash = calculate_data_hash(&self.transactions)?;

        if data_hash != self.header.data_hash {
            return Err(anyhow!("block has invalid data hash {}", data_hash));
        }

        Ok(())
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn encode(&mut self, mut enc: Box<dyn Encoder<Self>>) -> Result<()> {
        enc.encode(self)
    }

    pub fn decode(&mut self, mut dec: Box<dyn Decoder<Self>>) -> Result<()> {
        dec.decode(self)
    }

    pub fn genesis() -> Block {
        let header = Header {
            version: 1,
            data_hash: Hash::random(),
            prev_block_hash: Hash::default(),
            timestamp: std::time::Instant::now().elapsed().as_nanos(),
            height: 0,
        };

        Block::new(header, vec![])
    }

    pub fn random(height: u32, prev_block_hash: Hash) -> Result<Block> {
        let private_key = PrivateKey::generate();
        let tx = Transaction::random_with_signature();

        let header = Header {
            version: 1,
            data_hash: Hash::random(),
            prev_block_hash,
            timestamp: std::time::Instant::now().elapsed().as_nanos(),
            height,
        };

        let mut b = Block::new(header, vec![tx]);
        let data_hash = calculate_data_hash(&b.transactions)?;
        b.header.data_hash = data_hash;
        b.sign(&private_key)?;

        Ok(b)
    }
}

pub fn calculate_data_hash(txx: &Vec<Transaction>) -> Result<Hash> {
    let mut buf: Vec<u8> = vec![];

    for tx in txx {
        tx.encode(&mut BincodeEncoder::new(&mut buf))?;
    }

    let hash = Sha256::digest(buf.as_slice());

    Ok(Hash::from_bytes(&hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hasher::BlockHasher;
    use anyhow::Result;

    #[test]
    fn test_hash_block() -> Result<()> {
        let mut block = Block::random(0, Hash::default())?;
        let hash = block.hash(Box::new(BlockHasher));
        println!("hash: {hash}");
        Ok(())
    }

    #[test]
    fn test_sign_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0, Hash::default())?;
        b.sign(&private_key)?;
        assert!(b.signature.is_some());

        Ok(())
    }
    #[test]
    fn test_verify_block() -> Result<()> {
        let private_key = PrivateKey::generate();
        let mut b = Block::random(0, Hash::default())?;
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
