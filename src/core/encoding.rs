use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
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

impl<'a, T> Encoder<T> for BincodeEncoder<'a>
where
    T: Serialize,
{
    fn encode(&mut self, t: &T) -> Result<()> {
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

impl<'a, T> Decoder<T> for BincodeDecoder<'a>
where
    T: DeserializeOwned,
{
    fn decode(&mut self, t: &mut T) -> Result<()> {
        // let w = Box::into_inner(self.w);
        *t = bincode::deserialize_from(&mut self.r)?;
        Ok(())
    }
}
