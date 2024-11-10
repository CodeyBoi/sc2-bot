use crate::bot::{BotError, TerranBot};
use rust_sc2::prelude::*;
use UnitTypeId as UID;

const BUILD_ORDER: &[UID] = &[
    UID::Barracks,
    UID::Refinery,
    UID::Reaper,
    UID::OrbitalCommand,
    UID::CommandCenter,
];

impl TerranBot {
    pub(crate) fn process_base(&mut self, iteration: usize) {
        if iteration % 5 == 0 {
            self.train_workers();
        }
        self.process_townhalls();
        self.build_supply();
        if iteration % 5 == 1 {
            self.build_structures();
        }
        self.move_workers();
    }
    fn build_close_to(
        &mut self,
        building: UID,
        location: Point2,
        placement_options: Option<PlacementOptions>,
    ) -> Result<(), BotError> {
        if TECH_REQUIREMENTS
            .get(&building)
            .is_some_and(|&r| self.counter().tech().count(r) == 0)
        {
            return Err(BotError::UnfulfilledTechRequirement(building));
        }
        let placement_options = placement_options.unwrap_or(PlacementOptions {
            step: 4,
            ..Default::default()
        });
        let placement = self
            .find_placement(building, location, placement_options)
            .ok_or(BotError::NoSuitableLocation(building, location))?;

        let builder = self
            .get_closest_free_worker(placement)
            .ok_or(BotError::NoSuitableWorker)?;

        builder.build(building, placement, false);
        self.subtract_resources(building, false);
        Ok(())
    }

    pub(crate) fn build_in_base(&mut self, building: UID) -> Result<(), BotError> {
        if !self.can_afford(building, false) {
            return Err(BotError::CannotAfford(building));
        }
        let main_base = self.start_location.towards(self.game_info.map_center, 8.0);
        self.build_close_to(
            building,
            main_base,
            Some(PlacementOptions {
                step: if building == self.race_values.supply {
                    2
                } else {
                    5
                },
                max_distance: 30,
                ..Default::default()
            }),
        )
    }

    pub(crate) fn build_structures(&mut self) {
        if self.counter().all().count(UID::Barracks) == 0 {
            self.build_in_base(UID::Barracks).unwrap_or_default();
        }

        // Build at least one of each army building
        for building in [UID::Factory, UID::Starport] {
            if self.counter().all().count(building) == 0
                && self
                    .counter()
                    .all()
                    .tech()
                    .count(self.race_values.start_townhall)
                    != 1
            {
                self.build_in_base(building).unwrap_or_default();
            }
        }

        // If we have begun constructing barracks, try to build refinery
        match (
            self.counter().all().count(UID::Barracks),
            self.counter().count(UID::Factory),
            self.counter().all().count(self.race_values.gas),
        ) {
            (1, _, 0) | (_, 1, 1) => {
                for townhall in &self.units.my.townhalls {
                    if let Some(geyser) = self.find_gas_placement(townhall.position()) {
                        if let Some(builder) = self.get_closest_free_worker(geyser.position()) {
                            builder.build_gas(geyser.tag(), false);
                        }
                    }
                }
            }
            _ => {}
        }

        if self.counter().count(UID::Starport) > 0
            && self.minerals > 500
            && self.counter().all().count(UID::Barracks) < 6
        {
            self.build_in_base(UID::Barracks)
                .inspect_err(|e| println!("failed building barracks: {:?}", e))
                .unwrap_or_default();
        }

        // Upgrade barracks
        if self.counter().all().count(UID::Factory) > 0 {
            if let Some(barracks) = self
                .units
                .my
                .structures
                .iter()
                .idle()
                .of_type(UID::Barracks)
                .closest(self.start_location)
            {
                barracks.train(UID::BarracksReactor, false);
                self.subtract_resources(UID::BarracksReactor, false);
            }
        }

        if self
            .units
            .my
            .structures
            .iter()
            .of_type(UID::Barracks)
            .filter(|b| b.has_reactor())
            .count()
            >= 2
            && self
                .units
                .my
                .structures
                .iter()
                .of_type(UID::Barracks)
                .filter(|b| b.has_techlab())
                .count()
                < 1
        {
            if let Some(barracks) = self
                .units
                .my
                .structures
                .iter()
                .idle()
                .of_type(UID::Barracks)
                .closest(self.start_location)
            {
                barracks.train(UID::BarracksTechLab, false);
                self.subtract_resources(UID::BarracksTechLab, false);
            }
        }

        // // Upgrade factory
        // if let Some(factory) = self
        //     .units
        //     .my
        //     .structures
        //     .of_type(UID::Factory)
        //     .closest(self.start_location)
        // {
        //     factory.train(UID::FactoryTechLab, true);
        //     self.subtract_resources(UID::FactoryTechLab, false);
        // }

        // Build engineering bay
        if self.counter().all().count(UID::Starport) > 0
            && self.counter().all().count(UID::EngineeringBay) < 2
        {
            self.build_in_base(UID::EngineeringBay).unwrap_or_default();
        }

        for engineering_bay in self
            .units
            .my
            .structures
            .iter()
            .idle()
            .of_type(UID::EngineeringBay)
        {
            for upgrade in [
                AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel1,
                AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel2,
                AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel3,
                AbilityId::EngineeringBayResearchTerranInfantryArmorLevel1,
                AbilityId::EngineeringBayResearchTerranInfantryArmorLevel2,
                AbilityId::EngineeringBayResearchTerranInfantryArmorLevel1,
            ] {
                engineering_bay.use_ability(upgrade, true);
            }
        }

        if self.counter().count(UID::EngineeringBay) > 0
            && self.counter().all().count(UID::Armory) == 0
        {
            self.build_in_base(UID::Armory).unwrap_or_default();
        }
    }

