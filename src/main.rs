// Server
// Transport Layer => tcp, udp
// Block
// Tx
// Keypair

use crate::core::{BincodeEncoder, Encoder, Transaction};

use anyhow::Result;
use crypto::PrivateKey;
use network::{GetStatusMessage, Message, MessageType, NetAddr, Server, Transport};

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
    tr_remote_b.connect(tr_remote_a.clone()).await?;

    tr_remote_a.connect(tr_local.clone()).await?;

    let tr_local_clone = tr_local.clone();
    let tr_remote_a_clone = tr_remote_a.clone();

    init_remote_servers(vec![tr_remote_a.clone(), tr_remote_b, tr_remote_c]).await?;

    tokio::task::spawn(async move {
        loop {
            if let Err(err) =
                send_transaction(tr_remote_a_clone.clone(), tr_local_clone.addr()).await
            {
                println!("Error: {err}");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    });

    send_get_status_message(tr_remote_a, "REMOTE_B".to_string()).await?;

    let private_key = PrivateKey::generate();
    let mut local_server = make_server("LOCAL".into(), tr_local, Some(private_key)).await?;
    local_server.start().await;

    Ok(())
}

async fn init_remote_servers(trs: Vec<Box<dyn Transport>>) -> Result<()> {
    for (i, tr) in trs.into_iter().enumerate() {
        tokio::task::spawn(async move {
            let id = format!("REMOTE_{i}");
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
        transport: tr.clone_box(),
        id,
        transports: vec![tr],
        private_key,
        block_time: None,
        rpc_decode_fn: None,
    };
    let s = Server::new(opts).await?;
    Ok(s)
}

async fn send_get_status_message(tr: Box<dyn Transport>, to: NetAddr) -> Result<()> {
    let status_msg = GetStatusMessage {};
    let mut buf = vec![];
    BincodeEncoder::new(&mut buf).encode(&status_msg)?;

    let msg = Message::new(MessageType::GetStatus, buf);
    tr.send_message(&to, msg.bytes()?).await?;

    Ok(())
}

async fn send_transaction(tr: Box<dyn Transport>, to: NetAddr) -> Result<()> {
    let priv_key = PrivateKey::generate();
    let contract = contract();
    let mut tx = Transaction::new(contract);
    tx.sign(&priv_key);
    let mut buf: Vec<u8> = Vec::new();
    tx.encode(&mut BincodeEncoder::new(&mut buf))?;

    let msg = Message::new(MessageType::Tx, buf);

    tr.send_message(&to, msg.bytes()?).await?;
    Ok(())
}

fn contract() -> Vec<u8> {
    let mut data = vec![
        0x02, 0x0a, 0x03, 0x0a, 0x0b, 0x4f, 0x0c, 0x4f, 0x0c, 0x46, 0x0c, 0x03, 0x0a, 0x0d, 0x0f,
    ];
    let push_foo = vec![0x4f, 0x0c, 0x4f, 0x0c, 0x46, 0x0c, 0x03, 0x0a, 0x0d, 0x0ae];
    data.extend(push_foo);
    data
}
