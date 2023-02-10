// Server
// Transport Layer => tcp, udp
// Block
// Tx
// Keypair

use network::{Server, Transport};

mod core;
mod crypto;
mod network;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut tr_local = Box::new(network::LocalTransport::new("LOCAL".into()));
    let mut tr_remote = Box::new(network::LocalTransport::new("REMOTE".into()));

    tr_local.connect(tr_remote.clone()).await?;
    tr_remote.connect(tr_local.clone()).await?;

    let tr_local_clone = tr_local.clone();
    let tr_remote_clone = tr_remote.clone();

    tokio::task::spawn(async move {
        loop {
            tr_remote_clone
                .send_message(tr_local_clone.addr(), b"Hello World".to_vec())
                .await
                .unwrap();

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let opts = network::ServerOpts {
        transports: vec![tr_local],
    };

    let s = Server::new(opts);
    s.start().await;

    Ok(())
}
