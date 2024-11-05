use crate::bot::TerranBot;
use rust_sc2::prelude::*;

impl TerranBot {
    pub(crate) fn train_army(&mut self) {
        use UnitTypeId as UID;

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

    pub(crate) fn train_army_unit(&mut self, building: &Unit, unit: UnitTypeId) {
        if self.can_afford(unit, true) {
            building.train(unit, true);
            self.subtract_resources(unit, true);
        }
    }

    pub(crate) fn move_army(&self) {
        use UnitTypeId as UID;

        const UNITS: &[UID] = &[UID::Marine, UID::Hellion, UID::Medivac];
        let army = self.units.my.units.iter().of_types(&UNITS).idle();
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
                for m in army {
                    m.attack(Target::Pos(self.enemy_start), false);
                }
            } else {
                for m in army {
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
                for m in army {
                    if m.distance_squared(main_ramp) > 7.0_f32.powi(2) {
                        m.move_to(Target::Pos(main_ramp), false);
                    }
                }
            } else {
                for m in army {
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
        }
    }
}
