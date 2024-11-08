use crate::bot::TerranBot;
use rust_sc2::prelude::*;

use UnitTypeId as UID;

impl TerranBot {
    const UNITS: &'static [UID] = &[
        UID::Marine,
        UID::Hellion,
        UID::Medivac,
        UID::Reaper,
        UID::Cyclone,
    ];
    const COMBAT_UNITS: &'static [UID] = &[UID::Marine, UID::Hellion, UID::Reaper, UID::Cyclone];
    const SUPPORT_UNITS: &'static [UID] = &[UID::Medivac];

    pub(crate) fn train_army(&mut self) {
        // Loop through all our army buildings and build their units
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
                    let unit = if self.counter().count(UID::Reaper) < 2
                        && self.time < 180.0
                        && self.can_afford(UnitTypeId::Reaper, false)
                    {
                        UID::Reaper
                    } else {
                        UID::Marine
                    };
                    self.train_army_unit(building, unit);
                    if building.has_reactor() {
                        self.train_army_unit(building, unit);
                        self.train_army_unit(building, unit);
                        self.train_army_unit(building, UID::Marauder);
                    }
                }
                UID::Factory => {
                    self.train_army_unit(building, UID::Cyclone);
                }
                UID::Starport => {
                    if self.counter().count(UID::Medivac) < 3 {
                        self.train_army_unit(building, UID::Medivac)
                    }
                }
                _ => unreachable!("No other buildings should have passed the iterator filter"),
            }
        }
    }

    pub(crate) fn train_army_unit(&mut self, building: &Unit, unit: UnitTypeId) {
        if self.can_afford(unit, true) {
            building.train(unit, true);
            self.subtract_resources(unit, true);
            self.queued_units.push((unit, building.tag()))
        }
    }

    pub(crate) fn move_army(&self) {
        self.scout_and_harass();
        self.move_idle_army();
        self.move_active_army();
    }

    fn scout_and_harass(&self) {}

    fn move_idle_army(&self) {
        let idle_army = self.units.my.units.iter().of_types(&Self::UNITS).idle();
        let combat_units = self.units.my.units.iter().of_types(&Self::COMBAT_UNITS);
        let support_units = self.units.my.units.iter().of_types(&Self::SUPPORT_UNITS);
        let main_ramp: Point2 = self
            .ramps
            .my
            .top_center()
            .unwrap_or_else(|| {
                self.start_location
                    .towards(self.game_info.map_center, 10.0)
                    .into()
            })
            .into();

        // If we have more than 15 marines, attack. Otherwise, defend.
        if self.counter().count(UnitTypeId::Marine)
            >= self.counter().count(UnitTypeId::Barracks) * 15
            || self.supply_used >= 175
        {
            let targets = &self.units.enemy.all;
            if targets.is_empty() {
                for m in idle_army {
                    m.attack(Target::Pos(self.enemy_start), false);
                }
            } else {
                for m in idle_army {
                    m.attack(
                        Target::Tag(
                            targets
                                .closest(m)
                                .expect("We know `targets` isn't empty")
                                .tag(),
                        ),
                        false,
                    );
                }
            }
        } else {
            let targets = self.units.enemy.all.closer(30.0, self.start_location);
            if targets.is_empty() {
                for unit in idle_army {
                    if unit.distance_squared(main_ramp) > 7.0_f32.powi(2) {
                        unit.move_to(Target::Pos(main_ramp), false);
                    }
                }
            } else {
                for unit in combat_units {
                    unit.attack(
                        Target::Tag(
                            targets
                                .closest(unit)
                                .expect("We know `targets` isn't empty")
                                .tag(),
                        ),
                        false,
                    );
                }
                for unit in support_units {
                    if let Some(closest_combat_unit) = self
                        .units
                        .my
                        .units
                        .of_types(&Self::COMBAT_UNITS)
                        .closest(unit.position())
                    {
                        unit.move_to(Target::Tag(closest_combat_unit.tag()), false);
                    }
                }
            }
        }
    }

    fn move_active_army(&self) {
        for unit in self.units.my.units.iter().of_types(&Self::COMBAT_UNITS) {
            // Retreat units who are attacked and under 20% HP
            if unit.is_attacked() && unit.health_percentage().is_some_and(|h| h < 0.6) {
                if let Some(closest_enemy) = self.units.enemy.units.iter().closest(unit.position())
                {
                    let retreat = unit.position() * 2.0 - closest_enemy.position();
                    unit.move_to(Target::Pos(retreat), false);
                }
            }
            // Have retreated units close to a battle over 80% HP rejoin the fight
            else if !unit.is_attacking() && unit.health_percentage().is_some_and(|h| h >= 0.95) {
                if let Some(close_enemy) = self
                    .units
                    .enemy
                    .units
                    .iter()
                    .closer(50.0, unit.position())
                    .closest(unit.position())
                {
                    unit.attack(Target::Tag(close_enemy.tag()), false);
                }
            }
        }
    }
}
