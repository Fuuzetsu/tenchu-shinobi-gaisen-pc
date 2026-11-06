use super::*;

impl Blood {
    pub(super) fn emit_severed(
        &mut self,
        game: &Game,
        models: &Models,
        settings: &settings::Blood,
        frame: Option<&Frame>,
    ) {
        if !settings.severed_bleeding() {
            return;
        }
        self.severed.retain(|root, _| {
            game.roster.actors.iter().flatten().any(|actor| {
                actor.root_model == *root && actor.health == 0 && actor.severed.is_some()
            })
        });
        let duration = settings.severed_seconds * 30;
        let strength = f32::from(settings.severed) / 100.0;
        let amount = f32::from(settings.amount) / 100.0;
        let mut emitters = Vec::new();
        for actor in game.roster.actors.iter().flatten() {
            let Some(severed) = &actor.severed else {
                continue;
            };
            let age = self.severed.entry(actor.root_model).or_default();
            if *age >= duration || actor.health != 0 {
                continue;
            }
            let key = |model| Part::Actor {
                root: actor.root_model,
                model,
                kind: actor.mesh_kind,
                inspection: false,
            };
            let part = &game.mission.models[&actor.mesh_kind][severed.index];
            let mut center = Vec3::ZERO;
            let mut count = 0;
            for vertex in part.geometry.iter().flat_map(|g| &g.vertices) {
                center += Vec3::from(vertex.map(f32::from));
                count += 1;
            }
            if count != 0 {
                center /= count as f32;
            }
            let direction = center.try_normalize().unwrap_or(-Vec3::Y);
            let pressure = (1.0 - f32::from(*age) / f32::from(duration)).sqrt();
            if age.is_multiple_of(5) {
                emitters.push((
                    key(actor.parts[severed.index]),
                    Vec3::ZERO,
                    -direction,
                    pressure,
                    false,
                ));
            }
            if *age % 12 < 3
                && let Some(parent) = severed.parent
            {
                let local = Matrix {
                    translation: actor.motion.pose.translation[severed.index],
                    rotation: game
                        .math
                        .rotate_yxz(actor.motion.pose.rotation[severed.index]),
                };
                emitters.push((
                    key(parent),
                    translation(local),
                    rotation(local) * direction,
                    pressure,
                    true,
                ));
            }
            *age += 1;
        }
        let needed = emitters.iter().map(|&(part, ..)| part).collect();
        let matrices = transforms(game, models, frame, true, &needed);
        let mut remaining = emitters.len();
        for (part, local, normal, pressure, body) in emitters {
            let budget = DROP_LIMIT
                .saturating_sub(self.drops.len())
                .div_ceil(remaining);
            remaining -= 1;
            let Some(&matrix) = matrices.get(&part) else {
                continue;
            };
            let normal = (rotation(matrix) * normal).normalize_or_zero();
            let position =
                translation(matrix) + rotation(matrix) * local + normal * 12.0 - Vec3::Y * 12.0;
            let count =
                ((if body { 3.0 } else { 1.0 }) * strength * amount * pressure).ceil() as usize;
            for _ in 0..count.min(budget) {
                let spread =
                    Vec3::new(self.random(), self.random(), self.random()) - Vec3::splat(0.5);
                let velocity = if body {
                    (normal * 55.0 - Vec3::Y * (90.0 + self.random() * 45.0) + spread * 45.0)
                        * pressure
                        * strength.sqrt()
                } else {
                    normal * 8.0 + Vec3::Y * 8.0 + spread * 12.0
                };
                self.drops.push(Drop {
                    kind: DropKind::Severed,
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
    use tenchu_core::{actor::State, ids::MOT_DEAD};

    #[test]
    #[ignore = "requires TENCHU_DISC; detached parts, alternate deaths, finite bleeding, stains and serialized continuation"]
    fn severed_wounds_follow_both_ends_without_replaying_or_changing_gameplay() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for character in 0..2 {
            for head in [false, true] {
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                let models =
                    Models::build(&game.mission, &Default::default(), &Default::default()).unwrap();
                let surfaces = Surfaces::new(&render_mesh::stage(
                    &game.mission,
                    &game.math,
                    Some(&models),
                ));
                let settings = settings::Blood {
                    death: BloodStyle::Off,
                    drips: BloodStyle::Off,
                    hit: BloodStyle::Off,
                    model_stains: false,
                    weapons: false,
                    runoff: false,
                    ..settings::Blood::modern()
                };
                let mut blood = Blood::default();
                blood.tick(&game, &models, &surfaces, &settings, None);
                let mut deaths = crate::visuals::death::Deaths::load(&assets).unwrap();
                game.combat.dismemberment = tenchu_core::game::dismemberment::Settings {
                    player: true,
                    player_head: if head { 100 } else { 0 },
                    player_arms: 100,
                    ..Default::default()
                };
                let player = game.roster.get_mut(game.player_id);
                player.health = 0;
                player.status = State::Dead;
                player.motion.change(
                    MOT_DEAD,
                    &game.mission.characters[player.definition].motions,
                    &game.mission.common_motions,
                    &game.mission.clips,
                );
                let mut detached_drops = 0;
                let mut jets = 0;
                for tick in 0..210 {
                    game.step(0).unwrap();
                    deaths.tick(&game, 100).unwrap();
                    let before = crate::session::tests::state(&game);
                    blood.tick(&game, &models, &surfaces, &settings, deaths.frame.as_ref());
                    assert_eq!(before, crate::session::tests::state(&game));
                    if let Some(severed) = &game.player().severed {
                        let actor = game.player();
                        let model = actor.parts[severed.index];
                        let actual = game.scene.models[model].local;
                        if let Some(frame) = &deaths.frame {
                            assert_eq!(
                                Frame::transform(
                                    Some(frame),
                                    Part::Actor {
                                        root: actor.root_model,
                                        model,
                                        kind: actor.mesh_kind,
                                        inspection: true,
                                    },
                                    Matrix::default()
                                ),
                                actual,
                                "alternate death reattached a severed part"
                            );
                        }
                        for drop in blood
                            .drops
                            .iter()
                            .filter(|d| d.kind == DropKind::Severed && d.life == 89)
                        {
                            if drop.velocity.length() > 35.0 {
                                jets += 1;
                            }
                            if drop.previous.distance(translation(actual)) < 60.0 {
                                detached_drops += 1;
                            }
                        }
                    }
                    if tick == 35 {
                        let saved = postcard::to_allocvec(&blood).unwrap();
                        let restored: Blood = postcard::from_bytes(&saved).unwrap();
                        assert_eq!(restored.severed, blood.severed);
                    }
                    if [20, 140].contains(&tick)
                        && let Some(path) = std::env::var_os("TENCHU_SEVERED_REVIEW")
                    {
                        let path = std::path::PathBuf::from(path);
                        std::fs::create_dir_all(&path).unwrap();
                        crate::visuals::death::tests::capture(
                            &game,
                            &models,
                            &blood,
                            deaths.frame.as_ref(),
                            game.player().severed.as_ref().map(|part| {
                                translation(
                                    game.scene.models[game.player().parts[part.index]].local,
                                )
                            }),
                            &path.join(format!("hero-{character}-head-{head}-tick-{tick}.png")),
                        );
                    }
                }
                assert!(game.player().severed.is_some());
                assert!(jets > 0 && detached_drops > 0);
                assert!(!blood.marks.is_empty());
                assert!(blood.drops.iter().all(|d| d.kind != DropKind::Severed));
                blood.clear();
                for _ in 0..30 {
                    game.step(0).unwrap();
                    blood.tick(&game, &models, &surfaces, &settings, deaths.frame.as_ref());
                    assert!(
                        blood.drops.is_empty(),
                        "clearing blood restarted severed bleeding"
                    );
                }
            }
        }
    }
}
