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

    /// Difficulty level of the AI
    #[arg(short, long)]
    difficulty: Option<u8>,
}

fn main() -> SC2Result<()> {
    let args = Args::parse();

    let difficulty_level = if let Some(lvl) = args.difficulty {
        match lvl {
            0 => Difficulty::VeryEasy,
            1 => Difficulty::Easy,
            2 => Difficulty::Medium,
            3 => Difficulty::MediumHard,
            4 => Difficulty::Hard,
            5 => Difficulty::Harder,
            6 => Difficulty::VeryHard,
            7 => Difficulty::CheatVision,
            8 => Difficulty::CheatMoney,
            9 => Difficulty::CheatInsane,
            _ => Difficulty::CheatInsane,
        }
    } else {
        Difficulty::Medium
    };

    let mut bot = TerranBot::default();
    run_vs_computer(
        &mut bot,
        Computer::new(Race::Random, difficulty_level, None),
        "PortAleksanderLE",
        LaunchOptions {
            realtime: args.realtime,
            ..Default::default()
        },
    )
}
