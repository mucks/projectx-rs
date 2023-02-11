use super::{
    block::{Block, Header},
    storage::{MemoryStore, Storage},
    validator::{BlockValidator, Validator},
};
use anyhow::{anyhow, Result};

// maybe use a lifetime to only store a reference to the header?
// headers: Vec<&'a Header>,
pub struct Blockchain {
    store: Box<dyn Storage>,
    headers: Vec<Header>,
    validator: Option<Box<dyn Validator>>,
}

impl Blockchain {
    pub fn new(genesis: Block) -> Result<Self> {
        let mut bc = Blockchain {
            store: Box::new(MemoryStore::new()),
            validator: Some(Box::new(BlockValidator::new())),
            headers: vec![],
        };

        bc.add_block_without_validation(&genesis)?;
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

    fn add_block_without_validation(&mut self, b: &Block) -> Result<()> {
        self.headers.push(b.header);
        Ok(())
    }

    // [0, 1, 2, 3] => 4 len
    // [0, 1, 2, 3] => 3 height
    fn height(&self) -> u32 {
        (self.headers.len() - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_blockchain() -> Result<()> {
        let bc = Blockchain::new(Block::random(0))?;
        assert_eq!(bc.height(), 0);

        Ok(())
    }

    #[test]
    fn test_has_block() -> Result<()> {
        let bc = Blockchain::new(Block::random(0))?;
        assert!(bc.has_block(0));
        Ok(())
    }

    // this is quite slow, optimize this
    #[test]
    fn test_add_block() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0))?;

        let len_blocks = 1000;
        for i in 0..len_blocks {
            let mut b = Block::random_with_signature(i + 1)?;
            bc.add_block(&mut b)?;
        }

        assert_eq!(bc.height(), len_blocks);
        assert_eq!(bc.headers.len() as u32, len_blocks + 1);

        assert!(bc.add_block(&mut Block::random(89)).is_err());

        Ok(())
    }
}
