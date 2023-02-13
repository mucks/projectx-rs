use crate::types::Hash;

use super::{
    block::{Block, Header},
    hasher::{BlockHasher, Hasher},
    storage::{MemoryStore, Storage},
    validator::{BlockValidator, Validator},
};
use anyhow::{anyhow, Result};
use log::info;
use tokio::sync::RwLock;

// maybe use a lifetime to only store a reference to the header?
// headers: Vec<&'a Header>,
pub struct Blockchain {
    store: Box<dyn Storage>,
    headers: RwLock<Vec<Header>>,
    validator: Option<Box<dyn Validator>>,
}

impl Blockchain {
    pub async fn new(mut genesis: Block) -> Result<Self> {
        let mut bc = Blockchain {
            store: Box::new(MemoryStore::new()),
            validator: Some(Box::new(BlockValidator::new())),
            headers: RwLock::new(vec![]),
        };

        bc.add_block_without_validation(&mut genesis).await?;
        Ok(bc)
    }

    pub fn set_validator(&mut self, v: Box<dyn Validator>) {
        self.validator = Some(v);
    }

    pub async fn has_block(&self, height: u32) -> bool {
        height <= self.height().await
    }

    pub async fn add_block(&mut self, b: &mut Block) -> Result<()> {
        self.validator
            .as_ref()
            .ok_or_else(|| anyhow!("blockchain has no validator"))?
            .validate_block(self, b)
            .await?;
        self.add_block_without_validation(b).await?;
        Ok(())
    }

    async fn add_block_without_validation(&mut self, b: &mut Block) -> Result<()> {
        info!(
            "Adding block {} with height {} to blockchain",
            b.hash(Box::new(BlockHasher)),
            b.header.height
        );

        self.headers.write().await.push(b.header);
        Ok(())
    }

    pub async fn get_header(&self, height: u32) -> Result<Header> {
        if height > self.height().await {
            return Err(anyhow!("given height {height} too high"));
        }
        Ok(*self
            .headers
            .read()
            .await
            .get(height as usize)
            .ok_or_else(|| anyhow!("Block Header with height {height} not found"))?)
    }

    pub async fn get_prev_block_hash(&self, height: u32) -> Result<Hash> {
        let header = self.get_header(height - 1).await?;
        BlockHasher {}.hash(&header)
    }

    pub async fn len(&self) -> usize {
        self.headers.read().await.len()
    }

    pub async fn height(&self) -> u32 {
        self.headers.read().await.len() as u32 - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_blockchain() -> Result<()> {
        let bc = Blockchain::new(Block::random(0, Hash::default())).await?;
        assert_eq!(bc.height().await, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_has_block() -> Result<()> {
        let bc = Blockchain::new(Block::random(0, Hash::default())).await?;
        assert!(bc.has_block(0).await);
        Ok(())
    }

    // this is quite slow, optimize this
    #[tokio::test]
    async fn test_add_block() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random())).await?;

        let len_blocks = 10;
        for i in 0..len_blocks {
            let prev_block_hash = bc.get_prev_block_hash(i + 1).await?;
            let mut b = Block::random_with_signature(i + 1, prev_block_hash)?;
            bc.add_block(&mut b).await?;
        }

        assert_eq!(bc.height().await, len_blocks);
        assert_eq!(bc.len().await as u32, len_blocks + 1);

        assert!(bc
            .add_block(&mut Block::random(89, Hash::random()))
            .await
            .is_err());

        Ok(())
    }

    // this is quite slow
    #[tokio::test]
    async fn test_get_header() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random())).await?;

        let len_blocks = 10;

        for i in 0..len_blocks {
            let prev_block_hash = bc.get_prev_block_hash(i + 1).await?;
            let mut b = Block::random_with_signature(i + 1, prev_block_hash)?;
            bc.add_block(&mut b).await?;
            let header = bc.get_header(i + 1).await?;
            assert_eq!(header, b.header);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_add_block_too_high() -> Result<()> {
        let mut bc = Blockchain::new(Block::random(0, Hash::random())).await?;
        bc.add_block(&mut Block::random_with_signature(
            1,
            bc.get_prev_block_hash(1).await?,
        )?)
        .await?;
        assert!(bc
            .add_block(&mut Block::random_with_signature(3, Hash::random())?)
            .await
            .is_err());

        Ok(())
    }
}
