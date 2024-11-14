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
    pub(crate) upgrade_prio_index: usize,
}

impl TerranBot {
    pub(crate) fn log(&mut self, msg: &str) {
        self.chat_ally(msg);
        println!("{}", msg);
    }
}

impl Player for TerranBot {
    fn get_player_settings(&self) -> PlayerSettings {
        PlayerSettings {
            race: Race::Terran,
            ..Default::default()
        }
    }

    fn on_start(&mut self) -> SC2Result<()> {
        for worker in &self.units.my.workers {
            worker.stop(false);
        }
        for townhall in &self.units.my.townhalls {
            townhall.command(
                AbilityId::RallyCommandCenter,
                Target::Tag(townhall.tag()),
                false,
            );
        }
        Ok(())
    }

    fn on_step(&mut self, iteration: usize) -> SC2Result<()> {
        self.process_base(iteration);
        self.process_army(iteration);
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
                        let count = self.counter().alias().all().count(unit.type_id());
                        println!("{:?} destroyed! (count: {})", unit.type_id(), count);
                    } else if alliance.is_enemy() {
                        println!("Enemy {:?} destroyed!", unit.type_id());
                    }
                }
            }
            Event::UnitCreated(tag) => {
                if let Some(unit) = self.units.all.get(tag).cloned() {
                    if unit.type_id() != self.race_values.worker {
                        let count = self.counter().alias().all().count(unit.type_id());
                        // print!("{}", time);
                        // println!("{:?} created (count: {})", unit.type_id(), count);
                    }
                }
            }
            // Event::ConstructionStarted(tag) => {
            //     if let Some(unit) = self.units.all.get(tag) {
            //         print!("{}", time);
            //         println!("{:?}: construction started", unit.type_id());
            //     }
            // }
            Event::ConstructionComplete(tag) => {
                if let Some(unit) = self.units.all.get(tag).cloned() {
                    let count = self.counter().alias().all().count(unit.type_id());
                    // print!("{}", time);
                    // println!(
                    //     "{:?} finished! (count: {}, supply: {}/{})",
                    //     unit.type_id(),
                    //     count,
                    //     self.supply_used,
                    //     self.supply_cap
                    // );
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
pub(crate) enum BuildError {
    NoSuitableLocation(UnitTypeId, Point2),
    CannotAfford(UnitTypeId),
    NoSuitableWorker,
    UnfulfilledTechRequirement(UnitTypeId),
    EndOfBuildOrder,
    NoProducer(UnitTypeId),
    InvalidArgument(UnitTypeId),
    CannotAffordUpgrade(UpgradeId),
    NoResearcher(UpgradeId),
}
