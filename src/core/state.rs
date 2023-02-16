use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;

#[derive(Debug)]
pub struct State {
    data: HashMap<Vec<u8>, Vec<u8>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    pub fn put(&mut self, k: Vec<u8>, v: Vec<u8>) {
        self.data.insert(k, v);
    }

    pub fn delete(&mut self, k: &Vec<u8>) {
        self.data.remove(k);
    }

    pub fn get(&self, k: &Vec<u8>) -> Result<Vec<u8>> {
        self.data
            .get(k)
            .ok_or_else(|| anyhow!("given key {k:?} not found"))
            .cloned()
    }
}
