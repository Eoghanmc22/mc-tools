use crate::address::MinecraftAddress;
use clap::{Parser, ValueEnum};

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(help = "The ip address of the server to connect to")]
    pub server: MinecraftAddress,
    #[arg(help = "The amount of bots to spawn")]
    pub count: usize,
    #[arg(
        short,
        long,
        default_value_t = f64::NAN,
        help = "The radius (square) the bots will stay within"
    )]
    pub radius: f64,
    #[arg(
        short = 'p',
        long,
        default_value_t = 0,
        help = "The number of threads to create"
    )]
    pub threads: usize,
    #[arg(long, default_value_t = 500, help = "Time in ms between ui updates")]
    pub ui_update_rate: u64,
    #[arg(
        short,
        long,
        default_value_t = 15,
        help = "Time in ms between bot connections"
    )]
    pub join_rate: u64,
    #[arg(long, default_value_t = 50, help = "Time in ms between ticks")]
    pub tick_rate: u64,
    #[arg(long, help = "Disables bot actions")]
    pub no_action: bool,
    #[arg(long, help = "Disables bot movement")]
    pub no_move: bool,
    #[arg(long, help = "Disables terminal ui")]
    pub no_ui: bool,
    #[arg(long, help = "Disables sending yaw and pitch angles")]
    pub no_yaw: bool,
    #[arg(long, help = "The protocol id presented to the server")]
    pub proto_id: Option<u32>,
    #[arg(
        long,
        help = "The file to take chat messages from. Messages are seperated by new lines"
    )]
    pub message_file: Option<String>,
    #[arg(long, value_enum, default_value_t = Movement::Biased, help = "The method used to calculate movement updates")]
    pub movement: Movement,
    #[arg(
        long,
        default_value_t = 0.25,
        help = "The chance of sending an action packet"
    )]
    pub action_chance: f64,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Movement {
    Biased,
    Consistant,
    Random,
}
