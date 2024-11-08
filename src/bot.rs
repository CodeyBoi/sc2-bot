use rust_sc2::prelude::*;

#[bot]
#[derive(Default)]
pub(crate) struct WorkerRush;
impl Player for WorkerRush {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings::new(Race::Protoss)
    }
    fn on_start(&mut self) -> SC2Result<()> {
        for worker in &self.units.my.workers {
            worker.attack(Target::Pos(self.enemy_start), false);
        }
        Ok(())
    }
}

type Tag = u64;

#[bot]
#[derive(Default)]
pub(crate) struct TerranBot {
    pub(crate) queued_structures: Vec<UnitTypeId>,
    pub(crate) queued_units: Vec<(UnitTypeId, Tag)>,
}

impl Player for TerranBot {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings {
            race: Race::Terran,
            ..Default::default()
        }
    }

    fn on_start(&mut self) -> SC2Result<()> {
        self.set_game_step(4);
        Ok(())
    }

    fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
        self.reset_state();
        self.train_workers();
        self.process_townhalls();
        self.train_army();
        self.build_supply();
        self.build_structures();
        self.move_workers();
        self.move_army();
        Ok(())
    }

    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        Ok(())
    }

    fn on_event(&mut self, _event: Event) -> SC2Result<()> {
        let time = format!(
            "{:0>2}:{:0>2} ",
            self.time as usize / 60,
            self.time as usize % 60
        );
        match _event {
            Event::UnitDestroyed(tag, Some(alliance)) => {
                if let Some(unit) = self.units.all.get(tag) {
                    print!("{}", time);
                    if alliance.is_mine() {
                        let count = self.counter().all().count(unit.type_id());
                        println!("{:?} destroyed! (current count: {})", unit.type_id(), count);
                    } else if alliance.is_enemy() {
                        println!("Enemy {:?} destroyed!", unit.type_id());
                    }
                }
            }
            Event::UnitCreated(tag) => {
                if let Some(unit) = self.units.all.get(tag).cloned() {
                    let count = self.counter().all().count(unit.type_id());
                    if let Some(idx) = self
                        .queued_units
                        .iter()
                        .position(|&u| u.0 == unit.type_id())
                    {
                        self.queued_units.swap_remove(idx);
                    }
                    print!("{}", time);
                    println!("{:?} created (current count: {})", unit.type_id(), count);
                }
            }
            Event::ConstructionStarted(tag) => {
                if let Some(unit) = self.units.all.get(tag) {
                    print!("{}", time);
                    println!("Construction of {:?} started", unit.type_id());
                }
            }
            Event::ConstructionComplete(tag) => {
                if let Some(unit) = self.units.all.get(tag).cloned() {
                    let count = self.counter().all().count(unit.type_id());
                    if let Some(idx) = self
                        .queued_structures
                        .iter()
                        .position(|&s| s == unit.type_id())
                    {
                        self.queued_structures.swap_remove(idx);
                    }
                    print!("{}", time);
                    println!(
                        "Construction of {:?} finished! (current count: {})",
                        unit.type_id(),
                        count
                    );
                }
            }
            Event::RandomRaceDetected(race) => {
                print!("{}", time);
                println!("Detected random opponent to be {:?}", race);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum BotError {
    NoSuitableLocation(UnitTypeId, Point2),
    CannotAfford(UnitTypeId),
    NoSuitableWorker,
    UnfulfilledTechRequirement(UnitTypeId),
}

impl TerranBot {
    fn reset_state(&mut self) {
        self.queued_structures = Vec::new();
        self.queued_units = Vec::new();
    }
}
