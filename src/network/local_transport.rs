/*
LocalTransport is used for testing the network layer.
It is a simple in-memory transport that does not actually send messages over the network.
*/

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex, RwLock};

use super::{
    server::Channel,
    transport::{NetAddr, Rpc, Transport},
};

#[derive(Debug, Clone)]
pub struct LocalTransport {
    data: Arc<RwLock<LocalTransportData>>,
}

impl LocalTransport {
    pub fn new(addr: NetAddr) -> Self {
        Self {
            data: Arc::new(RwLock::new(LocalTransportData::new(addr))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalTransportData {
    addr: NetAddr,
    consume_channel: Channel<Rpc>,
    peers: HashMap<NetAddr, Box<dyn Transport>>,
}

impl LocalTransportData {
    fn new(addr: NetAddr) -> Self {
        Self {
            addr,
            consume_channel: Arc::new(Mutex::new(mpsc::channel(1024))),
            peers: HashMap::new(),
        }
    }
}

#[async_trait]
impl Transport for LocalTransport {
    async fn consume(&self) -> Channel<Rpc> {
        self.data.read().await.consume_channel.clone()
    }

    async fn connect(&self, tr: Box<dyn Transport>) -> Result<()> {
        let t = &mut self.data.write().await;
        t.peers.insert(tr.addr().await, tr);
        Ok(())
    }

    async fn send_message(&self, to: NetAddr, payload: Vec<u8>) -> Result<()> {
        let t = &mut self.data.write().await;
        let peer =
            t.peers
                .get(&to)
                .ok_or(anyhow!("{} could not send message to {}", t.addr, to))?;

        peer.consume()
            .await
            .lock()
            .await
            .0
            .send(Rpc {
                from: t.addr.clone(),
                payload,
            })
            .await?;

        Ok(())
    }

    async fn addr(&self) -> NetAddr {
        self.data.read().await.addr.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() -> Result<()> {
        let tr_a = LocalTransport::new("A".into());
        let tr_b = LocalTransport::new("B".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_b.connect(Box::new(tr_a.clone())).await?;

        assert_eq!(
            tr_a.data.read().await.peers[&tr_b.addr().await]
                .addr()
                .await,
            tr_b.addr().await
        );
        assert_eq!(
            tr_b.data.read().await.peers[&tr_a.addr().await]
                .addr()
                .await,
            tr_a.addr().await
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_send_message() -> Result<()> {
        let tr_a = LocalTransport::new("A".into());
        let tr_b = LocalTransport::new("B".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_b.connect(Box::new(tr_a.clone())).await?;

        let msg = b"hello world!".to_vec();
        tr_a.send_message(tr_b.addr().await, msg.clone()).await?;

        let rpc = tr_b.consume().await.lock().await.1.recv().await.unwrap();
        assert_eq!(rpc.from, tr_a.addr().await);
        assert_eq!(rpc.payload, msg.to_vec());

        Ok(())
    }
}
