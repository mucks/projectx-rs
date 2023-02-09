use std::fmt::Display;

use rand::RngCore;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash([u8; 32]);

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Hash {
    pub fn into_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() != 32 {
            panic!(
                "given bytes with length {} is not a valid hash, it must be 32 bytes long",
                bytes.len()
            );
        }
        let mut hash = [0; 32];
        hash.copy_from_slice(bytes);
        Self(hash)
    }

    pub fn is_zero(&self) -> bool {
        for byte in &self.0 {
            if *byte != 0 {
                return false;
            }
        }
        true
    }

    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut hash = [0; 32];
        rng.fill_bytes(&mut hash);
        Self(hash)
    }
}
