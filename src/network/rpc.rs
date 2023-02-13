// currently not using these traits because i couldn't get it to work with mutable references

use crate::core::{BincodeDecoder, BincodeEncoder, Decoder, Encoder, Transaction};

use super::transport::NetAddr;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum MessageType {
    Tx,
    Block,
    Other(u64),
}

pub struct RPC<'a> {
    pub from: NetAddr,
    pub payload: &'a mut dyn std::io::Read,
}

#[derive(Deserialize, Serialize)]
pub struct Message {
    pub header: MessageType,
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(header: MessageType, data: Vec<u8>) -> Self {
        Self { header, data }
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        BincodeEncoder::new(&mut buf).encode(&self)?;
        Ok(buf)
    }
}

pub trait RPCHandler {
    fn handle_rpc(&mut self, rpc: &mut RPC) -> Result<()>;
}

pub struct DefaultRPCHandler<'a> {
    p: &'a mut dyn RPCProcessor,
}

impl<'a> DefaultRPCHandler<'a> {
    pub fn new(p: &'a mut dyn RPCProcessor) -> Self {
        Self { p }
    }
}

impl<'a> RPCHandler for DefaultRPCHandler<'a> {
    fn handle_rpc(&mut self, rpc: &mut RPC) -> Result<()> {
        let mut msg = Message {
            header: MessageType::Tx,
            data: vec![],
        };
        let mut dec = BincodeDecoder::new(&mut rpc.payload);
        dec.decode(&mut msg)?;

        match msg.header {
            MessageType::Tx => {
                let mut tx = Transaction::new(vec![]);
                let mut dec = BincodeDecoder::new(&mut rpc.payload);
                dec.decode(&mut tx)?;
                self.p.process_transaction(&rpc.from, tx)?;
            }
            MessageType::Block => {}
            _ => {
                println!("unhandled message type");
            }
        }
        Ok(())
    }
}

pub trait RPCProcessor {
    fn process_transaction(&mut self, net_addr: &NetAddr, tx: Transaction) -> Result<()>;
}
