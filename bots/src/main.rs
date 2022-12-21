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
    time::{Duration, Instant},
};

mod address;
mod args;
mod bot;
mod console;
mod player;
mod threading;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

const TICK_DURATION: Duration = Duration::from_millis(50);

const CONEOLE_UI: bool = true;

// A way to stop everything, motd, implement cached reading (hash first few bytes and lookup in some kind
// of hash map), implement tps monetering, make ui more colerful, use write/read vectored, ipv6
// support, steal graphs from bottom, batch pachet sending, optimize
fn main() -> anyhow::Result<()> {
    if CONEOLE_UI {
        tui_logger::init_logger(LevelFilter::Info).unwrap();
    } else {
        env_logger::builder().filter_level(LevelFilter::Info).init();
    }

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
        // Worker threads
        for _ in 0..threads {
            let mut worker = Worker {
                packets_tx: Arc::new(AtomicU64::new(0)),
                packets_rx: Arc::new(AtomicU64::new(0)),
                bytes_tx: Arc::new(AtomicU64::new(0)),
                bytes_rx: Arc::new(AtomicU64::new(0)),
                bot_bound: unbounded(),
                console_bound: unbounded(),
                waker: None,
            };

            let bot_context = bot::setup_bot(&mut worker).expect("Setup bot");

            {
                let worker = worker.clone();
                s.spawn(|| {
                    bot::start(bot_context, &args, worker).unwrap();
                });
            }

            workers.push(worker);
        }

        // Console ui
        if CONEOLE_UI {
            s.spawn(|| {
                console::start(&args, &workers).expect("Run console");
            });
        }

        // Bot spawner
        s.spawn(|| {
            for i in 0..args.count {
                let worker = i % workers.len();
                let worker = &workers[worker];

                let name = format!("Bot{}", i);
                worker
                    .bot_bound
                    .0
                    .send(BotMessage::ConnectBot(name))
                    .expect("Send msg");

                worker.waker.as_ref().unwrap().wake().unwrap();

                thread::sleep(Duration::from_millis(args.bot_join_rate));
            }
        });

        // Tick schedualer
        s.spawn(|| {
            let mut tick = Instant::now();
            loop {
                for worker in &workers {
                    worker.bot_bound.0.send(BotMessage::Tick).expect("Send msg");
                    worker.waker.as_ref().unwrap().wake().unwrap();
                }

                tick += TICK_DURATION;
                let delay = tick - Instant::now();
                thread::sleep(delay);
            }
        });
    });

    Ok(())
}
