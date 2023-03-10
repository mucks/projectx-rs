use crate::core::block::Block;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

use super::{
    blockchain::Blockchain,
    hasher::{BlockHasher, Hasher},
};

#[async_trait]
pub trait Validator: Send + Sync {
    async fn validate_block(&self, bc: &Blockchain, block: &mut Block) -> Result<()>;
}

pub struct BlockValidator {}

impl BlockValidator {
    pub fn new() -> Self {
        BlockValidator {}
    }
}

#[async_trait]
impl Validator for BlockValidator {
    async fn validate_block(&self, bc: &Blockchain, b: &mut Block) -> Result<()> {
        if bc.has_block(b.header.height).await {
            return Err(anyhow!("block already known"));
            // return Err(anyhow!(
            //     "chain already contains block {} with hash {}",
            //     b.header.height.clone(),
            //     b.hash(Box::new(BlockHasher))
            // ));
        }

        let bc_height = bc.height().await;
        let block_height = b.header.height;

        if block_height != bc_height + 1 {
            return Err(anyhow!(
                "our_height: {}, Block with height {} and hash {} too high!",
                bc_height,
                block_height,
                b.hash(Box::new(BlockHasher))
            ));
        }

        let header = bc.get_header(b.header.height - 1).await?;
        let hash = BlockHasher {}.hash(&header)?;

        match b.header.prev_block_hash {
            Some(prev_block_hash) => {
                if hash != prev_block_hash {
                    return Err(anyhow!(
                        "the hash of the previous block {} is invalid!",
                        prev_block_hash
                    ));
                }
            }
            None => {
                return Err(anyhow!("the hash of the previous block is not set!",));
            }
        };

        b.verify()?;

        Ok(())
    }
}
