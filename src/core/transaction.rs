use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{PrivateKey, PublicKey, Signature},
    types::Hash,
};

use super::hasher::Hasher;

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub data: Vec<u8>,

    pub from: Option<PublicKey>,
    pub signature: Option<Signature>,
    // cached version of tx hash
    pub hash: Option<Hash>,
}

impl Transaction {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            from: None,
            signature: None,
            hash: None,
        }
    }

    pub fn hash(&mut self, hasher: Box<dyn Hasher<Transaction>>) -> Result<Hash> {
        match self.hash {
            Some(h) => Ok(h),
            None => {
                self.hash = Some(hasher.hash(&self)?);
                Ok(self.hash.unwrap())
            }
        }
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

    pub fn random_with_signature() -> Transaction {
        let private_key = PrivateKey::generate();

        let mut tx = Transaction {
            data: b"foo".to_vec(),
            from: None,
            signature: None,
            hash: None,
        };

        tx.sign(&private_key);
        tx
    }
}

#[cfg(test)]
mod tests {
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
}
