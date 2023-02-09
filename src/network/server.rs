use std::sync::Arc;

use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use super::transport::{Rpc, Transport};

pub type Channel<T> = Arc<Mutex<(mpsc::Sender<T>, mpsc::Receiver<T>)>>;

pub struct ServerOpts {
    pub transports: Vec<Box<dyn Transport>>,
}

pub struct Server {
    opts: ServerOpts,
    rpc_channel: Channel<Rpc>,
    quit_channel: Channel<()>,
}

impl Server {
    pub fn new(opts: ServerOpts) -> Self {
        Self {
            opts,
            rpc_channel: Arc::new(Mutex::new(mpsc::channel(1024))),
            quit_channel: Arc::new(Mutex::new(mpsc::channel(1))),
        }
    }

    pub async fn start(&self) {
        self.init_transports();
        let x_seconds = 5;
        let mut ticker = time::interval(time::Duration::from_secs(x_seconds));

        loop {
            if let Some(rpc) = self.rpc_channel.lock().await.1.recv().await {
                println!("RPC: {rpc:?}");
            } else if self.quit_channel.lock().await.1.recv().await.is_some() {
                break;
            } else {
                ticker.tick().await;
                println!("Do Stuff every x seconds");
            }
        }

        println!("Server shutdown");
    }

    fn init_transports(&self) {
        for tr in self.opts.transports.clone().into_iter() {
            let rpc_channel = self.rpc_channel.clone();
            tokio::task::spawn(async move {
                loop {
                    if let Some(rpc) = tr.consume().await.lock().await.1.recv().await {
                        println!("RPC in init_transports: {rpc:?}");
                        if let Err(err) = rpc_channel.lock().await.0.send(rpc).await {
                            println!("RPC Error: {err}");
                        }
                    }
                }
            });
        }
    }
}
