use anyhow::{anyhow, Result};
use log::{error, info};
use std::{
    io::Cursor,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    core::{BincodeDecoder, Decoder, Transaction, TxHasher},
    crypto::PrivateKey,
};
use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use super::{
    transport::{NetAddr, Rpc, Transport},
    tx_pool::TxPool,
    Message, MessageType, RPC,
};

// Sender can be passed within threads safely and cloned as many times as needed.
// Receiver needs to be wrapped in a Mutex to be shared across threads and can only be accessed once at a time.
pub type Channel<T> = (mpsc::Sender<T>, Arc<Mutex<mpsc::Receiver<T>>>);

pub fn new_channel<T>(buffer_size: usize) -> Channel<T> {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, Arc::new(Mutex::new(rx)))
}

pub struct ServerOpts {
    pub transports: Vec<Box<dyn Transport>>,
    pub private_key: Option<PrivateKey>,
    pub block_time: Option<Duration>,
}

pub struct Server {
    pub opts: ServerOpts,
    block_time: Duration,
    mem_pool: TxPool,
    is_validator: bool,
    rpc_channel: Channel<Rpc>,
    quit_channel: Channel<()>,
}

impl Server {
    pub fn new(opts: ServerOpts) -> Self {
        let mut bt: Duration = Duration::from_secs(5);

        if let Some(block_time) = opts.block_time {
            bt = block_time;
        }

        Self {
            rpc_channel: new_channel(1024),
            block_time: bt,
            mem_pool: TxPool::new(),
            quit_channel: new_channel(1),
            is_validator: opts.private_key.is_some(),
            opts,
        }
    }

    pub fn handle_rpc(&mut self, rpc: &mut RPC) -> Result<()> {
        let mut msg = Message {
            header: MessageType::Tx,
            data: vec![],
        };
        let mut dec = BincodeDecoder::new(&mut rpc.payload);
        dec.decode(&mut msg)
            .map_err(|err| anyhow!("invalid message header! error: {}", err))?;

        match msg.header {
            MessageType::Tx => {
                let mut tx = Transaction::new(vec![]);
                let mut cursor = Cursor::new(msg.data);
                let mut dec = BincodeDecoder::new(&mut cursor);
                dec.decode(&mut tx)?;
                self.process_transaction(&rpc.from, tx)?;
            }
            MessageType::Block => {}
            _ => {
                println!("unhandled message type");
            }
        }
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

        self.mem_pool.add(tx)?;
        Ok(())
    }

    pub async fn start(&mut self) {
        self.init_transports();
        let mut ticker = time::interval(self.block_time);

        loop {
            let mut opt_rpc: Option<Rpc> = None;
            {
                let mut rpc_channel = self.rpc_channel.1.lock().await;
                opt_rpc = rpc_channel.recv().await;
            }

            if let Some(mut rpc) = opt_rpc {
                // info!("Received RPC from {}", rpc.from);
                let mut cursor = Cursor::new(rpc.payload.as_mut_slice());

                let mut rpc = RPC {
                    from: rpc.from,
                    payload: &mut cursor,
                };

                if let Err(err) = self.handle_rpc(&mut rpc) {
                    error!("Error: {err}");
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
