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
    let mut tr_remote_a = Box::new(network::LocalTransport::new("REMOTE_A".into()));
    let mut tr_remote_b = Box::new(network::LocalTransport::new("REMOTE_B".into()));
    let tr_remote_c = Box::new(network::LocalTransport::new("REMOTE_C".into()));

    tr_local.connect(tr_remote_a.clone()).await?;
    tr_remote_a.connect(tr_remote_b.clone()).await?;
    tr_remote_b.connect(tr_remote_c.clone()).await?;

    init_remote_servers(vec![tr_remote_a, tr_remote_b, tr_remote_c]).await?;

    // tokio::task::spawn(async move {
    //     loop {
    //         if let Err(err) = send_transaction(tr_remote_clone.clone(), tr_local_clone.addr()).await
    //         {
    //             println!("Error: {err}");
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //     }
    // });

    let private_key = PrivateKey::generate();
    let mut local_server = make_server("LOCAL".into(), tr_local, Some(private_key)).await?;
    local_server.start().await;

    Ok(())
}

async fn init_remote_servers(trs: Vec<Box<dyn Transport>>) -> Result<()> {
    for (i, tr) in trs.into_iter().enumerate() {
        tokio::task::spawn(async move {
            let id = format!("ID{i}");
            let mut s = make_server(id, tr, None).await.unwrap();
            s.start().await;
        });
    }
    Ok(())
}

async fn make_server(
    id: String,
    tr: Box<dyn Transport>,
    private_key: Option<PrivateKey>,
) -> Result<Server> {
    let opts = network::ServerOpts {
        id,
        transports: vec![tr],
        private_key,
        block_time: None,
        rpc_decode_fn: None,
    };
    let s = Server::new(opts).await?;
    Ok(s)
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
