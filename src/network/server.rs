use anyhow::{anyhow, Result};
use log::{debug, error, info};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    core::{BincodeEncoder, Block, Blockchain, Transaction, TxHasher},
    crypto::PrivateKey,
    network::DecodedMessageData,
    types::Hash,
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
    chain: Arc<Mutex<Blockchain>>,
    is_validator: bool,
    rpc_channel: Channel<RPC>,
    quit_channel: Channel<()>,
}

impl Server {
    pub async fn new(mut opts: ServerOpts) -> Result<Self> {
        if opts.block_time.is_none() {
            opts.block_time = Some(Duration::from_secs(5));
        }

        if opts.rpc_decode_fn.is_none() {
            opts.rpc_decode_fn = Some(Box::new(default_rpc_decode_fn));
        }

        let chain = Arc::new(Mutex::new(Blockchain::new(Block::genesis()).await?));

        Ok(Self {
            chain,
            rpc_channel: new_channel(1024),
            mem_pool: TxPool::new(),
            quit_channel: new_channel(1),
            is_validator: opts.private_key.is_some(),
            opts,
        })
    }

    pub async fn start(&mut self) {
        self.init_transports();

        if self.is_validator {
            let block_time = self.opts.block_time.unwrap();
            let bc = self.chain.clone();
            let private_key = self.opts.private_key.as_ref().unwrap().clone();
            tokio::task::spawn(async move {
                Self::validator_loop(bc, private_key, block_time).await;
            });
        }

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
            }
        }

        info!("Server is shutting down");
    }

    pub async fn validator_loop(
        bc: Arc<Mutex<Blockchain>>,
        private_key: PrivateKey,
        block_time: Duration,
    ) {
        let mut ticker = time::interval(block_time);

        info!(
            "Starting validator loop with block_time {}",
            block_time.as_secs()
        );

        loop {
            ticker.tick().await;
            let mut bc = bc.lock().await;
            info!("Creating a new block");
            if let Err(err) = Self::create_new_block(&mut bc, private_key.clone()).await {
                error!("Error creating a new block: {}", err);
            }
        }
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

    pub async fn create_new_block(bc: &mut Blockchain, private_key: PrivateKey) -> Result<()> {
        let prev_header = bc.get_header(bc.height().await).await?;
        let mut block = Block::from_prev_header(prev_header, vec![])?;
        info!("Creating new block with height {}", block.header.height);
        block.sign(&private_key)?;
        bc.add_block(&mut block).await?;

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
