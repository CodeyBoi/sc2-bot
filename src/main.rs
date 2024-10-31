mod bot;

use bot::TerranBot;
use rust_sc2::prelude::*;

fn main() -> SC2Result<()> {
    let mut bot = TerranBot::default();
    run_vs_computer(
        &mut bot,
        Computer::new(Race::Random, Difficulty::Medium, None),
        "CyberForestLE",
        LaunchOptions {
            realtime: true,
            ..Default::default()
        },
    )
}
