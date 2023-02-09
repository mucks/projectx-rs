// Server
// Transport Layer => tcp, udp
// Block
// Tx
// Keypair

mod error;
pub use error::{Error, Result};
use network::{Server, Transport};

mod network;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tr_local = Box::new(network::LocalTransport::new("LOCAL".into()));
    let tr_remote = Box::new(network::LocalTransport::new("REMOTE".into()));

    tr_local.connect(tr_remote.clone()).await?;
    tr_remote.connect(tr_local.clone()).await?;

    let tr_local_clone = tr_local.clone();
    let tr_remote_clone = tr_remote.clone();

    tokio::task::spawn(async move {
        loop {
            println!("Sending message from LOCAL to REMOTE");
            tr_remote_clone
                .send_message("LOCAL".into(), b"Hello World".to_vec())
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
