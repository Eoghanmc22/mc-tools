use crate::{
    args::Args,
    threading::{BotMessage, Worker},
};
use anyhow::Context;
use clap::Parser;
use crossbeam::channel::unbounded;
use log::{info, LevelFilter};
use std::{
    sync::{atomic::AtomicU64, Arc},
    thread,
};

mod address;
mod args;
mod bot;
mod console;
mod player;
mod threading;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

// motd, implement cached reading (hash first few bytes and lookup in some kind
// of hash map), implement tps monetering, make ui more colerful, implement bots.rs
fn main() -> anyhow::Result<()> {
    tui_logger::init_logger(LevelFilter::Info).unwrap();

    info!("Starting {} - {}", NAME, VERSION);

    let args = Args::parse();

    let threads = if args.threads == 0 {
        thread::available_parallelism()
            .context(
                "Could not retrieve parallelism. Try specifying a thread count with -p THREADS",
            )?
            .get()
            / 2
    } else {
        args.threads
    };

    info!("Using {} threads", threads);

    let mut workers = Vec::new();

    thread::scope(|s| {
        for _ in 0..threads {
            let worker = Worker {
                packets_tx: Arc::new(AtomicU64::new(0)),
                packets_rx: Arc::new(AtomicU64::new(0)),
                bytes_tx: Arc::new(AtomicU64::new(0)),
                bytes_rx: Arc::new(AtomicU64::new(0)),
                bot_bound: unbounded(),
                console_bound: unbounded(),
            };

            {
                let worker = worker.clone();
                s.spawn(move || {
                    // todo!("Spawn workers");
                });
            }

            workers.push(worker);
        }

        s.spawn(|| {
            console::start(args.clone(), workers.clone()).expect("Run console");
        });

        {
            let workers = workers.clone();
            s.spawn(move || {
                for i in 0..args.count {
                    let worker = i % workers.len();
                    let worker = &workers[worker];

                    let name = format!("Bot{}", i);
                    worker
                        .bot_bound
                        .0
                        .send(BotMessage::ConnectBot(name))
                        .expect("Send msg");
                }
            });
        }
    });

    Ok(())
}
