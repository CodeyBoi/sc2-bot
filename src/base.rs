use crate::bot::{BuildError, TerranBot};
use rust_sc2::prelude::*;
use UnitTypeId as UID;

const BUILD_ORDER: &[UID] = &[
    UID::SupplyDepot,
    UID::Barracks,
    UID::Refinery,
    UID::BarracksReactor,
    UID::OrbitalCommand,
    UID::CommandCenter,
    UID::SupplyDepot,
    UID::Marine,
    UID::Marine,
    UID::Barracks,
    UID::Barracks,
    UID::BarracksTechLab,
    UID::BarracksTechLab,
    UID::Refinery,
    UID::Refinery,
    UID::Factory,
    UID::EngineeringBay,
    UID::EngineeringBay,
    UID::Barracks,
    UID::Barracks,
    UID::Refinery,
    UID::FactoryReactor,
    UID::Starport,
    UID::BarracksReactor,
    UID::BarracksReactor,
    UID::Factory,
    UID::FactoryTechLab,
    UID::Armory,
];

const UPGRADE_ORDER: &[UpgradeId] = &[
    UpgradeId::ShieldWall,
    UpgradeId::Stimpack,
    UpgradeId::TerranInfantryWeaponsLevel1,
    UpgradeId::TerranInfantryArmorsLevel1,
    UpgradeId::TerranInfantryWeaponsLevel2,
    UpgradeId::TerranInfantryArmorsLevel2,
    UpgradeId::TerranInfantryWeaponsLevel3,
    UpgradeId::TerranInfantryArmorsLevel3,
];

impl TerranBot {
    pub(crate) fn process_base(&mut self, iteration: usize) {
        if iteration % 5 == 0 {
            self.build_next_in_build_order().unwrap_or_default();
            self.research_next_in_upgrade_order().unwrap_or_default();
            self.process_supply();
            self.process_structure_abilities();
        }
        if iteration % 5 == 1 {
            self.train_workers();
        }
        self.move_workers();
    }

    fn build_next_in_build_order(&mut self) -> Result<(), BuildError> {
        let next = self
            .get_current_build_prio()
            .ok_or(BuildError::EndOfBuildOrder)?;
        if !self.can_afford(next, next.is_unit()) {
            return Err(BuildError::CannotAfford(next));
        } else if TECH_REQUIREMENTS
            .get(&next)
            .is_some_and(|&requirement| self.counter().tech().count(requirement) == 0)
        {
            return Err(BuildError::UnfulfilledTechRequirement(next));
        }

        match next {
            UID::CommandCenter => self.build_expansion()?,
            UID::OrbitalCommand => self.upgrade_townhall()?,
            _ if next == self.race_values.gas => self.build_gas_building()?,
            unit if next.is_unit() => self.train_unit(unit)?,
            addon if next.is_addon() => self.build_addon(addon)?,
            structure if next.is_structure() => self.build_structure(structure)?,
            _ => {}
        }

        let time = format!(
            "{:0>2}:{:0>2} ",
            self.time as usize / 60,
            self.time as usize % 60
        );
        self.log(&format!("{}{:?}: construction started", time, next));

        self.subtract_resources(next, next.is_unit());
        self.build_order_index += 1;
        Ok(())
    }

    fn build_expansion(&self) -> Result<(), BuildError> {
        // Find closest expansion site
        let expansion = self.get_expansion().ok_or(BuildError::NoSuitableLocation(
            self.race_values.start_townhall,
            self.start_location,
        ))?;
        // Find worker closest to expansion site
        let builder = self
            .get_closest_free_worker(expansion.loc)
            .ok_or(BuildError::NoSuitableWorker)?;
        builder.build(self.race_values.start_townhall, expansion.loc, false);
        Ok(())
    }

    fn upgrade_townhall(&self) -> Result<(), BuildError> {
        let command_center = self
            .units
            .my
            .townhalls
            .iter()
            .of_type(UID::CommandCenter)
            .idle()
            .closest(self.start_location)
            .ok_or(BuildError::NoProducer(UID::OrbitalCommand))?;
        command_center.use_ability(AbilityId::UpgradeToOrbitalOrbitalCommand, false);
        Ok(())
    }

