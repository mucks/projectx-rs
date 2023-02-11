use crate::core::block::Block;
use anyhow::{anyhow, Result};

use super::{blockchain::Blockchain, hasher::BlockHasher};

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

        b.verify()?;

        Ok(())
    }
}
