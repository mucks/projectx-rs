use anyhow::Result;
use log::info;
use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use crate::{
    core::{Hasher, Transaction, TxHasher},
    crypto::PrivateKey,
};

use super::{
    transport::{Rpc, Transport},
    tx_pool::TxPool,
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
    opts: ServerOpts,
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

    pub async fn start(&self) {
        self.init_transports();
        let mut ticker = time::interval(self.block_time);

        loop {
            if let Some(rpc) = self.rpc_channel.1.lock().await.recv().await {
                println!("RPC: {rpc:?}\n");
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

    fn handle_transaction(&mut self, mut tx: Transaction) -> Result<()> {
        tx.verify()?;

        let hash = tx.hash(Box::new(TxHasher))?;

        if self.mem_pool.has(&hash) {
            info!("mem_pool already contains tx {}", hash);
        }

        info!("Adding new tx {} to mem_pool", hash);

        self.mem_pool.add(tx)?;
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
