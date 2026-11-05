use super::*;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) enum Gush {
    Pending,
    Active {
        part: Part,
        position: Vec3,
        age: u16,
    },
    Finished,
}

impl Gush {
    pub fn active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn part(&self) -> Option<Part> {
        match self {
            Self::Active { part, .. } => Some(*part),
            _ => None,
        }
    }
}

impl Blood {
    pub(super) fn track_gushes(
        &mut self,
        game: &Game,
        models: &Models,
        contacts: &[Contact],
        settings: &settings::Blood,
        health: &BTreeMap<usize, i16>,
        frame: Option<&Frame>,
    ) {
        if !settings.gush(false) {
            return;
        }
        if game.cinematic.active {
            self.gushes.values_mut().for_each(|g| *g = Gush::Finished);
            self.drops.retain(|d| d.kind != DropKind::Gush);
            return;
        }
        self.gushes.retain(|root, _| {
            health.get(root) == Some(&0)
                && (*root != game.player().root_model || settings.gush(true))
        });
        for actor in game.roster.actors.iter().flatten() {
            if actor.health != 0
                || !settings.gush(actor.id == game.player_id)
                || actor.scripted
                || actor.kind & 0xf0 == 0xa0
                || actor.motion.action == tenchu_core::ids::MOT_DEAD_DROWN
            {
                if let Some(gush) = self.gushes.get_mut(&actor.root_model) {
                    *gush = Gush::Finished;
                }
                continue;
            }
            if self
                .health
                .get(&actor.root_model)
                .is_some_and(|&old| old > 0)
            {
                self.gushes.entry(actor.root_model).or_insert(Gush::Pending);
            }
            let Some(gush @ Gush::Pending) = self.gushes.get_mut(&actor.root_model) else {
                continue;
            };
            let Some(contact) = contacts.iter().find(|contact| {
                contact
                    .source
                    .is_some_and(|model| actor.parts.contains(&model))
            }) else {
                if corpse_ready(actor, frame) {
                    *gush = Gush::Finished;
                }
                continue;
            };
            appearance::visit_parts_filtered(
                game,
                true,
                Some(models),
                frame,
                |key| {
                    matches!(key, Part::Actor { root, model, .. }
                    if root == actor.root_model && Some(model) == contact.source)
                },
                |key, _, matrix| {
                    let r = rotation(matrix);
                    if r.determinant().abs() >= 0.0001 {
                        *gush = Gush::Active {
                            part: key.canonical(),
                            position: r.inverse() * (contact.position - translation(matrix)),
                            age: 0,
                        };
                    }
                },
            );
        }
    }