    fn build_gas_building(&self) -> Result<(), BuildError> {
        let geyser = self
            .units
            .my
            .townhalls
            .iter()
            .find_map(|t| self.find_gas_placement(t.position()))
            .ok_or(BuildError::NoSuitableLocation(
                self.race_values.gas,
                self.start_location,
            ))?;

        let builder = self
            .get_closest_free_worker(geyser.position())
            .ok_or(BuildError::NoSuitableWorker)?;

        builder.build_gas(geyser.tag(), false);
        Ok(())
    }

    fn train_unit(&self, unit: UID) -> Result<(), BuildError> {
        let producer = *PRODUCERS.get(&unit).ok_or(BuildError::NoProducer(unit))?;
        let producer = self
            .units
            .my
            .structures
            .iter()
            .of_type(producer)
            .almost_unused()
            .closest(self.start_location)
            .ok_or(BuildError::NoProducer(unit))?;
        producer.train(unit, true);
        Ok(())
    }

    fn build_structure(&self, structure: UID) -> Result<(), BuildError> {
        let main_base = if structure == self.race_values.supply {
            self.start_location.towards(self.game_info.map_center, 2.0)
        } else {
            self.start_location.towards(self.game_info.map_center, 10.0)
        };
        let placement_options = PlacementOptions {
            step: if structure == self.race_values.supply {
                3
            } else {
                4
            },
            addon: matches!(structure, UID::Barracks | UID::Factory | UID::Starport),
            max_distance: 30,
            ..Default::default()
        };
        let placement = self
            .find_placement(structure, main_base, placement_options)
            .ok_or(BuildError::NoSuitableLocation(structure, main_base))?;

        let builder = self
            .get_closest_free_worker(placement)
            .ok_or(BuildError::NoSuitableWorker)?;

        builder.build(structure, placement, false);

        Ok(())
    }

    fn build_addon(&mut self, addon: UID) -> Result<(), BuildError> {
        let producer = match addon {
            UID::BarracksReactor | UID::BarracksTechLab => UID::Barracks,
            UID::FactoryReactor | UID::FactoryTechLab => UID::Factory,
            UID::StarportReactor | UID::StarportTechLab => UID::Starport,
            _ => return Err(BuildError::InvalidArgument(addon)),
        };
        let producer = self
            .units
            .my
            .structures
            .iter()
            .of_type(producer)
            .idle()
            .ready()
            .filter(|p| !p.has_addon())
            .closest(self.start_location)
            .ok_or(BuildError::NoProducer(addon))?;

        producer.stop(false);
        producer.train(addon, false);

        Ok(())
    }

    fn process_supply(&mut self) {
        // Build supply if none is being built and we have less than 5 left
        let structure = self.race_values.supply;
        let ordered = self.counter().ordered().count(structure);
        if ((self.supply_left < 5 && ordered == 0)
            || (self.supply_left < 2 && ordered == 1 && self.time > 180.0))
            && self.can_afford(structure, false)
        {
            self.build_structure(structure).unwrap_or_default();
            self.subtract_resources(structure, false);
        }
    }

