use std::fmt::Display;

use p256::ecdsa::{
    signature::{Signer, Verifier},
    SigningKey, VerifyingKey,
};
use sha2::Digest;

use crate::types::Address;

pub struct PrivateKey {
    key: p256::SecretKey,
}

impl PrivateKey {
    pub fn generate() -> Self {
        let key = p256::SecretKey::random(&mut rand::thread_rng());
        Self { key }
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            key: self.key.public_key(),
        }
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        let signing_key = SigningKey::from(&self.key);
        let signature: p256::ecdsa::Signature = signing_key.sign(data);
        Signature { sig: signature }
    }
}

pub struct PublicKey {
    key: p256::PublicKey,
}

impl PublicKey {
    pub fn address(&self) -> Address {
        let mut sha = sha2::Sha256::new();
        sha.update(self.key.to_string().as_bytes());
        let b = sha.finalize();
        Address::from_bytes(b[b.len() - 20..].as_ref())
    }
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey::from(&self.key)
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.sig.to_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
pub struct Signature {
    sig: p256::ecdsa::Signature,
}

impl Signature {
    pub fn verify(&self, data: &[u8], public_key: &PublicKey) -> bool {
        public_key.verifying_key().verify(data, &self.sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_sign_verify_valid() {
        let private_key = PrivateKey::generate();
        let public_key = private_key.public_key();

        let msg = b"hello world";
        let sig = private_key.sign(msg);

        assert!(sig.verify(msg, &public_key));
    }

    #[test]
    fn test_keypair_sign_verify_fail() {
        let private_key = PrivateKey::generate();
        let _public_key = private_key.public_key();

        let msg = b"hello world";
        let sig = private_key.sign(msg);

        let other_private_key = PrivateKey::generate();
        let other_public_key = other_private_key.public_key();

        assert!(!sig.verify(msg, &other_public_key));
        assert!(!sig.verify(b"wrong message", &other_public_key));
    }
}
