use anyhow::{anyhow, Result};
use log::debug;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{PrivateKey, PublicKey, Signature},
    types::Hash,
};

use super::{
    encoding::{Decoder, Encoder},
    hasher::Hasher,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub data: Vec<u8>,

    pub from: Option<PublicKey>,
    pub signature: Option<Signature>,
    // cached version of tx hash
    #[serde(skip)]
    hash: Option<Hash>,
    // first_seen is the time when the transaction was first seen locally
    #[serde(skip)]
    first_seen: u128,
}

impl Transaction {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            from: None,
            signature: None,
            hash: None,
            first_seen: 0,
        }
    }

    pub fn set_first_seen(&mut self, first_seen: u128) {
        self.first_seen = first_seen;
    }

    pub fn first_seen(&self) -> u128 {
        self.first_seen
    }

    pub fn calculate_and_cache_hash(&mut self, hasher: Box<dyn Hasher<Transaction>>) -> Result<()> {
        if self.hash.is_none() {
            self.hash = Some(hasher.hash(self)?);
            debug!("calculated hash for transaction: {}", self.hash.unwrap());
        }
        Ok(())
    }

    pub fn has_cached_hash(&self) -> bool {
        self.hash.is_some()
    }

    pub fn hash(&self) -> Hash {
        self.hash.expect("transaction hash not calculated")
    }

    pub fn sign(&mut self, private_key: &PrivateKey) {
        let data = self.data.clone();
        self.from = Some(private_key.public_key());
        self.signature = Some(private_key.sign(&data));
    }

    pub fn verify(&self) -> Result<()> {
        let sig = self
            .signature
            .as_ref()
            .ok_or_else(|| anyhow!("transaction has no signature"))?;

        let pub_key = self
            .from
            .as_ref()
            .ok_or_else(|| anyhow!("from has no signature"))?;

        if !sig.verify(&self.data, pub_key) {
            return Err(anyhow!("transaction has invalid signature"));
        }

        Ok(())
    }

    pub fn encode(&self, enc: &mut dyn Encoder<Transaction>) -> Result<()> {
        enc.encode(self)
    }

    pub fn decode(&mut self, dec: &mut dyn Decoder<Transaction>) -> Result<()> {
        dec.decode(self)
    }

    pub fn random_with_signature() -> Transaction {
        let private_key = PrivateKey::generate();

        let mut tx = Transaction {
            data: thread_rng().gen::<[u8; 32]>().to_vec(),
            from: None,
            signature: None,
            hash: None,
            first_seen: std::time::Instant::now().elapsed().as_nanos(),
        };

        tx.sign(&private_key);
        tx
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::core::{BincodeDecoder, BincodeEncoder};

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_sign_transaction() {
        let mut tx = Transaction::new(vec![1, 2, 3]);
        let private_key = PrivateKey::generate();
        tx.sign(&private_key);

        assert!(tx.signature.is_some());
    }

    #[test]
    fn test_verify_transaction() -> Result<()> {
        let mut tx = Transaction::new(vec![1, 2, 3]);
        let private_key = PrivateKey::generate();
        tx.sign(&private_key);
        tx.verify()?;

        let other_private_key = PrivateKey::generate();
        tx.from = Some(other_private_key.public_key());
        assert!(tx.verify().is_err());

        Ok(())
    }

    #[test]
    fn test_encode_decode() -> Result<()> {
        let tx = Transaction::random_with_signature();

        let mut buf: Vec<u8> = vec![];
        let mut enc = BincodeEncoder::new(&mut buf);
        tx.encode(&mut enc)?;

        let mut f = Cursor::new(buf);
        let mut tx_decoded = Transaction::new(vec![]);
        let mut dec = BincodeDecoder::new(&mut f);
        tx_decoded.decode(&mut dec)?;
        assert_eq!(tx.data, tx_decoded.data);

        Ok(())
    }
}
