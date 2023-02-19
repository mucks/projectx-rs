/*
LocalTransport is used for testing the network layer.
It is a simple in-memory transport that does not actually send messages over the network.
*/

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::info;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use super::{
    new_channel,
    transport::{NetAddr, Transport},
    Channel, RPC,
};

#[derive(Debug, Clone)]
pub struct LocalTransport {
    addr: NetAddr,
    consume_channel: Channel<RPC>,
    peers: Arc<RwLock<HashMap<NetAddr, Box<dyn Transport>>>>,
}

impl LocalTransport {
    pub fn new(addr: NetAddr) -> Self {
        Self {
            addr,
            consume_channel: new_channel(1024),
            // so we can send transport between threads and have a shared state
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Transport for LocalTransport {
    fn consume(&self) -> Channel<RPC> {
        self.consume_channel.clone()
    }

    async fn recv(&self) -> Option<RPC> {
        self.consume_channel.1.lock().await.recv().await
    }

    async fn connect(&mut self, tr: Box<dyn Transport>) -> Result<()> {
        self.peers.write().await.insert(tr.addr(), tr);
        Ok(())
    }

    async fn send_message(&self, to: &NetAddr, payload: Vec<u8>) -> Result<()> {
        if &self.addr == to {
            return Ok(());
        }
        let peers = self.peers().await;
        let peer =
            peers
                .get(to)
                .ok_or(anyhow!("{} could not send message to {}", self.addr, to))?;

        info!("Sending Message from {} to {}", self.addr, to);
        info!("Peers: {:?}", peers);

        // println!("self_addr: {}, to: {}, Peer: {:?}", self.addr, to, peer);

        peer.consume()
            .0
            .send(RPC {
                from: self.addr.clone(),
                payload,
            })
            .await?;

        Ok(())
    }

    async fn broadcast(&self, payload: Vec<u8>) -> Result<()> {
        for peer in self.peers.read().await.iter() {
            self.send_message(peer.0, payload.clone()).await?;
        }
        Ok(())
    }

    fn addr(&self) -> NetAddr {
        self.addr.clone()
    }

    async fn peers(&self) -> HashMap<NetAddr, Box<dyn Transport>> {
        self.peers.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() -> Result<()> {
        let mut tr_a = LocalTransport::new("A".into());
        let mut tr_b = LocalTransport::new("B".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_b.connect(Box::new(tr_a.clone())).await?;

        assert_eq!(tr_a.peers().await[&tr_b.addr()].addr(), tr_b.addr());
        assert_eq!(tr_b.peers().await[&tr_a.addr()].addr(), tr_a.addr());

        Ok(())
    }

    #[tokio::test]
    async fn test_send_message() -> Result<()> {
        let mut tr_a = LocalTransport::new("A".into());
        let mut tr_b = LocalTransport::new("B".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_b.connect(Box::new(tr_a.clone())).await?;

        let msg = b"hello world!".to_vec();
        tr_a.send_message(&tr_b.addr(), msg.clone()).await?;

        let rpc = tr_b.recv().await.unwrap();
        assert_eq!(rpc.from, tr_a.addr());
        assert_eq!(rpc.payload, msg);

        Ok(())
    }

    #[tokio::test]
    async fn test_broadcast() -> Result<()> {
        let mut tr_a = LocalTransport::new("A".into());
        let tr_b = LocalTransport::new("B".into());
        let tr_c = LocalTransport::new("C".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_a.connect(Box::new(tr_c.clone())).await?;

        let msg = b"foo".to_vec();
        tr_a.broadcast(msg.clone()).await?;

        let rpc_b = tr_b.recv().await.unwrap();
        assert_eq!(rpc_b.payload, msg);

        let rpc_c = tr_c.recv().await.unwrap();
        assert_eq!(rpc_c.payload, msg);

        Ok(())
    }
}