    fn build_supply(&mut self) {
        // Build supply if none is being built and we have less than 5 left
        if self.supply_left < 5
            && self.counter().ordered().count(self.race_values.supply) == 0
            && self.can_afford(self.race_values.supply, false)
        {
            self.build_in_base(self.race_values.supply)
                .unwrap_or_default();
        }

        // Lower supply
        for supply in self
            .units
            .my
            .structures
            .iter()
            .of_type(self.race_values.supply)
        {
            if let Some(unit) = self
                .units
                .all
                .iter()
                .filter(|u| u.type_id().is_unit())
                .closest(supply)
            {
                if unit.is_mine() || unit.is_ally() {
                    supply.use_ability(AbilityId::MorphSupplyDepotLower, false);
                } else {
                    supply.use_ability(AbilityId::MorphSupplyDepotRaise, false);
                }
            }
        }
    }

    fn get_closest_free_worker(&self, location: Point2) -> Option<&Unit> {
        self.units
            .my
            .workers
            .iter()
            .filter(|w| !w.is_constructing() && !w.is_carrying_resource())
            .closest(location)
    }

    fn ideal_workers(&self) -> usize {
        80.min(
            self.counter()
                .all()
                .tech()
                .count(self.race_values.start_townhall)
                * 16
                + self.counter().all().tech().count(self.race_values.gas) * 3,
        )
    }

