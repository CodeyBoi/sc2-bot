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

#[bot]
pub(crate) struct TerranBot {
    build_queue: Vec<UnitTypeId>,
    build_queue_index: usize,
}

impl TerranBot {
    const BUILD_ORDER: &[UnitTypeId] = &[
        UnitTypeId::SupplyDepot,
        UnitTypeId::Barracks,
        UnitTypeId::Refinery,
        UnitTypeId::Refinery,
        UnitTypeId::OrbitalCommand,
        UnitTypeId::Reaper,
        UnitTypeId::SupplyDepot,
    ];
}

impl Default for TerranBot {
    fn default() -> Self {
        Self {
            _bot: Default::default(),
            build_queue: Self::BUILD_ORDER.to_vec(),
            build_queue_index: 0,
        }
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
        Ok(())
    }

    fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
        let next_structure = self.build_queue[self.build_queue_index];
        if self.can_afford(next_structure, true) {
            // See if we're able to build the next item in our build queue
            if let Some(location) = self.find_placement(
                next_structure,
                self.start_location,
                PlacementOptions {
                    step: 4,
                    max_distance: 100,
                    ..Default::default()
                },
            ) {
                // Find a suitable worker to build the item
                if let Some(builder) = self
                    .units
                    .my
                    .workers
                    .iter()
                    .filter(|w| !w.is_constructing())
                    .closest(location)
                {
                    builder.build(next_structure, location, false);
                    self.subtract_resources(next_structure, true);
                    self.build_queue_index += 1;
                }
            }
        }
        Ok(())
    }

    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        Ok(())
    }

    fn on_event(&mut self, _event: Event) -> SC2Result<()> {
        Ok(())
    }
}
