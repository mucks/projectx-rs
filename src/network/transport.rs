/*
In this file we define the Transport trait. This trait is used to abstract away the underlying network layer.
The transport layer is responsible for sending and receiving messages. It is also responsible for connecting to other peers.
*/

use super::RPC;
use anyhow::Result;
use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tokio::sync::{mpsc, Mutex, RwLock};

// Sender can be passed within threads safely and cloned as many times as needed.
// Receiver needs to be wrapped in a Mutex to be shared across threads and can only be accessed once at a time.
pub type Channel<T> = (mpsc::Sender<T>, Arc<Mutex<mpsc::Receiver<T>>>);

pub fn new_channel<T>(buffer_size: usize) -> Channel<T> {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, Arc::new(Mutex::new(rx)))
}

pub type NetAddr = String;

// Be very careful with rwlock, write can lock the whole program
pub type BTransport = Box<dyn Transport>;

#[async_trait]
pub trait Transport: TransportClone + Send + Sync + Debug {
    fn consume(&self) -> Channel<RPC>;
    async fn recv(&self) -> Option<RPC>;
    async fn connect(&self, tr: Box<dyn Transport>) -> Result<()>;
    async fn send_message(&self, to: &NetAddr, payload: Vec<u8>) -> Result<()>;
    async fn broadcast(&self, payload: Vec<u8>) -> Result<()>;
    async fn peers(&self) -> HashMap<NetAddr, Box<dyn Transport>>;
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
