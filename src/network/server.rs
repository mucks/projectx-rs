use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    core::{BincodeEncoder, Block, BlockHasher, Blockchain, Encoder, Transaction, TxHasher},
    crypto::PrivateKey,
    network::DecodedMessageData,
};
use tokio::{sync::Mutex, time};

use super::{
    default_rpc_decode_fn,
    message::{GetStatusMessage, StatusMessage},
    new_channel,
    transport::NetAddr,
    tx_pool::TxPool,
    BTransport, Channel, DecodedMessage, GetBlocksMessage, Message, MessageType, RPCDecodeFn,
    Transport, RPC,
};

pub struct ServerOpts {
    pub rpc_decode_fn: Option<RPCDecodeFn>,
    pub transports: Vec<BTransport>,
    pub private_key: Option<PrivateKey>,
    pub block_time: Option<Duration>,
    pub id: String,
    pub transport: BTransport,
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
            mem_pool: Arc::new(Mutex::new(TxPool::new(100))),
            quit_channel: new_channel(1),
            is_validator: opts.private_key.is_some(),
            opts,
        })
    }

    pub async fn get_status_from_transports(
        self_tr: BTransport,
        transports: Vec<BTransport>,
    ) -> Result<()> {
        for tr in transports {
            if tr.addr() != self_tr.addr() {
                if let Err(err) = Self::send_get_status_message(&self_tr, &tr.addr()).await {
                    error!("Send get_status_message error: {:?}", err);
                }
            }
        }
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        // println!("{:?}", self.opts.transports);
        self.init_transports();
        {
            let transports = self.opts.transports.clone();
            let tr = self.opts.transport.clone();
            tokio::task::spawn(async move {
                Self::get_status_from_transports(tr, transports)
                    .await
                    .unwrap();
            });
        }

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
                            debug!(
                                "ID={} RPC Message incoming from: {}, data: {:?}",
                                &self.opts.id, msg.from, msg.data
                            );

                            // if self.opts.transport.addr() == msg.from {
                            //     warn!("ID={} Message from self, ignoring", &self.opts.id);
                            //     continue;
                            // }

                            if let Err(err) = self.process_message(msg).await {
                                if err.to_string() != "block already known" {
                                    error!("ID={} error processing message: {}", self.opts.id, err);
                                }
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
        Ok(())
    }

    pub async fn validator_loop(
        bc: Arc<Mutex<Blockchain>>,
        tx_pool: Arc<Mutex<TxPool>>,
        private_key: PrivateKey,
        block_time: Duration,
        transports: Vec<BTransport>,
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

    // Send and Broadcast functions

    async fn send_get_status_message(tr: &BTransport, to: &NetAddr) -> Result<()> {
        let status_msg = GetStatusMessage {};
        let mut buf = vec![];
        BincodeEncoder::new(&mut buf).encode(&status_msg)?;

        let msg = Message::new(MessageType::GetStatus, buf);
        tr.send_message(to, msg.bytes()?).await?;

        Ok(())
    }

    pub async fn broadcast_block(transports: &Vec<BTransport>, b: &Block) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        b.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Block, buf);

        Self::broadcast(transports, msg.bytes()?).await?;

        Ok(())
    }

    pub async fn broadcast(transports: &Vec<BTransport>, payload: Vec<u8>) -> Result<()> {
        for tr in transports {
            tr.broadcast(payload.clone()).await?;
        }
        Ok(())
    }

    pub async fn broadcast_tx(transports: &Vec<BTransport>, tx: &Transaction) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        tx.encode(&mut BincodeEncoder::new(&mut buf))?;

        let msg = Message::new(MessageType::Tx, buf);
        Self::broadcast(transports, msg.bytes()?).await?;
        //let buf: Vec<u8> = Vec::new();
        Ok(())
    }

    // Process functions

    pub async fn process_message(&mut self, msg: DecodedMessage) -> Result<()> {
        match msg.data {
            DecodedMessageData::Tx(tx) => self.process_transaction(&msg.from, tx).await,
            DecodedMessageData::Block(block) => self.process_block(block).await,
            DecodedMessageData::StatusMessage(message) => {
                self.process_status_message(&msg.from, message).await
            }
            DecodedMessageData::GetStatusMessage => {
                let id = self.opts.id.clone();
                let tr = self.opts.transport.clone();
                let bc = self.chain.clone();
                let from = msg.from;
                tokio::task::spawn(async move {
                    Self::process_get_status_message(&id, tr, bc, &from).await
                });
                Ok(())
            }
            DecodedMessageData::GetBlocksMessage(get_block_message) => {
                self.process_get_blocks_message(&msg.from, &get_block_message)
                    .await
            }
        }
    }

    async fn process_get_blocks_message(
        &mut self,
        from: &NetAddr,
        data: &GetBlocksMessage,
    ) -> Result<()> {
        println!("got get blocks message => {}", data.to);

        Ok(())
    }

    pub async fn process_get_status_message(
        id: &str,
        tr: BTransport,
        bc: Arc<Mutex<Blockchain>>,
        from: &NetAddr,
    ) -> Result<()> {
        info!("ID={}, Received get_status_message from {}", id, from);
        let height = bc.lock().await.height().await;

        //TODO: get version from somewhere
        let status_msg = StatusMessage::new(id.to_string(), 0, height);

        let mut buf = vec![];
        BincodeEncoder::new(&mut buf).encode(&status_msg)?;

        let msg = Message::new(MessageType::Status, buf);
        info!("ID={}, sending status message to {}", id, from);

        let to = from.clone();

        tokio::task::spawn(async move {
            tr.send_message(&to, msg.bytes().unwrap()).await.unwrap();
        });

        Ok(())
    }

    pub async fn process_status_message(
        &mut self,
        from: &NetAddr,
        msg: StatusMessage,
    ) -> Result<()> {
        let our_height = self.chain.lock().await.height().await;
        info!(
            "ID={}, height: {}, received status message from: {}, height: {}",
            self.opts.id, our_height, from, msg.current_height
        );

        if msg.current_height <= our_height {
            warn!(
                "ID={} cannot sync block_height too low our height: {}, their height: {}, addr: {}",
                self.opts.id, our_height, msg.current_height, from
            );
            return Ok(());
        }
        info!(
            "ID={} syncing block_height our height: {}, their height: {}, addr: {}",
            self.opts.id, our_height, msg.current_height, from
        );

        // In this case we are behind and need to sync
        let get_blocks_msg = GetBlocksMessage {
            from: our_height + 1,
            to: msg.current_height,
        };

        let mut buf = vec![];
        BincodeEncoder::new(&mut buf).encode(&get_blocks_msg)?;

        let msg = Message::new(MessageType::GetBlocks, buf);

        let tr = self.opts.transport.clone();
        let to = from.to_owned();

        tokio::task::spawn(async move {
            tr.send_message(&to, msg.bytes().unwrap()).await.unwrap();
        });

        Ok(())
    }

    pub async fn process_block(&mut self, mut block: Block) -> Result<()> {
        {
            for tx in &mut block.transactions {
                if !tx.has_cached_hash() {
                    tx.calculate_and_cache_hash(Box::new(TxHasher))?;
                }
            }
        }
        // info!("Received block: {}", block.hash(Box::new(BlockHasher)));

        {
            self.chain.lock().await.add_block(&mut block).await?;
        }

        let transports = self.opts.transports.clone();

        tokio::task::spawn(async move {
            if let Err(err) = Self::broadcast_block(&transports, &block).await {
                error!("Error broadcasting block: {err}");
            }
        });

        Ok(())
    }

    pub async fn process_transaction(
        &mut self,
        net_addr: &NetAddr,
        mut tx: Transaction,
    ) -> Result<()> {
        tx.calculate_and_cache_hash(Box::new(TxHasher))?;

        let hash = tx.hash();
        let mut mem_pool = self.mem_pool.lock().await;

        if mem_pool.has(&hash) {
            debug!("Tx {} already in mem_pool", hash);
            return Ok(());
        }

        tx.verify()?;
        tx.set_first_seen(Instant::now().elapsed().as_nanos());

        info!(
            "ID={} Adding new tx {} to mem_pool (pending_count: {})",
            self.opts.id,
            hash,
            mem_pool.pending_count()
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
        transports: Vec<BTransport>,
    ) -> Result<()> {
        let prev_header = bc.get_header(bc.height().await).await?;

        // For now we're going to use all transactions that are in the mempool
        // Later on when we know the internal structure of our transaction
        // we will implement some kind of complexity function
        // to determine how many transactions can be inculded in a block
        let txx = tx_pool.pending_cloned();

        let mut block = Block::from_prev_header(prev_header, txx)?;
        info!(
            "ID={} Creating new block with height {}",
            bc.server_id, block.header.height
        );

        block.sign(&private_key)?;
        bc.add_block(&mut block).await?;

        //TODO: pending pool of tx should only reflect on validator nodes
        // Right now "normal nodes" don't have their pending pool cleared
        tx_pool.clear_pending();

        tokio::task::spawn(async move {
            if let Err(err) = Self::broadcast_block(&transports, &block).await {
                error!("Error broadcasting block: {err}");
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
