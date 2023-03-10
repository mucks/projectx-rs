use crate::core::{BincodeEncoder, Encoder, Transaction};

use anyhow::Result;
use crypto::PrivateKey;
use log::{error, info};
use network::{BTransport, Message, MessageType, NetAddr, Server, Transport};

mod core;
mod crypto;
mod network;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let transports = transports();

    let tr_local = transports[0].clone();
    let tr_late = transports[1].clone();

    late_server_task(transports.clone(), tr_local.clone(), tr_late.clone());

    let private_key = PrivateKey::generate();
    let mut local_server = make_server(
        "LOCAL_SERVER".into(),
        tr_local,
        transports,
        Some(private_key),
    )
    .await?;
    local_server.start().await?;

    Ok(())
}

fn new_local_transport(name: &str) -> BTransport {
    Box::new(network::LocalTransport::new(name.into()))
}

fn transports() -> Vec<BTransport> {
    vec![
        new_local_transport("LOCAL"),
        new_local_transport("LATE_REMOTE"),
    ]
}

async fn late_node(
    transports: Vec<BTransport>,
    tr_local: BTransport,
    tr_late: BTransport,
) -> Result<()> {
    tokio::time::sleep(tokio::time::Duration::from_secs(7)).await;
    tr_late.connect(tr_local.clone()).await?;
    tr_local.connect(tr_late.clone()).await?;

    let mut late_server = make_server(
        "LATE_SERVER".into(),
        tr_late.clone(),
        transports.clone(),
        None,
    )
    .await?;

    late_server.start().await?;

    Ok(())
}

fn late_server_task(transports: Vec<BTransport>, tr_local: BTransport, tr_late: BTransport) {
    tokio::task::spawn(async move {
        if let Err(err) = late_node(transports, tr_late, tr_local).await {
            error!("{}", err)
        }
    });
}

fn send_initial_transaction() {
    // let mut tr_local_clone = transports_mut[0].clone();
    // let tr_remote_a = transports_mut[1].clone();

    //Send a transaction from remote_a to local
    // tokio::task::spawn(async move {
    //     loop {
    //         if let Err(err) = send_transaction(tr_remote_a.clone(), tr_local_clone.addr()).await {
    //             println!("Error: {err}");
    //         }

    //         tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    //     }
    // });
}

// Connect all nodes to each other
// async fn bootstrap_nodes(transports: &Vec<BTransport>) -> Result<()> {
//     for tr_mut in transports {
//         for tr in transports {
//             let tr = tr.read().await;

//             if tr.addr() != tr_mut.read().await.addr() {
//                 if let Err(err) = tr_mut.write().await.connect(tr.clone()) {
//                     error!("could not connect to remote error: {:?}", err);
//                 }
//                 info!(
//                     "we {} connected to remote node: {}",
//                     tr_mut.read().await.addr(),
//                     tr.addr()
//                 );
//             }
//         }
//     }

//     Ok(())
// }

async fn init_remote_servers(trs: Vec<BTransport>) -> Result<()> {
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
    tr: BTransport,
    transports: Vec<BTransport>,
    private_key: Option<PrivateKey>,
) -> Result<Server> {
    let opts = network::ServerOpts {
        transport: tr.clone(),
        id,
        transports,
        private_key,
        block_time: None,
        rpc_decode_fn: None,
    };
    let s = Server::new(opts).await?;
    Ok(s)
}

async fn send_transaction(tr: BTransport, to: NetAddr) -> Result<()> {
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
