use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::crypto::{PrivateKey, PublicKey, Signature};

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub data: Vec<u8>,

    pub public_key: Option<PublicKey>,
    pub signature: Option<Signature>,
}

impl Transaction {
    pub fn sign(&mut self, private_key: &PrivateKey) {
        let data = self.data.clone();
        self.public_key = Some(private_key.public_key());
        self.signature = Some(private_key.sign(&data));
    }

    pub fn verify(&self) -> Result<()> {
        let sig = self
            .signature
            .as_ref()
            .ok_or_else(|| anyhow!("transaction has no signature"))?;

        let pub_key = self
            .public_key
            .as_ref()
            .ok_or_else(|| anyhow!("public_key has no signature"))?;

        if !sig.verify(&self.data, pub_key) {
            return Err(anyhow!("transaction has invalid signature"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_sign_transaction() {
        let mut tx = Transaction {
            data: vec![1, 2, 3],
            public_key: None,
            signature: None,
        };
        let private_key = PrivateKey::generate();
        tx.sign(&private_key);

        assert!(tx.signature.is_some());
    }

    #[test]
    fn test_verify_transaction() -> Result<()> {
        let mut tx = Transaction {
            data: vec![1, 2, 3],
            public_key: None,
            signature: None,
        };
        let private_key = PrivateKey::generate();
        tx.sign(&private_key);
        tx.verify()?;

        let other_private_key = PrivateKey::generate();
        tx.public_key = Some(other_private_key.public_key());
        assert!(tx.verify().is_err());

        Ok(())
    }
}
