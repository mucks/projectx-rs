use anyhow::Result;

pub trait Storage {
    fn put(&self) -> Result<()>;
    fn get(&self) -> Result<()>;
}

pub struct MemoryStore;

impl MemoryStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl Storage for MemoryStore {
    fn put(&self) -> Result<()> {
        Ok(())
    }

    fn get(&self) -> Result<()> {
        Ok(())
    }
}
