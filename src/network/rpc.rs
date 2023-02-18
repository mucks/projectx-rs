// currently not using these traits because i couldn't get it to work with mutable references

use super::{transport::NetAddr, GetBlocksMessage};
use crate::{
    core::{BincodeDecoder, BincodeEncoder, Block, Decoder, Encoder, Header, Transaction},
    network::message::StatusMessage,
};
use anyhow::{anyhow, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Deserialize, Serialize, Debug)]
pub enum MessageType {
    Tx = 0x01,
    Block = 0x02,
    GetBlocks = 0x03,
    Status = 0x04,
    GetStatus = 0x05,
}

#[derive(Debug, Clone)]
pub struct RPC {
    pub from: NetAddr,
    pub payload: Vec<u8>,
}

pub enum DecodedMessageData {
    Tx(Transaction),
    Block(Block),
    StatusMessage(StatusMessage),
    GetStatusMessage,
    GetBlocksMessage(GetBlocksMessage),
}

pub struct DecodedMessage {
    pub from: NetAddr,
    pub data: DecodedMessageData,
}

pub type RPCDecodeFn = Box<dyn Fn(RPC) -> Result<DecodedMessage> + Send + Sync>;

pub fn default_rpc_decode_fn(mut rpc: RPC) -> Result<DecodedMessage> {
    let mut msg = Message {
        header: MessageType::Tx,
        data: vec![],
    };

    let mut cursor = Cursor::new(&mut rpc.payload);
    let mut dec = BincodeDecoder::new(&mut cursor);

    dec.decode(&mut msg)
        .map_err(|err| anyhow!("invalid message header! error: {}", err))?;

    debug!(
        "new incoming message from {} of type : {:?}",
        rpc.from, msg.header
    );

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
        MessageType::Block => {
            let mut block = Block::new(Header::default(), vec![]);
            let mut cursor = Cursor::new(&mut msg.data);
            let mut dec = BincodeDecoder::new(&mut cursor);
            dec.decode(&mut block)?;
            Ok(DecodedMessage {
                from: rpc.from.clone(),
                data: DecodedMessageData::Block(block),
            })
        }
        MessageType::GetStatus => Ok(DecodedMessage {
            from: rpc.from.clone(),
            data: DecodedMessageData::GetStatusMessage,
        }),
        MessageType::Status => {
            let mut message = StatusMessage::new("".into(), 0, 0);
            let mut cursor = Cursor::new(&mut msg.data);
            let mut dec = BincodeDecoder::new(&mut cursor);
            dec.decode(&mut message)?;
            Ok(DecodedMessage {
                from: rpc.from.clone(),
                data: DecodedMessageData::StatusMessage(message),
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
