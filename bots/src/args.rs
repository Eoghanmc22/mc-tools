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
        short = 'd',
        default_value = "5",
        help = "The time in ms to wait between connecting each bot"
    )]
    pub delay: usize,
    #[clap(
        short = 'r',
        default_value = "160",
        help = "The radius (square) the bots will stay within"
    )]
    pub radius: usize,
    #[clap(
        short = 's',
        default_value = "0.2",
        help = "The distance each bot will move every tick"
    )]
    pub speed: f64,
    #[clap(
        short = 'p',
        default_value = "0",
        help = "The number of threads to create"
    )]
    pub threads: usize,
    #[clap(
        short = 'n',
        default_value = "200",
        help = "Time in ms between ui updates"
    )]
    pub ui_update_rate: u64,
    #[clap(
        short = 'j',
        default_value = "15",
        help = "Time in ms between bot connections"
    )]
    pub bot_join_rate: u64,
}
