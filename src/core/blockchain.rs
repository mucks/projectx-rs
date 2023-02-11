use crate::types::Hash;

use super::{
    block::{Block, Header},
    hasher::{BlockHasher, Hasher},
    storage::{MemoryStore, Storage},
    validator::{BlockValidator, Validator},
};
use anyhow::{anyhow, Result};
use log::info;

// maybe use a lifetime to only store a reference to the header?
// headers: Vec<&'a Header>,
pub struct Blockchain {
    store: Box<dyn Storage>,
    headers: Vec<Header>,
    validator: Option<Box<dyn Validator>>,
}

impl Blockchain {
    pub fn new(mut genesis: Block) -> Result<Self> {
        let mut bc = Blockchain {
            store: Box::new(MemoryStore::new()),
            validator: Some(Box::new(BlockValidator::new())),
            headers: vec![],
        };

        bc.add_block_without_validation(&mut genesis)?;
        Ok(bc)
    }

    pub fn set_validator(&mut self, v: Box<dyn Validator>) {
        self.validator = Some(v);
    }

    pub fn has_block(&self, height: u32) -> bool {
        height <= self.height()
    }

    pub fn add_block(&mut self, b: &mut Block) -> Result<()> {
        self.validator
            .as_ref()
            .ok_or_else(|| anyhow!("blockchain has no validator"))?
            .validate_block(&self, b)?;
        self.add_block_without_validation(b)?;
        Ok(())
    }

    fn add_block_without_validation(&mut self, b: &mut Block) -> Result<()> {
        info!(
            "Adding block {} with height {} to blockchain",
            b.hash(Box::new(BlockHasher)),
            b.header.height
        );
        self.headers.push(b.header);
        Ok(())
    }

    pub fn get_header(&self, height: u32) -> Result<Header> {
        if height > self.height() {
            return Err(anyhow!("given height {height} too high"));
        }
        Ok(*self
            .headers
            .get(height as usize)
            .ok_or_else(|| anyhow!("Block Header with height {height} not found"))?)
    }

    pub fn get_prev_block_hash(&self, height: u32) -> Result<Hash> {
        let header = self.get_header(height - 1)?;
        BlockHasher {}.hash(&header)
    }

    // [0, 1, 2, 3] => 4 len
    // [0, 1, 2, 3] => 3 height
    pub fn height(&self) -> u32 {
        (self.headers.len() - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_blockchain() -> Result<()> {
        let bc = Blockchain::new(Block::random(0, Hash::default()))?;
        assert_eq!(bc.height(), 0);

        Ok(())
    }

    #[test]
    fn test_has_block() -> Result<()> {
        let bc = Blockchain::new(Block::random(0, Hash::default()))?;
        assert!(bc.has_block(0));
        Ok(())
    }

    // this is quite slow, optimize this
    #[test]
    fn test_add_block() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random()))?;

        let len_blocks = 100;
        for i in 0..len_blocks {
            let prev_block_hash = bc.get_prev_block_hash(i + 1)?;
            let mut b = Block::random_with_signature(i + 1, prev_block_hash)?;
            bc.add_block(&mut b)?;
        }

        assert_eq!(bc.height(), len_blocks);
        assert_eq!(bc.headers.len() as u32, len_blocks + 1);

        assert!(bc
            .add_block(&mut Block::random(89, Hash::random()))
            .is_err());

        Ok(())
    }

    #[test]
    fn test_get_header() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random()))?;

        let len_blocks = 100;

        for i in 0..len_blocks {
            let prev_block_hash = bc.get_prev_block_hash(i + 1)?;
            let mut b = Block::random_with_signature(i + 1, prev_block_hash)?;
            bc.add_block(&mut b)?;
            let header = bc.get_header(i + 1)?;
            assert_eq!(header, b.header);
        }
        Ok(())
    }

    #[test]
    fn test_add_block_too_high() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random()))?;
        bc.add_block(&mut Block::random_with_signature(
            1,
            bc.get_prev_block_hash(1)?,
        )?)?;
        assert!(bc
            .add_block(&mut Block::random_with_signature(3, Hash::random())?)
            .is_err());

        Ok(())
    }
}