    pub(super) fn emit_gushes(
        &mut self,
        settings: &settings::Blood,
        transforms: &BTreeMap<Part, Matrix>,
    ) {
        if self.gushes.is_empty() {
            return;
        }
        let duration = settings.gush_seconds.min(15) * 30;
        let strength = f32::from(settings.death_gush.min(200)) / 100.0;
        let amount = f32::from(settings.amount.min(200)) / 100.0;
        let mut gushes = std::mem::take(&mut self.gushes);
        let mut emitting = gushes
            .values()
            .filter(|g| {
                matches!(g,
            Gush::Active { age, .. } if *age < duration && *age % 30 < 3)
            })
            .count();
        for gush in gushes.values_mut() {
            let Gush::Active {
                part,
                position,
                age,
            } = gush
            else {
                continue;
            };
            let Some(matrix) = transforms.get(part).filter(|_| *age < duration) else {
                *gush = Gush::Finished;
                continue;
            };
            let pressure = (1.0 - f32::from(*age) / f32::from(duration)).sqrt();
            let emit = *age % 30 < 3;
            *age += 1;
            if !emit {
                continue;
            }
            let position = rotation(*matrix) * *position + translation(*matrix);
            // STAGE8A/R.CAD sequence 15 and CHRANIM.C use repeated upward bursts,
            // unlike the directed SetGore jets used for ordinary deaths.
            let count = (20.0 / 3.0 * strength * amount * pressure).ceil() as usize;
            let budget = DROP_LIMIT
                .saturating_sub(self.drops.len())
                .div_ceil(emitting.max(1));
            emitting = emitting.saturating_sub(1);
            for _ in 0..count.min(budget) {
                let velocity = Vec3::new(
                    (self.random() - 0.5) * 120.0,
                    -90.0 - self.random() * 60.0,
                    (self.random() - 0.5) * 120.0,
                ) * pressure
                    * strength.sqrt();
                self.drops.push(Drop {
                    kind: DropKind::Gush,
                    previous: position,
                    position,
                    velocity,
                    life: 90,
                });
            }
        }
        self.gushes = gushes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::{actor::State, ids::*};

    #[test]
    #[ignore = "requires TENCHU_DISC; death gushes respect the original emission, alternate player poses, corpse cleanup and serialized continuation"]
    fn death_gushes_follow_real_death_events_without_changing_gameplay() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for (character, player, motion, alternate) in [
            (0, true, MOT_DEAD, 0),
            (1, true, MOT_DEAD, 100),
            (0, false, MOT_DEAD_STEALTH_BACK, 0),
            (1, false, MOT_DEAD_STEALTH_BACK + 3, 0),
        ] {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            game.roster.keep_all_active = true;
            game.intelligence.ignore_player = true;
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let id = if player {
                game.player_id
            } else {
                *game
                    .roster
                    .order
                    .iter()
                    .find(|&&id| {
                        let actor = game.roster.get(id);
                        actor.patrol.is_some() && actor.weapons.iter().any(Option::is_some)
                    })
                    .unwrap()
            };
            let position = game.player().position;
            if !player {
                game.move_actor(id, position, 0, true).unwrap();
            }
            let models = Models::build(
                &game.mission,
                &Default::default(),
                &settings::Graphics::default(),
            )
            .unwrap();
            let surfaces = Surfaces::new(&render_mesh::stage(
                &game.mission,
                &game.math,
                Some(&models),
            ));
            let settings = settings::Blood {
                death_gush: 100,
                gush_seconds: 3,
                drips: BloodStyle::Off,
                weapons: false,
                model_stains: false,
                runoff: false,
                ..settings::Blood::modern()
            };
            let mut blood = Blood::default();
            blood.tick(&game, &models, &surfaces, &settings, None);
            let root = game.roster.get(id).root_model;
            let actor = game.roster.get_mut(id);
            actor.health = 0;
            actor.status = State::Dead;
            actor.velocity = [0; 3];
            actor.motion.change(
                motion,
                &game.mission.characters[actor.definition].motions,
                &game.mission.common_motions,
                &game.mission.clips,
            );
            let mut deaths = crate::visuals::death::Deaths::load(&assets).unwrap();
            let mut first_original = None;
            let mut first_gush = None;
            let mut last_gush = 0;
            let mut restored: Option<Blood> = None;
            let mut followed = 0;
            for tick in 0..360 {
                game.step(0).unwrap();
                deaths.tick(&game, alternate).unwrap();
                let frame = deaths.frame.as_ref();
                if crate::visuals::death::blood::slots(&game.effects, frame)
                    .enumerate()
                    .any(|(i, effect)| {
                        matches!(effect, Some(Effect::Blood(b)) if b.gore)
                            && crate::visuals::death::blood::gore_source(&game.effects, frame, i)
                                .is_some_and(|p| game.roster.get(id).parts.contains(&p))
                    })
                {
                    first_original.get_or_insert(tick);
                }
                let before = crate::session::tests::state(&game);
                blood.tick(&game, &models, &surfaces, &settings, frame);
                assert_eq!(before, crate::session::tests::state(&game));
                let new: Vec<_> = blood
                    .drops
                    .iter()
                    .filter(|d| d.kind == DropKind::Gush && d.life == 89)
                    .collect();
                if !new.is_empty() {
                    first_gush.get_or_insert(tick);
                    last_gush = tick;
                    let Gush::Active { part, position, .. } = &blood.gushes[&root] else {
                        panic!("active emitter")
                    };
                    let transforms = transforms(&game, &models, frame, true, &[*part].into());
                    let matrix = transforms[part];
                    let expected = rotation(matrix) * *position + translation(matrix);
                    assert!(new.iter().all(|d| d.previous.distance(expected) < 0.01));
                    followed += 1;
                }
                assert!(blood.drops.len() <= DROP_LIMIT);
                assert!(
                    blood
                        .drops
                        .iter()
                        .all(|d| d.position.is_finite() && d.velocity.is_finite())
                );
                if let Some(restored) = &mut restored {
                    restored.tick(&game, &models, &surfaces, &settings, frame);
                    assert_eq!(
                        postcard::to_allocvec(restored).unwrap(),
                        postcard::to_allocvec(&blood).unwrap()
                    );
                } else if first_gush.is_some() {
                    let mut recorder = crate::rewind::Recorder::default();
                    recorder.begin();
                    blood.capture(&mut recorder).unwrap();
                    restored = Some(recorder.finish().unwrap().decode().unwrap());
                }
            }
            assert_eq!(
                first_gush, first_original,
                "character {character}, motion {motion:x}, alternate {alternate}"
            );
            assert!(followed >= 6, "No sustained bursts");
            assert!(last_gush - first_gush.unwrap() < 90);
            assert!(matches!(blood.gushes[&root], Gush::Finished));
            assert!(blood.drops.is_empty());
            blood.clear();
            for _ in 0..120 {
                game.step(0).unwrap();
                blood.tick(&game, &models, &surfaces, &settings, deaths.frame.as_ref());
            }
            assert!(
                blood.drops.is_empty(),
                "Cleared corpses must not start bleeding again"
            );
        }
    }

    #[test]
    fn crowded_gushes_share_the_particle_budget_and_stop() {
        let settings = settings::Blood {
            death_gush: 200,
            gush_seconds: 2,
            ..settings::Blood::modern()
        };
        let mut blood = Blood::default();
        let mut transforms = BTreeMap::new();
        for root in 0..64 {
            let part = Part::Actor {
                root,
                model: root,
                kind: 0,
                inspection: false,
            };
            transforms.insert(part, Matrix::default());
            blood.gushes.insert(
                root,
                Gush::Active {
                    part,
                    position: Vec3::ZERO,
                    age: 0,
                },
            );
        }
        for _ in 0..90 {
            blood.emit_gushes(&settings, &transforms);
            assert!(blood.drops.len() <= DROP_LIMIT);
            blood.drops.clear();
        }
        assert!(blood.gushes.values().all(|g| !g.active()));
    }
}
