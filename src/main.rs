// Server
// Transport Layer => tcp, udp
// Block
// Tx
// Keypair

use crate::core::{BincodeEncoder, Transaction};

use anyhow::Result;
use crypto::PrivateKey;
use network::{Message, NetAddr, Server, Transport};
use rand::{thread_rng, Rng};

mod core;
mod crypto;
mod network;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut tr_local = Box::new(network::LocalTransport::new("LOCAL".into()));
    let mut tr_remote = Box::new(network::LocalTransport::new("REMOTE".into()));

    tr_local.connect(tr_remote.clone()).await?;
    tr_remote.connect(tr_local.clone()).await?;

    let tr_local_clone = tr_local.clone();
    let tr_remote_clone = tr_remote.clone();

    tokio::task::spawn(async move {
        loop {
            if let Err(err) = send_transaction(tr_remote_clone.clone(), tr_local_clone.addr()).await
            {
                println!("Error: {err}");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let opts = network::ServerOpts {
        transports: vec![tr_local],
        private_key: None,
        block_time: None,
        rpc_decode_fn: None,
    };

    let mut s = Server::new(opts);

    s.start().await;

    Ok(())
}

async fn send_transaction(tr: Box<dyn Transport>, to: NetAddr) -> Result<()> {
    let priv_key = PrivateKey::generate();
    let data = thread_rng().gen::<[u8; 32]>();
    let mut tx = Transaction::new(data.to_vec());
    tx.sign(&priv_key);
    let mut buf: Vec<u8> = Vec::new();
    tx.encode(&mut BincodeEncoder::new(&mut buf))?;

    let msg = Message::new(network::MessageType::Tx, buf);

    tr.send_message(&to, msg.bytes()?).await?;
    Ok(())
}
