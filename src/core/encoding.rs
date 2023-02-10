use anyhow::Result;
use std::io;

pub trait Encoder<T> {
    fn encode(&self, w: Box<dyn io::Write>, t: &T) -> Result<()>;
}

pub trait Decoder<T> {
    fn decode(&self, r: Box<dyn io::Read>, t: &T) -> Result<()>;
}
