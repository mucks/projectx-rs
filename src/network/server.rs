use std::sync::Arc;

use tokio::{
    sync::{mpsc, Mutex},
    time,
};

use super::transport::{Rpc, Transport};

// Sender can be passed within threads safely and cloned as many times as needed.
// Receiver needs to be wrapped in a Mutex to be shared across threads and can only be accessed once at a time.
pub type Channel<T> = (mpsc::Sender<T>, Arc<Mutex<mpsc::Receiver<T>>>);

pub fn new_channel<T>(buffer_size: usize) -> Channel<T> {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, Arc::new(Mutex::new(rx)))
}

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
            rpc_channel: new_channel(1024),
            quit_channel: new_channel(1),
        }
    }

    pub async fn start(&self) {
        self.init_transports();
        let x_seconds = 5;
        let mut ticker = time::interval(time::Duration::from_secs(x_seconds));

        loop {
            if let Some(rpc) = self.rpc_channel.1.lock().await.recv().await {
                println!("RPC: {rpc:?}\n");
            } else if self.quit_channel.1.lock().await.recv().await.is_some() {
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
