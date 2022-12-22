use crate::address::MinecraftAddress;
use clap::Parser;

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(help = "The ip address of the server to connect to")]
    pub server: MinecraftAddress,
    #[clap(help = "The amount of bots to spawn")]
    pub count: usize,
    #[clap(
        short,
        long,
        default_value = "5",
        help = "The time in ms to wait between connecting each bot"
    )]
    pub delay: usize,
    #[clap(
        short,
        long,
        default_value = "160",
        help = "The radius (square) the bots will stay within"
    )]
    pub radius: usize,
    #[clap(
        short = 'p',
        long,
        default_value = "0",
        help = "The number of threads to create"
    )]
    pub threads: usize,
    #[clap(
        short,
        long,
        default_value = "500",
        help = "Time in ms between ui updates"
    )]
    pub ui_update_rate: u64,
    #[clap(
        short,
        long,
        default_value = "15",
        help = "Time in ms between bot connections"
    )]
    pub join_rate: u64,
    #[clap(long, default_value = "50", help = "Time in ms between ticks")]
    pub tick_rate: u64,
    #[clap(
        long = "no-ui",
        action = clap::ArgAction::SetFalse,
        help = "Disables terminal ui"
    )]
    pub ui: bool,
    #[clap(long, help = "The protocol id presented to the server")]
    pub proto_id: Option<u32>,
}
