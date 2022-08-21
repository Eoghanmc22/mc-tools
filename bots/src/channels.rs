use crossbeam::channel::{Sender, Receiver};

pub enum ConsoleMessage {
    BotConnected,
    BotDisconnected
}

pub enum BotMessage {
    ConnectBot()
}
