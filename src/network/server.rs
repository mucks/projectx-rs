use anyhow::{anyhow, Result};
use log::{debug, error, info};
use std::{
    io::Cursor,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    core::{BincodeDecoder, BincodeEncoder, Decoder, Transaction, TxHasher},
    crypto::PrivateKey,
    network::DecodedMessageData,
};
use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use super::{
    default_rpc_decode_fn,
    transport::{NetAddr, Transport},
    tx_pool::TxPool,
    DecodedMessage, Message, MessageType, RPCDecodeFn, RPC,
};

// Sender can be passed within threads safely and cloned as many times as needed.
// Receiver needs to be wrapped in a Mutex to be shared across threads and can only be accessed once at a time.
pub type Channel<T> = (mpsc::Sender<T>, Arc<Mutex<mpsc::Receiver<T>>>);

pub fn new_channel<T>(buffer_size: usize) -> Channel<T> {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, Arc::new(Mutex::new(rx)))
}

pub struct ServerOpts {
    pub rpc_decode_fn: Option<RPCDecodeFn>,
    pub transports: Vec<Box<dyn Transport>>,
    pub private_key: Option<PrivateKey>,
    pub block_time: Option<Duration>,
}

pub struct Server {
    pub opts: ServerOpts,
    mem_pool: TxPool,
    is_validator: bool,
    rpc_channel: Channel<RPC>,
    quit_channel: Channel<()>,
}

impl Server {
    pub fn new(mut opts: ServerOpts) -> Self {
        if opts.block_time.is_none() {
            opts.block_time = Some(Duration::from_secs(5));
        }

        if opts.rpc_decode_fn.is_none() {
            opts.rpc_decode_fn = Some(Box::new(default_rpc_decode_fn));
        }

        Self {
            rpc_channel: new_channel(1024),
            mem_pool: TxPool::new(),
            quit_channel: new_channel(1),
            is_validator: opts.private_key.is_some(),
            opts,
        }
    }

    pub async fn start(&mut self) {
        self.init_transports();
        let mut ticker = time::interval(self.opts.block_time.unwrap());

        loop {
            // Waits for an RPC message to arrive and then proccesses it with the dynamic function that's passed
            let opt_rpc =
                (|| async { return self.rpc_channel.1.lock().await.recv().await })().await;

            if let Some(rpc) = opt_rpc {
                if let Some(rpc_decode_fn) = self.opts.rpc_decode_fn.as_mut() {
                    match rpc_decode_fn(rpc) {
                        Ok(msg) => {
                            debug!("RPC Message incoming from: {} ", msg.from);
                            if let Err(err) = self.process_message(msg) {
                                error!("error processing message: {}", err);
                            };
                        }
                        Err(err) => error!("RPC Decoding Error: {err}"),
                    }
                }
            } else if self.quit_channel.1.lock().await.recv().await.is_some() {
                break;
            } else {
                ticker.tick().await;
                if self.is_validator {
                    self.create_new_block();
                }
            }
        }

        println!("Server shutdown");
    }

    pub fn process_message(&mut self, msg: DecodedMessage) -> Result<()> {
        match msg.data {
            DecodedMessageData::Tx(tx) => {
                if let Err(err) = self.process_transaction(&msg.from, tx) {
                    error!("Error processing transaction: {err}");
                };
            }
            DecodedMessageData::Block(block) => {
                println!("Received a new Block");
            }
        }
        Ok(())
    }
    // find some way to not have to clone the payload
    pub async fn broadcast(&self, payload: Vec<u8>) -> Result<()> {
        for tr in &self.opts.transports {
            tr.broadcast(payload.clone()).await?;
        }
        Ok(())
    }

    pub async fn broadcast_tx(&self, tx: &Transaction) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        tx.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Tx, buf);
        self.broadcast(msg.bytes()?).await?;
        //let buf: Vec<u8> = Vec::new();
        Ok(())
    }

    pub async fn broadcast_sync(transports: &[Box<dyn Transport>], payload: Vec<u8>) -> Result<()> {
        for tr in transports {
            tr.broadcast(payload.clone()).await?;
        }
        Ok(())
    }

    pub async fn broadcast_tx_sync(
        transports: &[Box<dyn Transport>],
        tx: &Transaction,
    ) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        tx.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Tx, buf);
        Self::broadcast_sync(transports, msg.bytes()?).await?;
        //let buf: Vec<u8> = Vec::new();
        Ok(())
    }

    pub fn process_transaction(&mut self, net_addr: &NetAddr, mut tx: Transaction) -> Result<()> {
        tx.verify()?;

        let hash = tx.hash(Box::new(TxHasher))?;

        if self.mem_pool.has(&hash) {
            info!("mem_pool already contains tx {}", hash);
        }

        tx.set_first_seen(Instant::now().elapsed().as_nanos());

        info!(
            "Adding new tx {} to mem_pool (len: {})",
            hash,
            self.mem_pool.len()
        );

        // TODO: broadcast this tx to peers

        let transports = self.opts.transports.clone();
        let tx_clone = tx.clone();

        tokio::task::spawn(async move {
            if let Err(err) = Self::broadcast_tx_sync(&transports, &tx_clone).await {
                error!("Error broadcasting tx: {err}");
            }
        });

        self.mem_pool.add(tx)?;

        Ok(())
    }

    fn create_new_block(&self) -> Result<()> {
        println!("Creating a new Block");
        Ok(())
    }

    fn init_transports(&self) {
        for tr in self.opts.transports.clone().into_iter() {
            let rpc_channel = self.rpc_channel.clone();
            tokio::task::spawn(async move {
                loop {
                    if let Some(rpc) = tr.recv().await {
                        if let Err(err) = rpc_channel.0.send(rpc).await {
                            println!("RPC Error: {err}");
                        }
                    }
                }
            });
        }
    }
}
