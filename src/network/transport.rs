/*
In this file we define the Transport trait. This trait is used to abstract away the underlying network layer.
The transport layer is responsible for sending and receiving messages. It is also responsible for connecting to other peers.
*/

use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;

use super::{server::Channel, RPC};

pub type NetAddr = String;

#[async_trait]
pub trait Transport: TransportClone + Send + Sync + Debug {
    fn consume(&self) -> Channel<RPC>;
    async fn recv(&self) -> Option<RPC>;
    async fn connect(&mut self, tr: Box<dyn Transport>) -> Result<()>;
    async fn send_message(&self, to: &NetAddr, payload: Vec<u8>) -> Result<()>;
    async fn broadcast(&self, payload: Vec<u8>) -> Result<()>;
    fn addr(&self) -> NetAddr;
}

pub trait TransportClone {
    fn clone_box(&self) -> Box<dyn Transport>;
}

impl<T> TransportClone for T
where
    T: 'static + Transport + Clone,
{
    fn clone_box(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Transport> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
