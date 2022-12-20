use std::{fmt::Display, sync::atomic::Ordering, time::Duration};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use crate::threading::{ConsoleMessage, Worker};

#[derive(Default, Debug, Clone)]
pub struct App<'a> {
    pub bots: u64,
    pub bot_count_data: Vec<(f64, f64)>,
    pub bandwidth_in_data: Vec<(f64, f64)>,
    pub bandwidth_out_data: Vec<(f64, f64)>,

    pub bytes_tx: Bytes,
    pub bytes_rx: Bytes,

    pub packets_tx: u64,
    pub packets_rx: u64,

    pub tps: Vec<u64>,

    pub should_quit: bool,
    pub show_help: bool,

    pub server: String,

    workers: &'a [Worker],
    tick: u64,
}

impl<'a> App<'a> {
    pub fn new(workers: &'a [Worker], server: String) -> Self {
        Self {
            workers,
            server,
            ..Default::default()
        }
    }

    pub fn on_mouse(&mut self, mouse: MouseEvent) {
        // todo!()
    }

    pub fn on_paste(&mut self, paste: String) {
        // todo!()
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Char('?') => self.show_help = !self.show_help,
            _ => {}
        }
    }

    pub fn on_tick(&mut self, tick_time: Duration) {
        let bytes_tx_last = self.bytes_tx.0;
        let bytes_rx_last = self.bytes_rx.0;
        let mut bytes_tx_next = 0;
        let mut bytes_rx_next = 0;
        let mut packets_tx_next = 0;
        let mut packets_rx_next = 0;

        for worker in self.workers {
            for message in worker.console_bound.1.try_iter() {
                match message {
                    ConsoleMessage::BotConnected => {
                        self.bots += 1;
                    }
                    ConsoleMessage::BotDisconnected => {
                        self.bots -= 1;
                    }
                }
            }

            bytes_tx_next += worker.bytes_tx.load(Ordering::Relaxed);
            bytes_rx_next += worker.bytes_rx.load(Ordering::Relaxed);
            packets_tx_next += worker.packets_tx.load(Ordering::Relaxed);
            packets_rx_next += worker.packets_rx.load(Ordering::Relaxed);
        }

        let bandwidth_tx = (bytes_tx_next - bytes_tx_last) as f64 / tick_time.as_secs_f64();
        let bandwidth_rx = (bytes_rx_next - bytes_rx_last) as f64 / tick_time.as_secs_f64();

        self.bot_count_data
            .push((self.tick as f64, self.bots as f64));
        self.bandwidth_in_data
            .push((self.tick as f64, bandwidth_rx as f64));
        self.bandwidth_out_data
            .push((self.tick as f64, bandwidth_tx as f64));

        let lower = self.bot_count_data.len().max(100) - 100;
        self.bot_count_data.drain(0..lower);
        self.bandwidth_in_data.drain(0..lower);
        self.bandwidth_out_data.drain(0..lower);

        self.bytes_tx = Bytes(bytes_tx_next);
        self.bytes_rx = Bytes(bytes_rx_next);

        self.packets_tx = packets_tx_next;
        self.packets_rx = packets_rx_next;

        // TODO tps

        self.tick += 1;
    }
}

#[derive(Default, Debug, Clone)]
pub struct Bytes(pub u64);
impl Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = number_prefix::NumberPrefix::binary(self.0 as f64 * 8.0);
        match prefix {
            number_prefix::NumberPrefix::Standalone(num) => {
                write!(f, "{:5.0} bits", num)
            }
            number_prefix::NumberPrefix::Prefixed(prefix, num) => {
                write!(f, "{:7.2} {:2}b", num, prefix)
            }
        }
    }
}
