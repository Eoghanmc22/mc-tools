use std::sync::atomic::AtomicU64;

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

#[derive(Debug)]
pub struct Worker {
    pub packets_tx: AtomicU64,
    pub packets_rx: AtomicU64,
    pub bytes_tx: AtomicU64,
    pub bytes_rx: AtomicU64,

    pub bot_bound: (Sender<BotMessage>, Receiver<BotMessage>),
    pub console_bound: (Sender<ConsoleMessage>, Receiver<ConsoleMessage>),

    pub waker: Option<Waker>,
}