    fn process_structure_abilities(&self) {
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

        // Call down MULEs
        for orbital in self.units.my.townhalls.iter().of_type(UID::OrbitalCommand) {
            if !orbital.has_ability(AbilityId::CalldownMULECalldownMULE) {
                continue;
            }
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

    fn research_next_in_upgrade_order(&mut self) -> Result<(), BuildError> {
        let upgrade = *UPGRADE_ORDER
            .get(self.upgrade_order_index)
            .ok_or(BuildError::EndOfBuildOrder)?;
        if !self.can_afford_upgrade(upgrade) {
            return Err(BuildError::CannotAffordUpgrade(upgrade));
        } else if matches!(
            upgrade,
            UpgradeId::TerranInfantryWeaponsLevel2
                | UpgradeId::TerranInfantryArmorsLevel2
                | UpgradeId::TerranInfantryWeaponsLevel3
                | UpgradeId::TerranInfantryArmorsLevel3
        ) && self.counter().count(UID::Armory) == 0
        {
            return Err(BuildError::UnfulfilledTechRequirement(UID::Armory));
        }

        let producer = *RESEARCHERS
            .get(&upgrade)
            .ok_or(BuildError::NoResearcher(upgrade))?;
        let producer = self
            .units
            .my
            .structures
            .iter()
            .of_type(producer)
            .idle()
            .ready()
            .closest(self.start_location)
            .ok_or(BuildError::NoResearcher(upgrade))?;
        producer.research(upgrade, false);
        self.subtract_upgrade_cost(upgrade);
        self.upgrade_order_index += 1;

        let time = format!(
            "{:0>2}:{:0>2} ",
            self.time as usize / 60,
            self.time as usize % 60
        );
        self.log(&format!("{}{:?}: research started", time, upgrade));
        Ok(())
    }

    pub(crate) fn get_current_build_prio(&self) -> Option<UID> {
        BUILD_ORDER.get(self.build_order_index).copied()
    }

    // pub(crate) fn build_structures_old(&mut self) {
    //     if self.counter().all().count(UID::Barracks) == 0 {
    //         self.build_in_base(UID::Barracks).unwrap_or_default();
    //     }

    //     // Build at least one of each army building
    //     for building in [UID::Factory, UID::Starport] {
    //         if self.counter().all().count(building) == 0
    //             && self
    //                 .counter()
    //                 .all()
    //                 .tech()
    //                 .count(self.race_values.start_townhall)
    //                 != 1
    //         {
    //             self.build_in_base(building).unwrap_or_default();
    //         }
    //     }

    //     // If we have begun constructing barracks, try to build refinery
    //     match (
    //         self.counter().all().count(UID::Barracks),
    //         self.counter().count(UID::Factory),
    //         self.counter().all().count(self.race_values.gas),
    //     ) {
    //         (1, _, 0) | (_, 1, 1) => {
    //             for townhall in &self.units.my.townhalls {
    //                 if let Some(geyser) = self.find_gas_placement(townhall.position()) {
    //                     if let Some(builder) = self.get_closest_free_worker(geyser.position()) {
    //                         builder.build_gas(geyser.tag(), false);
    //                     }
    //                 }
    //             }
    //         }
    //         _ => {}
    //     }

    //     if self.counter().count(UID::Starport) > 0
    //         && self.minerals > 500
    //         && self.counter().all().count(UID::Barracks) < 6
    //     {
    //         self.build_in_base(UID::Barracks)
    //             .inspect_err(|e| println!("failed building barracks: {:?}", e))
    //             .unwrap_or_default();
    //     }

    //     // Upgrade barracks
    //     if self.counter().all().count(UID::Factory) > 0 {
    //         if let Some(barracks) = self
    //             .units
    //             .my
    //             .structures
    //             .iter()
    //             .idle()
    //             .of_type(UID::Barracks)
    //             .closest(self.start_location)
    //         {
    //             barracks.train(UID::BarracksReactor, false);
    //             self.subtract_resources(UID::BarracksReactor, false);
    //         }
    //     }

    //     if self
    //         .units
    //         .my
    //         .structures
    //         .iter()
    //         .of_type(UID::Barracks)
    //         .filter(|b| b.has_reactor())
    //         .count()
    //         >= 2
    //         && self
    //             .units
    //             .my
    //             .structures
    //             .iter()
    //             .of_type(UID::Barracks)
    //             .filter(|b| b.has_techlab())
    //             .count()
    //             < 1
    //     {
    //         if let Some(barracks) = self
    //             .units
    //             .my
    //             .structures
    //             .iter()
    //             .idle()
    //             .of_type(UID::Barracks)
    //             .closest(self.start_location)
    //         {
    //             barracks.train(UID::BarracksTechLab, false);
    //             self.subtract_resources(UID::BarracksTechLab, false);
    //         }
    //     }

    //     // // Upgrade factory
    //     // if let Some(factory) = self
    //     //     .units
    //     //     .my
    //     //     .structures
    //     //     .of_type(UID::Factory)
    //     //     .closest(self.start_location)
    //     // {
    //     //     factory.train(UID::FactoryTechLab, true);
    //     //     self.subtract_resources(UID::FactoryTechLab, false);
    //     // }

    //     // Build engineering bay
    //     if self.counter().all().count(UID::Starport) > 0
    //         && self.counter().all().count(UID::EngineeringBay) < 2
    //     {
    //         self.build_in_base(UID::EngineeringBay).unwrap_or_default();
    //     }

    //     for engineering_bay in self
    //         .units
    //         .my
    //         .structures
    //         .iter()
    //         .idle()
    //         .of_type(UID::EngineeringBay)
    //     {
    //         for upgrade in [
    //             AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel1,
    //             AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel2,
    //             AbilityId::EngineeringBayResearchTerranInfantryWeaponsLevel3,
    //             AbilityId::EngineeringBayResearchTerranInfantryArmorLevel1,
    //             AbilityId::EngineeringBayResearchTerranInfantryArmorLevel2,
    //             AbilityId::EngineeringBayResearchTerranInfantryArmorLevel1,
    //         ] {
    //             engineering_bay.use_ability(upgrade, true);
    //         }
    //     }

    //     if self.counter().count(UID::EngineeringBay) > 0
    //         && self.counter().all().count(UID::Armory) == 0
    //     {
    //         self.build_in_base(UID::Armory).unwrap_or_default();
    //     }
    // }

    fn get_closest_free_worker(&self, location: Point2) -> Option<&Unit> {
        self.units
            .my
            .workers
            .iter()
            .filter(|w| w.is_collecting() && !w.is_constructing() && !w.is_carrying_resource())
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
        // To keep track of idle workers
        let mut idle_workers: Vec<_> = self
            .units
            .my
            .workers
            .iter()
            .idle()
            .map(|w| w.tag())
            .collect();

        // Fill all gas buildings with workers
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
                continue;
            };
            worker.gather(gas_building.tag(), false);
            if let Some(idx) = idle_workers.iter().position(|&t| t == worker.tag()) {
                idle_workers.swap_remove(idx);
            }
        }

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
                idle_workers.push(worker.tag());
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
                idle_workers.push(worker.tag());
            }
        }

        // Let each townhall with too few workers yoink some idle ones
        for townhall in &self.units.my.townhalls {
            let wanted_amount = if let (Some(ideal), Some(assigned)) =
                (townhall.ideal_harvesters(), townhall.assigned_harvesters())
            {
                ideal.saturating_sub(assigned)
            } else {
                eprintln!("Error: tried to get assigned_harvesters from non-townhall unit");
                continue;
            };
            for _ in 0..wanted_amount {
                if let Some(worker) = self
                    .units
                    .my
                    .workers
                    .find_tags(&idle_workers)
                    .closest(townhall)
                {
                    let resource = self
                    .units
                    .mineral_fields
                    .iter()
                    .closer(8.0, townhall)
                    .closest(worker)
                    .expect(
                        "If ideal_harvesters > 0 then townhall should have nearby mineral resource",
                    );
                    worker.gather(resource.tag(), false);
                    if let Some(idx) = idle_workers.iter().position(|&t| t == worker.tag()) {
                        idle_workers.swap_remove(idx);
                    }
                }
            }
        }
    }

    fn train_workers(&mut self) {
        if !self.can_afford(self.race_values.worker, false)
            || (self
                .get_current_build_prio()
                .is_some_and(|b| b == UID::OrbitalCommand)
                && self
                    .units
                    .my
                    .structures
                    .iter()
                    .of_type(UID::Barracks)
                    .closest(self.start_location)
                    .is_some_and(|b| {
                        !b.is_ready() && (1.0 - b.build_progress()) * b.build_time() < 12.0
                    }))
        {
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

        // Build worker in each idle townhall until we have enough
        let townhalls: Vec<_> = self
            .units
            .my
            .townhalls
            .iter()
            .almost_idle()
            .filter(|t| t.is_ready())
            .take(target_amount.saturating_sub(current_amount))
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
