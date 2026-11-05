use super::*;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct Wound {
    pub part: Part,
    pub position: Vec3,
    pub normal: Vec3,
    pub age: u16,
}

impl Blood {
    pub(super) fn emit_wounds(
        &mut self,
        tick: u64,
        settings: &settings::Blood,
        transforms: &BTreeMap<Part, Matrix>,
    ) {
        let duration = settings.seconds.min(60) * 30;
        self.wounds.retain_mut(|wound| {
            wound.age = wound.age.saturating_add(1);
            wound.age < duration && transforms.contains_key(&wound.part)
        });
        let amount = (f32::from(settings.amount) / 100.0).min(1.0);
        if tick.is_multiple_of(8) {
            for index in 0..self.wounds.len() {
                let wound = &self.wounds[index];
                let m = transforms[&wound.part];
                let position = rotation(m) * wound.position + translation(m);
                if self.drops.len() >= DROP_LIMIT || self.random() > amount {
                    break;
                }
                let velocity = Vec3::new(
                    (self.random() - 0.5) * 18.0,
                    12.0,
                    (self.random() - 0.5) * 18.0,
                );
                self.drops.push(Drop {
                    weapon: false,
                    previous: position,
                    position,
                    velocity,
                    life: 90,
                });
            }
        }
        if settings.spurts == 0 || self.drops.len() >= DROP_LIMIT {
            return;
        }
        let strength = f32::from(settings.spurts.min(200)) / 100.0;
        for index in 0..self.wounds.len() {
            let wound = &self.wounds[index];
            if wound.age % 18 >= 3 {
                continue;
            }
            let m = transforms[&wound.part];
            let r = rotation(m);
            if r.determinant().abs() < 0.0001 {
                continue;
            }
            let Some(normal) = (r.inverse().transpose() * wound.normal).try_normalize() else {
                continue;
            };
            let position = r * wound.position + translation(m) + normal * 12.0;
            let pressure = (1.0 - f32::from(wound.age) / f32::from(duration)).sqrt();
            for _ in 0..(strength * 2.0).ceil() as usize {
                if self.drops.len() >= DROP_LIMIT {
                    return;
                }
                if self.random() > amount * pressure {
                    continue;
                }
                let spread =
                    Vec3::new(self.random(), self.random(), self.random()) - Vec3::splat(0.5);
                let velocity = normal * (55.0 + self.random() * 30.0) * strength.sqrt() * pressure
                    + spread * 18.0
                    - Vec3::Y * 12.0;
                self.drops.push(Drop {
                    weapon: false,
                    previous: position,
                    position,
                    velocity,
                    life: 90,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; ordinary melee hits must produce animated wound jets without changing the game"]
    fn real_hits_create_wounds_and_jets() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.events = Default::default();
        let guard = *game
            .roster
            .order
            .iter()
            .find(|&&id| {
                let a = game.roster.get(id);
                id != game.player_id && a.patrol.is_some() && a.weapons.iter().any(Option::is_some)
            })
            .unwrap();
        for id in game.roster.order.clone() {
            if id != game.player_id && id != guard {
                let _ = game.remove_actor(id);
            }
        }
        let enemy = game.roster.get(guard);
        let offset = game
            .math
            .velocity(enemy.rotation[1], 1300, 0)
            .map(i32::from);
        game.teleport_player(enemy.position.wrapping_add(offset.into()), true)
            .unwrap();
        let delta = game
            .roster
            .get(guard)
            .position
            .wrapping_sub(game.player().position);
        game.roster.get_mut(game.player_id).rotation[1] =
            game.math.atan2(-delta.x, -delta.z).unwrap();
        crate::demo::sparring(&mut game, guard).unwrap();
        game.roster.get_mut(guard).invulnerable = false;
        game.intelligence.difficulty = tenchu_core::ai::Difficulty::Easy;
        let graphics = settings::Graphics::default();
        let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
        let surfaces = Surfaces::new(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let settings = settings::Blood {
            spurts: 100,
            weapons: false,
            ..settings::Blood::modern()
        };
        let mut blood = Blood::default();
        blood.tick(&game, &models, &surfaces, &settings, None);
        let mut wounds = 0;
        let mut jets = 0;
        for tick in 0..480u32 {
            game.step(if tick.is_multiple_of(45) {
                tenchu_core::input::SQUARE
            } else {
                0
            })
            .unwrap();
            let before = crate::session::tests::state(&game);
            blood.tick(&game, &models, &surfaces, &settings, None);
            assert_eq!(before, crate::session::tests::state(&game));
            wounds = wounds.max(blood.wounds.len());
            jets += blood
                .drops
                .iter()
                .filter(|d| d.life == 89 && d.velocity.length() > 40.0)
                .count();
        }
        assert!(
            game.roster.get(guard).health < 9999,
            "The demo must land real hits"
        );
        assert!(wounds > 0, "No wounds from ordinary melee hits");
        assert!(jets > 0, "No jets from ordinary melee wounds");
    }
}
