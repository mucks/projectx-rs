use crate::core::{BincodeEncoder, Encoder, Transaction};

use anyhow::Result;
use crypto::PrivateKey;
use log::{error, info};
use network::{GetStatusMessage, Message, MessageType, NetAddr, Server, Transport};

mod core;
mod crypto;
mod network;
mod types;

fn transports() -> Vec<Box<dyn Transport>> {
    let tr_local = Box::new(network::LocalTransport::new("LOCAL".into()));
    let tr_remote_a = Box::new(network::LocalTransport::new("REMOTE_A".into()));
    let tr_remote_b = Box::new(network::LocalTransport::new("REMOTE_B".into()));
    let tr_remote_c = Box::new(network::LocalTransport::new("REMOTE_C".into()));
    vec![tr_local, tr_remote_a, tr_remote_b, tr_remote_c]
}

// Connect all nodes to each other
async fn bootstrap_nodes(
    transports: Vec<Box<dyn Transport>>,
    transports_mut: &mut Vec<Box<dyn Transport>>,
) -> Result<()> {
    for tr_mut in transports_mut {
        for tr in &transports {
            if tr.addr() != tr_mut.addr() {
                if let Err(err) = tr_mut.connect(tr.clone()).await {
                    error!("could not connect to remote error: {:?}", err);
                }
                info!(
                    "we {} connected to remote node: {}",
                    tr_mut.addr(),
                    tr.addr()
                );
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // tr_local.connect(tr_remote_a.clone()).await?;
    // tr_remote_a.connect(tr_remote_b.clone()).await?;
    // tr_remote_b.connect(tr_remote_c.clone()).await?;
    // tr_remote_b.connect(tr_remote_a.clone()).await?;
    // tr_remote_a.connect(tr_local.clone()).await?;

    // let tr_local_clone = tr_local.clone();
    // let tr_remote_a_clone = tr_remote_a.clone();

    let mut transports_mut = transports();
    let transports = transports_mut.clone();
    bootstrap_nodes(transports, &mut transports_mut).await?;

    let tr_local = transports_mut[0].clone();

    init_remote_servers(transports_mut.clone()).await?;

    // tokio::task::spawn(async move {
    //     loop {
    //         if let Err(err) =
    //             send_transaction(tr_remote_a_clone.clone(), tr_local_clone.addr()).await
    //         {
    //             println!("Error: {err}");
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //     }
    // });
    // send_get_status_message(tr_remote_a, "REMOTE_B".to_string()).await?;

    let private_key = PrivateKey::generate();
    let mut local_server = make_server(
        "LOCAL".into(),
        tr_local,
        transports_mut.clone(),
        Some(private_key),
    )
    .await?;
    local_server.start().await?;

    Ok(())
}

async fn init_remote_servers(trs: Vec<Box<dyn Transport>>) -> Result<()> {
    let transports = trs.clone();
    for (i, tr) in trs.into_iter().enumerate() {
        let transports = transports.clone();
        tokio::task::spawn(async move {
            let id = format!("REMOTE_{i}");
            let mut s = make_server(id, tr, transports, None).await.unwrap();
            s.start().await.unwrap();
        });
    }
    Ok(())
}

async fn make_server(
    id: String,
    tr: Box<dyn Transport>,
    transports: Vec<Box<dyn Transport>>,
    private_key: Option<PrivateKey>,
) -> Result<Server> {
    let opts = network::ServerOpts {
        transport: tr.clone_box(),
        id,
        transports,
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
