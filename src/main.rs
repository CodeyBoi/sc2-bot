mod army;
mod base;
mod bot;

use bot::TerranBot;
use rust_sc2::prelude::*;

fn main() -> SC2Result<()> {
    let mut bot = TerranBot::default();
    run_vs_computer(
        &mut bot,
        Computer::new(Race::Random, Difficulty::Harder, None),
        "PortAleksanderLE",
        LaunchOptions {
            realtime: true,
            ..Default::default()
        },
    )
}
