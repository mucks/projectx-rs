use crate::types::Hash;
use anyhow::Result;
use sha2::{Digest, Sha256};

use super::transaction::Transaction;

#[derive(Debug, Default)]
pub struct Header {
    version: u32,
    prev_block: Hash,
    timestamp: u64,
    height: u32,
    nonce: u64,
}

impl Header {
    // Generic implementation of encode_binary for any type that implements Write
    pub fn encode_binary(&self, w: &mut dyn std::io::Write) -> Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.prev_block.into_bytes())?;
        w.write_all(&self.timestamp.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&self.nonce.to_le_bytes())?;
        Ok(())
    }

    // Generic implementation of decode_binary for any type that implements Read
    pub fn decode_binary(&mut self, r: &mut dyn std::io::Read) -> Result<()> {
        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;
        self.version = u32::from_le_bytes(buf);

        let mut buf: [u8; 32] = [0; 32];
        r.read_exact(&mut buf)?;
        self.prev_block = Hash::from_bytes(&buf);

        let mut buf: [u8; 8] = [0; 8];
        r.read_exact(&mut buf)?;
        self.timestamp = u64::from_le_bytes(buf);

        let mut buf: [u8; 4] = [0; 4];
        r.read_exact(&mut buf)?;
        self.height = u32::from_le_bytes(buf);

        let mut buf: [u8; 8] = [0; 8];
        r.read_exact(&mut buf)?;
        self.nonce = u64::from_le_bytes(buf);

        Ok(())
    }
}

pub struct Block {
    header: Header,
    txs: Vec<Transaction>,
    // Cached version of the header hash
    hash: Hash,
}

impl Block {
    pub fn hash(&mut self) -> Hash {
        let mut buf: Vec<u8> = Vec::new();
        self.header.encode_binary(&mut buf).unwrap();

        if self.hash.is_zero() {
            self.hash = Hash::from_bytes(Sha256::digest(&buf).as_slice());
        }

        self.hash
    }

    pub fn encode_binary(&self, w: &mut dyn std::io::Write) -> Result<()> {
        self.header.encode_binary(w)?;

        for tx in self.txs.iter() {
            tx.encode_binary(w)?;
        }

        Ok(())
    }

    pub fn decode_binary(&mut self, r: &mut dyn std::io::Read) -> Result<()> {
        self.header.decode_binary(r)?;

        for tx in self.txs.iter_mut() {
            tx.decode_binary(r)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    // test that we can encode and decode a header
    #[test]
    fn test_header_encode_decode() {
        let header = Header {
            version: 1,
            prev_block: Hash::random(),
            timestamp: std::time::Instant::now().elapsed().as_secs(),
            height: 10,
            nonce: 96060373,
        };

        let mut buf = Vec::new();
        header.encode_binary(&mut buf).unwrap();

        let mut header2 = Header::default();

        header2.decode_binary(&mut buf.as_slice()).unwrap();

        assert_eq!(header.version, header2.version);
        assert_eq!(header.prev_block, header2.prev_block);
        assert_eq!(header.timestamp, header2.timestamp);
        assert_eq!(header.height, header2.height);
        assert_eq!(header.nonce, header2.nonce);
    }

    // test that we can encode and decode a block
    #[test]
    fn test_block_encode_decode() {
        let block = Block {
            header: Header {
                version: 1,
                prev_block: Hash::random(),
                timestamp: std::time::Instant::now().elapsed().as_secs(),
                height: 10,
                nonce: 96060373,
            },
            txs: vec![],
            hash: Hash::default(),
        };

        let mut buf = Vec::new();
        block.encode_binary(&mut buf).unwrap();

        let mut block2 = Block {
            header: Header::default(),
            txs: vec![],
            hash: Hash::default(),
        };

        block2.decode_binary(&mut buf.as_slice()).unwrap();

        assert_eq!(block.header.version, block2.header.version);
        assert_eq!(block.header.prev_block, block2.header.prev_block);
        assert_eq!(block.header.timestamp, block2.header.timestamp);
        assert_eq!(block.header.height, block2.header.height);
        assert_eq!(block.header.nonce, block2.header.nonce);
    }

    #[test]
    fn test_block_hash() {
        let mut block = Block {
            header: Header {
                version: 1,
                prev_block: Hash::random(),
                timestamp: std::time::Instant::now().elapsed().as_secs(),
                height: 10,
                nonce: 96060373,
            },
            txs: vec![],
            hash: Hash::default(),
        };

        let hash = block.hash();

        assert_eq!(hash, block.hash());
    }
}
