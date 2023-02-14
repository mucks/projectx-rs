// currently not using these traits because i couldn't get it to work with mutable references

use std::io::Cursor;

use crate::core::{BincodeDecoder, BincodeEncoder, Block, Decoder, Encoder, Transaction};

use super::transport::NetAddr;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum MessageType {
    Tx,
    Block,
}

#[derive(Debug, Clone)]
pub struct RPC {
    pub from: NetAddr,
    pub payload: Vec<u8>,
}

pub enum DecodedMessageData {
    Tx(Transaction),
    Block(Block),
}

pub struct DecodedMessage {
    pub from: NetAddr,
    pub data: DecodedMessageData,
}

pub type RPCDecodeFn = Box<dyn FnMut(RPC) -> Result<DecodedMessage>>;

pub fn default_rpc_decode_fn(mut rpc: RPC) -> Result<DecodedMessage> {
    let mut msg = Message {
        header: MessageType::Tx,
        data: vec![],
    };

    let mut cursor = Cursor::new(&mut rpc.payload);
    let mut dec = BincodeDecoder::new(&mut cursor);
    dec.decode(&mut msg)
        .map_err(|err| anyhow!("invalid message header! error: {}", err))?;

    match msg.header {
        MessageType::Tx => {
            let mut tx = Transaction::new(vec![]);
            let mut cursor = Cursor::new(&mut msg.data);
            let mut dec = BincodeDecoder::new(&mut cursor);
            dec.decode(&mut tx)?;
            Ok(DecodedMessage {
                from: rpc.from.clone(),
                data: DecodedMessageData::Tx(tx),
            })
        }
        // MessageType::Block => {}
        _ => Err(anyhow!("unhandled message type")),
    }
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

pub trait RPCProcessor {
    fn process_transaction(&mut self, net_addr: &NetAddr, tx: Transaction) -> Result<()>;
}