    fn move_workers(&self) {
        // For each resource gather point with too many workers, make unnecessary workers idle
        for townhall in self
            .units
            .my
            .townhalls
            .iter()
            .filter(|t| t.assigned_harvesters() > t.ideal_harvesters())
        {
            if let Some(worker) = self
                .units
                .my
                .workers
                .iter()
                .filter(|w| {
                    w.target_tag()
                        .is_some_and(|tag| self.units.mineral_fields.get(tag).is_some())
                })
                .closest(townhall)
            {
                worker.stop(false);
            }
        }

        for gas_building in self
            .units
            .my
            .gas_buildings
            .iter()
            .filter(|g| g.assigned_harvesters() > g.ideal_harvesters())
        {
            if let Some(worker) = self
                .units
                .my
                .workers
                .iter()
                .filter(|w| w.target_tag().is_some_and(|tag| tag == gas_building.tag()))
                .closest(gas_building.position())
            {
                worker.stop(false);
            }
        }

        for gas_building in self
            .units
            .my
            .gas_buildings
            .iter()
            .filter(|g| g.assigned_harvesters() < g.ideal_harvesters())
        {
            let worker = if let Some(worker) = self
                .units
                .my
                .workers
                .iter()
                .filter(|w| {
                    !w.is_constructing()
                        && !w
                            .target_tag()
                            .is_some_and(|tag| self.units.my.gas_buildings.get(tag).is_some())
                })
                .closest(gas_building)
            {
                worker
            } else {
                // Getting None here means we have no workers (we are Boned)
                return;
            };
            worker.gather(gas_building.tag(), false);
        }

        let mut workers = self.units.my.workers.iter().idle();

        // For each townhall with too few workers, joink an idle worker
        for townhall in self
            .units
            .my
            .townhalls
            .iter()
            .filter(|t| t.assigned_harvesters() < t.ideal_harvesters())
        {
            if let Some(worker) = workers.next() {
                let resource = self
                    .units
                    .mineral_fields
                    .iter()
                    .closer(10.0, townhall)
                    .closest(worker)
                    .expect(
                        "If ideal_harvesters > 0 then townhall should have nearby mineral resource",
                    );
                worker.gather(resource.tag(), false);
            }
        }
    }

    fn process_townhalls(&mut self) {
        // Always build expansion when we can afford it
        if self.can_afford(self.race_values.start_townhall, false)
            && self
                .counter()
                .ordered()
                .count(self.race_values.start_townhall)
                == 0
            && self.counter().all().count(self.race_values.start_townhall) < 6
        {
            // Find closest expansion site
            if let Some(expansion) = self.get_expansion() {
                // Find worker closest to expansion site
                if let Some(builder) = self.get_closest_free_worker(expansion.loc) {
                    builder.build(UID::CommandCenter, expansion.loc, false);
                    self.subtract_resources(UID::CommandCenter, false);
                }
            }
        }

        // Upgrade command centers to orbital commands
        if self.can_afford(UID::OrbitalCommand, false)
            && self.counter().ordered().count(UID::OrbitalCommand) == 0
        {
            if let Some(command_center) = self
                .units
                .my
                .townhalls
                .iter()
                .of_type(UID::CommandCenter)
                .closest(self.start_location)
            {
                command_center.use_ability(AbilityId::UpgradeToOrbitalOrbitalCommand, true);
            }
        }

        // Call down MULEs
        for orbital in self
            .units
            .my
            .townhalls
            .iter()
            .filter(|t| t.type_id() == UID::OrbitalCommand)
        {
            if orbital.has_ability(AbilityId::CalldownMULECalldownMULE) {
                if let Some(townhall) = self
                    .units
                    .my
                    .townhalls
                    .iter()
                    .filter(|t| t.assigned_harvesters() < t.ideal_harvesters())
                    .closest(orbital.position())
                {
                    if let Some(mineral) = self.units.mineral_fields.closest(townhall.position()) {
                        orbital.command(
                            AbilityId::CalldownMULECalldownMULE,
                            Target::Tag(mineral.tag()),
                            false,
                        );
                    }
                }
            }
        }
    }

    fn train_workers(&mut self) {
        if !self.can_afford(self.race_values.worker, false) {
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

        // Build worker in each idle townhall until we have enough
        let townhalls: Vec<_> = self
            .units
            .my
            .townhalls
            .iter()
            .idle()
            .filter(|t| t.is_ready())
            .take(target_amount - current_amount)
            .cloned()
            .collect();
        for townhall in townhalls {
            let worker = self.race_values.worker;
            if !self.can_afford(worker, false) {
                // We cannot afford any more workers anyway
                break;
            }
            townhall.train(worker, false);
            self.subtract_resources(worker, true);
        }
    }
}
