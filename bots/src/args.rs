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
        default_value = "NAN",
        help = "The radius (square) the bots will stay within"
    )]
    pub radius: f64,
    #[clap(
        short = 'p',
        long,
        default_value = "0",
        help = "The number of threads to create"
    )]
    pub threads: usize,
    #[clap(long, default_value = "500", help = "Time in ms between ui updates")]
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
    #[clap(long, help = "Disables bot actions")]
    pub no_action: bool,
    #[clap(long, help = "Disables bot movement")]
    pub no_move: bool,
    #[clap(long, help = "Disables terminal ui")]
    pub no_ui: bool,
    #[clap(long, help = "The protocol id presented to the server")]
    pub proto_id: Option<u32>,
    #[clap(
        long,
        help = "The file to take chat messages from. Messages are seperated by new lines"
    )]
    pub message_file: Option<String>,
}
