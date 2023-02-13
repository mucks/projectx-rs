use anyhow::Result;
use std::io;

use super::Transaction;

pub trait Encoder<T> {
    fn encode(&mut self, t: &T) -> Result<()>;
}

pub trait Decoder<T> {
    fn decode(&mut self, t: &mut T) -> Result<()>;
}

pub struct BincodeEncoder<'a> {
    w: &'a mut dyn std::io::Write,
}

impl<'a> BincodeEncoder<'a> {
    pub fn new(w: &'a mut dyn std::io::Write) -> Self {
        Self { w }
    }
}

impl<'a> Encoder<Transaction> for BincodeEncoder<'a> {
    fn encode(&mut self, t: &Transaction) -> Result<()> {
        // let w = Box::into_inner(self.w);
        bincode::serialize_into(&mut self.w, t)?;
        Ok(())
    }
}

pub struct BincodeDecoder<'a> {
    r: &'a mut dyn std::io::Read,
}

impl<'a> BincodeDecoder<'a> {
    pub fn new(r: &'a mut dyn std::io::Read) -> Self {
        Self { r }
    }
}

impl<'a> Decoder<Transaction> for BincodeDecoder<'a> {
    fn decode(&mut self, t: &mut Transaction) -> Result<()> {
        // let w = Box::into_inner(self.w);
        *t = bincode::deserialize_from(&mut self.r)?;
        Ok(())
    }
}
