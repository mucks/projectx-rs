use anyhow::{anyhow, Result};
use log::{debug, error, info};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    core::{BincodeEncoder, Block, BlockHasher, Blockchain, Transaction, TxHasher},
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
    pub id: String,
}

pub struct Server {
    pub opts: ServerOpts,
    mem_pool: Arc<Mutex<TxPool>>,
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

        let bc = Blockchain::new(opts.id.clone(), Block::genesis()).await?;
        let chain = Arc::new(Mutex::new(bc));

        Ok(Self {
            chain,
            rpc_channel: new_channel(1024),
            mem_pool: Arc::new(Mutex::new(TxPool::new(1000))),
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
            let tx_pool = self.mem_pool.clone();
            let transports = self.opts.transports.clone();
            tokio::task::spawn(async move {
                Self::validator_loop(bc, tx_pool, private_key, block_time, transports).await;
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
                            if let Err(err) = self.process_message(msg).await {
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
        tx_pool: Arc<Mutex<TxPool>>,
        private_key: PrivateKey,
        block_time: Duration,
        transports: Vec<Box<dyn Transport>>,
    ) {
        let mut ticker = time::interval(block_time);

        info!(
            "Starting validator loop with block_time {}",
            block_time.as_secs()
        );

        loop {
            ticker.tick().await;
            let mut bc = bc.lock().await;
            let mut tx_pool = tx_pool.lock().await;
            info!("Creating a new block");
            if let Err(err) = Self::create_new_block(
                &mut bc,
                &mut tx_pool,
                private_key.clone(),
                transports.clone(),
            )
            .await
            {
                error!("Error creating a new block: {}", err);
            }
        }
    }

    pub async fn process_message(&mut self, msg: DecodedMessage) -> Result<()> {
        match msg.data {
            DecodedMessageData::Tx(tx) => {
                if let Err(err) = self.process_transaction(&msg.from, tx).await {
                    error!("Error processing transaction: {err}");
                };
            }
            DecodedMessageData::Block(block) => {
                println!("Received a new Block");
            }
        }
        Ok(())
    }

    pub async fn broadcast_block(transports: &[Box<dyn Transport>], b: &Block) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        b.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Block, buf);

        Self::broadcast(transports, msg.bytes()?).await?;

        Ok(())
    }

    pub async fn broadcast(transports: &[Box<dyn Transport>], payload: Vec<u8>) -> Result<()> {
        for tr in transports {
            tr.broadcast(payload.clone()).await?;
        }
        Ok(())
    }

    pub async fn broadcast_tx(transports: &[Box<dyn Transport>], tx: &Transaction) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        tx.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Tx, buf);
        Self::broadcast(transports, msg.bytes()?).await?;
        //let buf: Vec<u8> = Vec::new();
        Ok(())
    }

    pub async fn process_transaction(
        &mut self,
        net_addr: &NetAddr,
        mut tx: Transaction,
    ) -> Result<()> {
        tx.calculate_and_cache_hash(Box::new(TxHasher));

        let hash = tx.hash();
        let mut mem_pool = self.mem_pool.lock().await;

        if mem_pool.has(&hash) {
            debug!("Tx {} already in mem_pool", hash);
            return Ok(());
        }

        tx.verify()?;
        tx.set_first_seen(Instant::now().elapsed().as_nanos());

        info!(
            "Adding new tx {} to mem_pool (len: {})",
            hash,
            mem_pool.len()
        );

        // TODO: broadcast this tx to peers

        let transports = self.opts.transports.clone();
        let tx_clone = tx.clone();

        tokio::task::spawn(async move {
            if let Err(err) = Self::broadcast_tx(&transports, &tx_clone).await {
                error!("Error broadcasting tx: {err}");
            }
        });

        mem_pool.add(tx)?;

        Ok(())
    }

    pub async fn create_new_block(
        bc: &mut Blockchain,
        tx_pool: &mut TxPool,
        private_key: PrivateKey,
        transports: Vec<Box<dyn Transport>>,
    ) -> Result<()> {
        let prev_header = bc.get_header(bc.height().await).await?;

        // For now we're going to use all transactions that are in the mempool
        // Later on when we know the internal structure of our transaction
        // we will implement some kind of complexity function
        // to determine how many transactions can be inculded in a block
        let txx = tx_pool.pending_cloned();

        let mut block = Block::from_prev_header(prev_header, txx)?;
        info!("Creating new block with height {}", block.header.height);

        block.sign(&private_key)?;
        bc.add_block(&mut block).await?;

        //TODO: pending pool of tx should only reflect on validator nodes
        tx_pool.clear_pending();

        tokio::task::spawn(async move {
            if let Err(err) = Self::broadcast_block(&transports, &block).await {
                error!("Error broadcasting tx: {err}");
            }
        });

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
