use crate::{
    args::Args,
    threading::{BotMessage, Worker},
};
use anyhow::Context;
use clap::Parser;
use crossbeam::channel::unbounded;
use log::{info, LevelFilter};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

mod address;
mod args;
mod bot;
mod console;
pub mod context;
mod player;
mod threading;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static STOP_THE_WORLD: AtomicBool = AtomicBool::new(false);

// motd, implement cached reading (hash first few bytes and lookup in some kind
// of hash map), make ui more colerful, use write/read vectored,
// steal graphs from bottom, batch pachet sending, Write docs, Write readme, make log widget update
// faster
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if !args.no_ui {
        tui_logger::init_logger(LevelFilter::Info).unwrap();
    } else {
        env_logger::builder().filter_level(LevelFilter::Info).init();
    }

    ctrlc::set_handler(|| STOP_THE_WORLD.store(true, Ordering::SeqCst))
        .context("Set ctrl-c handler")?;

    info!("Starting {} - {}", NAME, VERSION);

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
                packets_tx: AtomicU64::new(0),
                packets_rx: AtomicU64::new(0),
                bytes_tx: AtomicU64::new(0),
                bytes_rx: AtomicU64::new(0),
                bot_bound: unbounded(),
                console_bound: unbounded(),
                waker: None,
            };

            let bot_context = bot::setup_bot(&mut worker).expect("Setup bot");

            let worker = Arc::new(worker);

            {
                let worker = worker.clone();
                s.spawn(|| {
                    bot::start(bot_context, &args, worker).unwrap();
                });
            }

            workers.push(worker);
        }

        // Console ui
        if !args.no_ui {
            s.spawn(|| {
                console::start(&args, &workers).expect("Run console");
            });
        }

        // Bot spawner
        s.spawn(|| {
            for i in 0..args.count {
                if STOP_THE_WORLD.load(Ordering::SeqCst) {
                    break;
                }

                let worker = i % workers.len();
                let worker = &workers[worker];

                let name = format!("Bot{i}");
                worker
                    .bot_bound
                    .0
                    .send(BotMessage::ConnectBot(name))
                    .expect("Send msg");

                worker.waker.as_ref().unwrap().wake().unwrap();

                thread::sleep(Duration::from_millis(args.join_rate));
            }
        });

        // Tick schedualer
        s.spawn(|| {
            let mut tick = Instant::now();
            let tick_duration = Duration::from_millis(args.tick_rate);

            loop {
                if STOP_THE_WORLD.load(Ordering::SeqCst) {
                    for worker in &workers {
                        worker.bot_bound.0.send(BotMessage::Stop).expect("Send msg");
                        worker.waker.as_ref().unwrap().wake().unwrap();
                    }

                    break;
                }

                for worker in &workers {
                    worker.bot_bound.0.send(BotMessage::Tick).expect("Send msg");
                    worker.waker.as_ref().unwrap().wake().unwrap();
                }

                tick += tick_duration;
                let delay = tick - Instant::now();
                thread::sleep(delay);
            }
        });
    });

    println!("Exiting!");

    Ok(())
}
