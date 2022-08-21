use std::thread;
use anyhow::Context;
use crate::args::Args;
use clap::Parser;

mod args;
mod address;
mod console;
mod bot;
mod channels;
mod player;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let threads = if args.threads == 0 {
        thread::available_parallelism().context("Could not retrieve parallelism. Try specifying a thread count with -p THREADS")?.get() / 2
    } else {
        args.threads
    };
    
    

    Ok(())
}
