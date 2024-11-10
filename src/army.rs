use crate::bot::TerranBot;
use rust_sc2::prelude::*;

use UnitTypeId as UID;

const UNITS: &[UID] = &[
    UID::Marine,
    UID::Hellion,
    UID::Medivac,
    UID::Reaper,
    UID::Cyclone,
];
const COMBAT_UNITS: &[UID] = &[UID::Marine, UID::Hellion, UID::Cyclone];
const SUPPORT_UNITS: &[UID] = &[UID::Medivac];

impl TerranBot {
    pub(crate) fn process_army(&mut self, iteration: usize) {
        if iteration % 5 == 0 {
            self.train_army();
        }
        self.scout_and_harass();
        self.move_idle_army();
        self.move_active_army();
    }

    fn train_army(&mut self) {
        // Loop through all our army buildings and build their units
        let buildings: Vec<_> = self
            .units
            .my
            .structures
            .iter()
            .of_types(&vec![UID::Barracks, UID::Factory, UID::Starport])
            .filter(|b| {
                b.is_ready() && (!b.is_active() || (b.orders().len() < 2 && b.has_reactor()))
            })
            .cloned()
            .collect();
        for building in &buildings {
            match building.type_id() {
                UID::Barracks => {
                    let unit = if self.counter().count(UID::Reaper) < 1
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

    fn train_army_unit(&mut self, building: &Unit, unit: UnitTypeId) {
        if self.can_afford(unit, false) {
            building.train(unit, true);
            self.subtract_resources(unit, true);
        }
    }

    fn scout_and_harass(&self) {
        for reaper in self.units.my.units.iter().of_type(UID::Reaper) {
            self.reaper_ai(reaper);
        }
    }

    fn combat_ai(&self, unit: &Unit) {
        if let Some(enemy) = self
            .units
            .enemy
            .units
            .iter()
            .closer(unit.sight_range() * 1.2, unit)
            .closest(unit)
        {
            if unit.is_attacking() || !unit.on_cooldown() {
                unit.attack(Target::Tag(enemy.tag()), false);
            } else {
                let retreat = unit.position() * 2.0 - enemy.position();
                unit.move_to(Target::Pos(retreat), false);
            }
        }
        // No enemy in range, move towards closest enemy structure
        else if let Some(structure) = self.units.enemy.structures.iter().closest(unit) {
            unit.move_to(Target::Tag(structure.tag()), false);
        }
        // No enemy unit at all, move to enemy start location
        else {
            unit.move_to(Target::Pos(self.enemy_start), false);
        }
    }

    fn avoid_close_allies(&self, unit: &Unit) {
        const MINIMUM_DISTANCE_BETWEEN_ARMY_UNITS: f32 = 1.0;
        if let Some(ally) = self
            .units
            .my
            .units
            .iter()
            .closer(MINIMUM_DISTANCE_BETWEEN_ARMY_UNITS, unit)
            .closest(unit)
        {
            let opposite_dir = (unit.position() * 2.0 - ally.position()).normalize();
            unit.move_to(Target::Pos(opposite_dir), false);
        }
    }

    fn reaper_ai(&self, reaper: &Unit) {
        self.combat_ai(reaper);
    }

    fn marine_ai(&self, marine: &Unit) {
        self.combat_ai(marine);
    }

    fn move_idle_army(&self) {
        let army = self.units.my.units.iter().of_types(&UNITS);
        let combat_units = self.units.my.units.iter().of_types(&COMBAT_UNITS);
        let support_units = self.units.my.units.iter().of_types(&SUPPORT_UNITS);
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
            || self.supply_used >= 190
        {
            for unit in army {
                self.marine_ai(unit);
            }
        } else {
            let targets = self.units.enemy.all.closer(30.0, self.start_location);
            if targets.is_empty() {
                for unit in army {
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
                        .of_types(&COMBAT_UNITS)
                        .closest(unit.position())
                    {
                        unit.move_to(Target::Tag(closest_combat_unit.tag()), false);
                    }
                }
            }
        }
    }

    fn move_active_army(&self) {
        for unit in self.units.my.units.iter().of_types(&COMBAT_UNITS) {
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
