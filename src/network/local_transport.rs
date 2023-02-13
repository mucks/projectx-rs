/*
LocalTransport is used for testing the network layer.
It is a simple in-memory transport that does not actually send messages over the network.
*/

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;

use super::{
    server::{new_channel, Channel},
    transport::{NetAddr, Rpc, Transport},
};

#[derive(Debug, Clone)]
pub struct LocalTransport {
    addr: NetAddr,
    consume_channel: Channel<Rpc>,
    peers: HashMap<NetAddr, Box<dyn Transport>>,
}

impl LocalTransport {
    pub fn new(addr: NetAddr) -> Self {
        Self {
            addr,
            consume_channel: new_channel(1024),
            peers: HashMap::new(),
        }
    }
}

#[async_trait]
impl Transport for LocalTransport {
    fn consume(&self) -> Channel<Rpc> {
        self.consume_channel.clone()
    }

    async fn recv(&self) -> Option<Rpc> {
        self.consume_channel.1.lock().await.recv().await
    }

    async fn connect(&mut self, tr: Box<dyn Transport>) -> Result<()> {
        self.peers.insert(tr.addr(), tr);
        Ok(())
    }

    async fn send_message(&self, to: NetAddr, payload: Vec<u8>) -> Result<()> {
        let peer =
            self.peers
                .get(&to)
                .ok_or(anyhow!("{} could not send message to {}", self.addr, to))?;

        peer.consume()
            .0
            .send(Rpc {
                from: self.addr.clone(),
                payload,
            })
            .await?;

        Ok(())
    }

    fn addr(&self) -> NetAddr {
        self.addr.clone()
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

        assert_eq!(tr_a.peers[&tr_b.addr()].addr(), tr_b.addr());
        assert_eq!(tr_b.peers[&tr_a.addr()].addr(), tr_a.addr());

        Ok(())
    }

    #[tokio::test]
    async fn test_send_message() -> Result<()> {
        let mut tr_a = LocalTransport::new("A".into());
        let mut tr_b = LocalTransport::new("B".into());

        tr_a.connect(Box::new(tr_b.clone())).await?;
        tr_b.connect(Box::new(tr_a.clone())).await?;

        let msg = b"hello world!".to_vec();
        tr_a.send_message(tr_b.addr(), msg.clone()).await?;

        let rpc = tr_b.recv().await.unwrap();
        assert_eq!(rpc.from, tr_a.addr());
        assert_eq!(rpc.payload, msg.to_vec());

        Ok(())
    }
}
