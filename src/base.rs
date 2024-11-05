use crate::bot::{BotError, TerranBot};
use rust_sc2::prelude::*;

impl TerranBot {
    pub(crate) fn ideal_workers(&self) -> usize {
        80.min(
            self.counter()
                .all()
                .tech()
                .count(self.race_values.start_townhall)
                * 16
                + self.counter().all().tech().count(self.race_values.gas) * 3,
        )
    }

    pub(crate) fn train_workers(&mut self) {
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

        // Build worker in each idle townhall until we have enough
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
        }

        for _ in 0..units_in_progress {
            self.subtract_resources(UnitTypeId::SCV, true);
        }
    }

    pub(crate) fn get_closest_free_worker(&self, location: Point2) -> Option<&Unit> {
        self.units
            .my
            .workers
            .iter()
            .filter(|w| !w.is_constructing())
            .closest(location)
    }

    pub(crate) fn build_expansion(&mut self) {
        // Always build expansion when we can afford it
        if self.can_afford(UnitTypeId::CommandCenter, false)
            && self.counter().ordered().count(UnitTypeId::CommandCenter) == 0
        {
            // Find closest expansion site
            if let Some(expansion) = self.get_expansion() {
                // Find worker closest to expansion site
                if let Some(builder) = self.get_closest_free_worker(expansion.loc) {
                    builder.build(UnitTypeId::CommandCenter, expansion.loc, false);
                    self.subtract_resources(UnitTypeId::CommandCenter, false);
                }
            }
        }

        // TODO: Upgrade command centers to orbital command
    }

    pub(crate) fn build_supply(&mut self) {
        // Build supply if none is being built and we have less than 5 left
        if self.supply_left < 5
            && self.counter().ordered().count(self.race_values.supply) == 0
            && self.can_afford(self.race_values.supply, false)
        {
            self.build_in_base(self.race_values.supply)
                .unwrap_or_default();
        }
    }

    pub(crate) fn build_in_base(&mut self, building: UnitTypeId) -> Result<(), BotError> {
        if !self.can_afford(building, false) {
            return Err(BotError::CannotAfford(building));
        }
        let main_base = self.start_location.towards(self.game_info.map_center, 8.0);
        self.build_close_to(building, main_base)
    }

    pub(crate) fn build_close_to(
        &mut self,
        building: UnitTypeId,
        location: Point2,
    ) -> Result<(), BotError> {
        if TECH_REQUIREMENTS
            .get(&building)
            .is_some_and(|&r| self.counter().count(r) == 0)
        {
            return Err(BotError::UnfulfilledTechRequirement(building));
        }
        let placement = self
            .find_placement(
                building,
                location,
                PlacementOptions {
                    step: 4,
                    max_distance: 25,
                    ..Default::default()
                },
            )
            .ok_or(BotError::NoSuitableLocation(building, location))?;

        let builder = self
            .get_closest_free_worker(placement)
            .ok_or(BotError::NoSuitableWorker)?;

        builder.build(building, placement, false);
        self.subtract_resources(building, false);
        Ok(())
    }

    pub(crate) fn build_structures(&mut self) {
        use UnitTypeId as UID;

        if self.counter().all().count(UID::Barracks) == 0 {
            self.build_in_base(UID::Barracks).unwrap_or_default();
        }

        // Build at least one of each army building
        for building in [UID::Factory, UID::Starport] {
            if self.counter().all().count(building) == 0
                && self.counter().all().count(self.race_values.start_townhall) != 1
            {
                self.build_in_base(building).unwrap_or_default();
            }
        }

        // If we have built barracks, try to build refinery
        if self.counter().all().count(UID::Barracks) != 0 {
            for townhall in &self.units.my.townhalls {
                if let Some(geyser) = self.find_gas_placement(townhall.position()) {
                    if let Some(builder) = self.get_closest_free_worker(geyser.position()) {
                        builder.build_gas(geyser.tag(), false);
                    }
                }
            }
        }

        if self.counter().count(UID::Starport) > 0
            && self.minerals > 500
            && self.counter().count(UID::Barracks) < 4
        {
            self.build_in_base(UID::Barracks).unwrap_or_default();
        }
    }

    pub(crate) fn move_workers(&self) {
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
                .closest(townhall.position())
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
                .closest(gas_building.position())
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
            let resource = self
                .units
                .mineral_fields
                .iter()
                .closest(townhall.position())
                .expect(
                    "If ideal_harvesters > 0 then townhall should have nearby mineral resource",
                );
            if let Some(worker) = workers.next() {
                worker.gather(resource.tag(), false);
            }
        }
    }
}
