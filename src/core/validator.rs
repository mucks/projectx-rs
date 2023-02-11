use crate::core::block::Block;
use anyhow::{anyhow, Result};

use super::{
    blockchain::Blockchain,
    hasher::{BlockHasher, Hasher},
};

pub trait Validator {
    fn validate_block(&self, bc: &Blockchain, block: &mut Block) -> Result<()>;
}

pub struct BlockValidator {}

impl BlockValidator {
    pub fn new() -> Self {
        BlockValidator {}
    }
}

impl Validator for BlockValidator {
    fn validate_block(&self, bc: &Blockchain, b: &mut Block) -> Result<()> {
        if bc.has_block(b.header.height) {
            return Err(anyhow!(
                "chain already contains block {} with hash {}",
                b.header.height.clone(),
                b.hash(Box::new(BlockHasher))
            ));
        }

        if b.header.height != bc.height() + 1 {
            return Err(anyhow!("Block {} too high!", b.hash(Box::new(BlockHasher))));
        }

        let header = bc.get_header(b.header.height - 1)?;
        let hash = BlockHasher {}.hash(&header)?;

        if hash != b.header.prev_block_hash {
            return Err(anyhow!(
                "the hash of the previous block {} is invalid!",
                b.header.prev_block_hash
            ));
        }

        b.verify()?;

        Ok(())
    }
}
