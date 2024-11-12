mod army;
mod base;
mod bot;

use bot::TerranBot;
use clap::Parser;
use rust_sc2::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// If the game should run in realtime or not
    #[arg(short, long)]
    realtime: bool,
}

fn main() -> SC2Result<()> {
    let args = Args::parse();
    let mut bot = TerranBot::default();
    run_vs_computer(
        &mut bot,
        Computer::new(Race::Random, Difficulty::Harder, None),
        "PortAleksanderLE",
        LaunchOptions {
            realtime: args.realtime,
            ..Default::default()
        },
    )
}
