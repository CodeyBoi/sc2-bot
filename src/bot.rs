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
#[derive(Default)]
pub(crate) struct TerranBot;

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

    fn on_step(&mut self, iteration: usize) -> SC2Result<()> {
        if iteration % 10 == 0 {
            println!(
                "Tick {}: Minerals: {}, Gas: {}",
                iteration, self.minerals, self.vespene
            );
        }
        self.train_workers();
        self.build_townhall();
        self.train_army();
        self.build_supply();
        self.build_army_buildings();
        Ok(())
    }

    fn on_end(&self, _result: GameResult) -> SC2Result<()> {
        Ok(())
    }

    fn on_event(&mut self, _event: Event) -> SC2Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
enum BotError {
    NoSuitableLocation(UnitTypeId, Point2),
    CannotAfford(UnitTypeId),
    NoSuitableWorker,
}

impl TerranBot {
    fn ideal_workers(&self) -> usize {
        self.units
            .my
            .townhalls
            .iter()
            .map(|t| {
                t.ideal_harvesters()
                    .expect("unit in townhalls iterator should have ideal_harvesters")
                    as usize
            })
            .sum::<usize>()
            + self
                .units
                .my
                .gas_buildings
                .iter()
                .map(|g| {
                    g.ideal_harvesters()
                        .expect("unit in gas_buildings iterator should have ideal_harvesters")
                        as usize
                })
                .sum::<usize>()
    }

    fn train_workers(&mut self) {
        if !self.can_afford(UnitTypeId::SCV, false) {
            return;
        }

        let target_amount = self.ideal_workers();
        let current_amount = self.units.my.workers.len()
            + self
                .units
                .my
                .townhalls
                .iter()
                .filter(|t| t.is_active())
                .count();

        if current_amount >= target_amount {
            return;
        }

        let mut units_in_progress = 0;
        for townhall in self
            .units
            .my
            .townhalls
            .iter()
            .idle()
            .take(target_amount - current_amount)
        {
            townhall.train(UnitTypeId::SCV, false);
            units_in_progress += 1;
            println!(
                "Training SCV (current: {}, target: {})",
                current_amount + units_in_progress,
                target_amount
            );
        }

        for _ in 0..units_in_progress {
            self.subtract_resources(UnitTypeId::SCV, true);
        }
    }

    fn get_closest_free_worker(&self, location: Point2) -> Option<&Unit> {
        self.units
            .my
            .workers
            .iter()
            .filter(|w| !w.is_constructing())
            .closest(location)
    }

    fn build_townhall(&mut self) {
        if !self.can_afford(UnitTypeId::CommandCenter, false)
            || self.counter().ordered().count(UnitTypeId::CommandCenter) != 0
        {
            return;
        }

        if let Some(expansion) = self.get_expansion() {
            if let Some(builder) = self.get_closest_free_worker(expansion.loc) {
                builder.build(UnitTypeId::CommandCenter, expansion.loc, false);
                println!("Building Command Center at {:?}", expansion.loc);
                self.subtract_resources(UnitTypeId::CommandCenter, false);
            } else {
                println!("Couldn't find suitable worker to build expansion");
            }
        } else {
            println!("Couldn't get suitable expansion");
        }
    }

    fn build_supply(&mut self) {
        if self.supply_left < 3
            && self.counter().ordered().count(UnitTypeId::SupplyDepot) == 0
            && self.can_afford(UnitTypeId::SupplyDepot, false)
        {
            self.build_in_base(UnitTypeId::SupplyDepot).unwrap();
        }
    }

    fn build_in_base(&mut self, building: UnitTypeId) -> Result<(), BotError> {
        if !self.can_afford(building, false) {
            return Err(BotError::CannotAfford(building));
        }
        let main_base = self.start_location.towards(self.game_info.map_center, 8.0);
        self.build_close_to(building, main_base)
    }

    fn build_close_to(&mut self, building: UnitTypeId, location: Point2) -> Result<(), BotError> {
        let placement = self
            .find_placement(
                building,
                location,
                PlacementOptions {
                    step: 4,
                    ..Default::default()
                },
            )
            .ok_or(BotError::NoSuitableLocation(building, location))?;

        let builder = self
            .get_closest_free_worker(placement)
            .ok_or(BotError::NoSuitableWorker)?;

        println!("Building {:?} at {:?}", building, placement);
        builder.build(building, placement, false);
        self.subtract_resources(building, false);
        Ok(())
    }

    fn train_army(&mut self) {
        use UnitTypeId as UID;
        let buildings: Vec<_> = self
            .units
            .my
            .structures
            .iter()
            .idle()
            .of_types(&vec![UID::Barracks, UID::Factory, UID::Starport])
            .cloned()
            .collect();
        for building in &buildings {
            match building.type_id() {
                UID::Barracks => {
                    self.train_army_unit(building, UID::Marine);
                    if building.has_reactor() {
                        self.train_army_unit(building, UID::Marine);
                    }
                }
                UID::Factory => self.train_army_unit(building, UID::Hellion),
                UID::Starport => self.train_army_unit(building, UID::Medivac),
                _ => unreachable!("No other buildings should have passed the iterator filter"),
            }
        }
    }

    fn train_army_unit(&mut self, building: &Unit, unit: UnitTypeId) {
        if self.can_afford(unit, true) {
            building.train(unit, true);
            self.subtract_resources(unit, true);
        }
    }

    fn build_army_buildings(&mut self) {
        use UnitTypeId as UID;
        for building in [UID::Barracks, UID::Factory, UID::Starport] {
            if self.counter().all().count(building) == 0 && self.build_in_base(building).is_ok() {
            } else {
            }
        }
    }
}
