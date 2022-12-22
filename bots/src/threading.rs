use std::sync::{atomic::AtomicU64, Arc};

use crossbeam::channel::{Receiver, Sender};
use mio::Waker;

pub enum ConsoleMessage {
    BotConnected,
    BotDisconnected,
}

pub enum BotMessage {
    ConnectBot(String),
    Tick,
    Stop,
}

#[derive(Debug, Clone)]
pub struct Worker {
    pub packets_tx: Arc<AtomicU64>,
    pub packets_rx: Arc<AtomicU64>,
    pub bytes_tx: Arc<AtomicU64>,
    pub bytes_rx: Arc<AtomicU64>,

    pub bot_bound: (Sender<BotMessage>, Receiver<BotMessage>),
    pub console_bound: (Sender<ConsoleMessage>, Receiver<ConsoleMessage>),

    pub waker: Option<Arc<Waker>>,
}
