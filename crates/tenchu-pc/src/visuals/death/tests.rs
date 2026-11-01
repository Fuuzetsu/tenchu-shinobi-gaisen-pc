use super::*;
use crate::{
    render_mesh,
    settings::{BloodStyle, Graphics},
    visuals::{
        appearance, blood::Blood, interpolation::History, models::Models, surfaces::Surfaces,
    },
};
use glam::Vec3;

fn assets() -> GameAssets<crate::source::DiscSource> {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap()
}

fn kill(game: &mut Game, action: i16) {
    let player = game.roster.get_mut(game.player_id);
    player.health = 0;
    player.status = State::Dead;
    player.motion.change(
        action,
        &game.mission.characters[player.definition].motions,
        &game.mission.common_motions,
        &game.mission.clips,
    );
    let clip = &game.mission.clips[player.motion.clip.unwrap()];
    player.velocity = game.math.velocity(
        player.rotation[1],
        i16::from(clip.forward_speed),
        i16::from(clip.side_speed),
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; both rigs, full death poses, attached weapons, interpolated poses and centered pools without changing simulation"]
fn ninja_deaths_keep_gameplay_and_attached_visuals() {
    let assets = assets();
    let output = std::env::var_os("TENCHU_DEATH_CAPTURES").map(std::path::PathBuf::from);
    for character in 0..2 {
        for variant in 0..4 {
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
            let graphics = Graphics {
                blood: crate::settings::Blood {
                    hit: BloodStyle::Off,
                    drips: BloodStyle::Off,
                    ground: BloodStyle::Off,
                    death: BloodStyle::Modern,
                    player_death: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
            let surfaces = Surfaces::new(&render_mesh::stage(
                &game.mission,
                &game.math,
                Some(&models),
            ));
            let mut blood = Blood::default();
            let mut deaths = Deaths::load(&assets).unwrap();
            let mut original = Deaths::load(&assets).unwrap();
            assert_eq!(deaths.clips.len(), 4);
            assert_eq!(
                deaths
                    .clips
                    .iter()
                    .map(|c| c.id)
                    .collect::<std::collections::BTreeSet<_>>(),
                [343, 345, 346, 473].into_iter().collect()
            );
            let mut history = History::default();
            let definition = &game
                .mission
                .characters
                .iter()
                .find(|c| c.kind == if character == 0 { 0x21 } else { 0x22 })
                .unwrap();
            let ninja = tenchu_assets::model::archive(
                &assets
                    .asset(&format!("HUMAN/{}.MAD", definition.model))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                appearance::parents(&ninja),
                appearance::parents(&game.mission.models[&character])
            );
            blood.tick(&game, &models, &surfaces, &graphics.blood, None);
            kill(&mut game, MOT_DEAD);
            if game.mission.clips[game.player().motion.clip.unwrap()].id == deaths.clips[variant].id
            {
                continue;
            }
            for seed in 1..1000 {
                game.random.seed = seed;
                deaths.reset();
                deaths.tick(&game, 100).unwrap();
                if deaths.death.as_ref().unwrap().motion.as_ref().unwrap().clip == Some(variant) {
                    break;
                }
            }
            assert_eq!(
                deaths.death.as_ref().unwrap().motion.as_ref().unwrap().clip,
                Some(variant)
            );
            assert_ne!(
                game.mission.clips[game.player().motion.clip.unwrap()].id,
                deaths.clips[variant].id
            );
            let end = deaths.clips[variant].duration - i16::from(deaths.clips[variant].sweep);
            let mut settled = None;
            let mut original_ready = false;
            let mut original_late = 0;
            let mut replaced_late = 0;
            for step in 0..310 {
                game.step(0).unwrap();
                let before = (step % 50 == 0).then(|| crate::session::tests::state(&game));
                original.tick(&game, 0).unwrap();
                assert!(original.frame.is_none());
                deaths.tick(&game, if step == 0 { 100 } else { 0 }).unwrap();
                let frame = deaths.frame.as_ref().unwrap();
                if step > end + 80 {
                    assert!(blood::slots(&game.effects, Some(frame)).skip(200).all(|effect| {
                        !matches!(effect, Some(tenchu_core::effects::Effect::Blood(b)) if b.mode == 0)
                            && !matches!(effect, Some(tenchu_core::effects::Effect::Bleed(_)))
                    }), "spurts must end with the selected collapse");
                }
                history.tick(&game);
                history.present(game.tick, frame);
                blood.tick(&game, &models, &surfaces, &graphics.blood, Some(frame));
                if game.player().motion.loops < 0 {
                    original_ready = true;
                }
                let geometry = blood.geometry(&game, &models, Some(frame), None, 1.0);
                assert_eq!(
                    geometry.vertices.is_empty(),
                    deaths.death.as_ref().unwrap().settled.is_none(),
                    "hero {character}, variant {variant}, step {step}: pool must follow the visible collapse"
                );
                let sounds = blood::sounds(&game.effects, Some(frame)).collect::<Vec<_>>();
                assert_eq!(
                    game.sounds().unwrap(),
                    game.sounds_with_effects(blood::sounds(&game.effects, None))
                        .unwrap()
                );
                if step > end + 100 {
                    original_late += game
                        .effects
                        .gore_sounds
                        .iter()
                        .filter(|(_, source)| *source == game.player().parts[1])
                        .count();
                    replaced_late += sounds.iter().filter(|(_, sound)| *sound == 0x37).count();
                }
                if step > end {
                    let poses = deaths
                        .death
                        .as_ref()
                        .unwrap()
                        .motion
                        .as_ref()
                        .unwrap()
                        .pose
                        .matrices(&game.math, deaths.death.as_ref().unwrap().settled.unwrap());
                    for (&model, expected) in game.player().parts.iter().zip(&poses) {
                        assert_eq!(
                            Frame::transform(
                                Some(frame),
                                Part::Actor {
                                    root: game.player().root_model,
                                    model,
                                    kind: game.player().mesh_kind,
                                    inspection: true,
                                },
                                Default::default()
                            ),
                            *expected
                        );
                    }
                    if let Some(previous) = &settled {
                        assert_eq!(*previous, poses);
                    }
                    settled = Some(poses);
                }
                let player = game.player();
                let part = |model, kind, inspection| Part::Actor {
                    root: player.root_model,
                    model,
                    kind,
                    inspection,
                };
                for inspection in [false, true] {
                    for weapon in player.weapons.iter().flatten() {
                        let parent = game.scene.models[weapon.model].parent.unwrap();
                        let hand = Frame::transform(
                            Some(frame),
                            part(parent, player.mesh_kind, inspection),
                            Default::default(),
                        );
                        let weapon_pose = Frame::transform(
                            Some(frame),
                            part(weapon.model, weapon.kind, inspection),
                            Default::default(),
                        );
                        assert_eq!(
                            weapon_pose,
                            hand.compose(&game.scene.models[weapon.model].local)
                        );
                    }
                }
                if let Some(interpolated) = history.sample(game.tick, 0.5) {
                    let vertices = render_mesh::characters_presented(
                        &game,
                        Some(&models),
                        true,
                        Some(&interpolated),
                    );
                    assert!(!vertices.is_empty());
                    assert!(vertices.iter().flat_map(|v| v.position).all(f32::is_finite));
                }
                if let Some(before) = before {
                    assert!(crate::session::tests::state(&game) == before);
                }
                if [25, 60, 300].contains(&step)
                    && let Some(output) = &output
                {
                    std::fs::create_dir_all(output).unwrap();
                    for alternate in [false, true] {
                        capture(
                            &game,
                            &models,
                            &blood,
                            alternate.then_some(frame),
                            &output.join(format!(
                                "hero-{character}-variant-{variant}-tick-{step}-{}.png",
                                if alternate { "ninja" } else { "original" }
                            )),
                        );
                    }
                }
            }
            assert!(original_ready);
            if end < 100 {
                assert!(
                    original_late > 0,
                    "fixture must expose original-duration gore sounds"
                );
                assert_eq!(
                    replaced_late, 0,
                    "short collapse must stop its own gore sounds"
                );
            }
            let player = game.player();
            let torso = Frame::transform(
                deaths.frame.as_ref(),
                Part::Actor {
                    root: player.root_model,
                    model: player.parts[0],
                    kind: player.mesh_kind,
                    inspection: true,
                },
                Default::default(),
            )
            .translation;
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for page in blood.pages() {
                for v in page.iter() {
                    min = min.min(Vec3::from(v.position));
                    max = max.max(Vec3::from(v.position));
                }
            }
            let center = (min + max) * 0.5;
            assert!(center.is_finite(), "death pool must be present");
            assert!(
                (center.x - torso.x as f32).abs() < 100.0
                    && (center.z - torso.z as f32).abs() < 100.0
            );
            game.roster.get_mut(game.player_id).health = 100;
            deaths.tick(&game, 100).unwrap();
            assert!(deaths.frame.is_none());
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; a completed visual collapse settles on landing, stays put, and does not restart when recreating the view"]
fn alternate_corpse_settles_only_on_the_ground() {
    let assets = assets();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    kill(&mut game, MOT_DEAD);
    let mut deaths = Deaths::load(&assets).unwrap();
    deaths.tick(&game, 100).unwrap();
    game.tick += 250;
    let player = game.roster.get_mut(game.player_id);
    player.position.y -= 500;
    player.map.height = 500;
    player.velocity[1] = 20;
    deaths.tick(&game, 100).unwrap();
    assert!(deaths.death.as_ref().unwrap().settled.is_none());
    game.tick += 1;
    let player = game.roster.get_mut(game.player_id);
    player.position.y += 500;
    player.map.height = 0;
    player.velocity[1] = 0;
    deaths.tick(&game, 100).unwrap();
    let root = deaths.death.as_ref().unwrap().settled.unwrap();
    assert_eq!(root.translation.y, game.player().position.y);
    let key = Part::Actor {
        root: game.player().root_model,
        model: game.player().parts[0],
        kind: game.player().mesh_kind,
        inspection: true,
    };
    let settled = Frame::transform(deaths.frame.as_ref(), key, Default::default());
    game.tick += 1;
    let player = game.roster.get_mut(game.player_id);
    player.position.x += 100;
    player.position.y -= 500;
    player.rotation[1] += 100;
    player.map.height = 500;
    player.velocity[1] = 20;
    deaths.tick(&game, 100).unwrap();
    assert_eq!(
        Frame::transform(deaths.frame.as_ref(), key, Default::default()),
        settled
    );
    let player = game.roster.get_mut(game.player_id);
    player.position.y += 500;
    player.map.height = 0;
    player.velocity = [0; 3];
    player.motion.loops = -1;
    player.motion.count = 0;
    deaths.reset();
    deaths.tick(&game, 100).unwrap();
    assert!(deaths.death.as_ref().unwrap().settled.is_some());
}

fn capture(
    game: &Game,
    models: &Models,
    blood: &Blood,
    frame: Option<&Frame>,
    path: &std::path::Path,
) {
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let stage =
        review
            .renderer
            .upload(&render_mesh::stage(&game.mission, &game.math, Some(models)));
    let actors = review.renderer.upload(&render_mesh::characters_presented(
        game,
        Some(models),
        true,
        frame,
    ));
    let position = Vec3::from(game.player().position.map(|v| v as f32).to_array());
    let eye = position + Vec3::new(1700.0, -1800.0, 2200.0);
    let delta = position - Vec3::Y * 500.0 - eye;
    let camera = crate::debug::camera::Camera {
        position: eye,
        yaw: delta.x.atan2(delta.z),
        pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
        fov: 55.0,
    };
    let mut geometry = crate::render_effects::build_presented(
        game,
        false,
        Some(camera.source_view()),
        false,
        Some(models),
        frame,
        crate::render_effects::Style {
            blood: crate::settings::Blood::modern(),
            ..Default::default()
        },
    );
    geometry.append(blood.geometry(game, models, frame, None, 1.0));
    let effects = review.renderer.upload_effects(geometry);
    review
        .capture(
            camera.matrix(1280.0 / 720.0),
            &[&stage, &actors],
            Some(&effects),
            false,
            path,
        )
        .unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC; stable per-death selection, defaults and protected death types"]
fn death_choice_is_stable_and_special_deaths_keep_their_motions() {
    let assets = assets();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    game.step(0).unwrap();
    kill(&mut game, MOT_DEAD);
    let mut deaths = Deaths::load(&assets).unwrap();
    let mut chosen = 0;
    let mut variants = std::collections::BTreeMap::<i16, usize>::new();
    for seed in 1..=1000 {
        game.random.seed = seed;
        deaths.reset();
        deaths.tick(&game, 50).unwrap();
        let selected = deaths.frame.is_some();
        chosen += usize::from(selected);
        if let Some(motion) = &deaths.death.as_ref().unwrap().motion {
            *variants
                .entry(deaths.clips[motion.clip.unwrap()].id)
                .or_default() += 1;
        }
        game.random.seed = seed + 1;
        deaths.tick(&game, if selected { 0 } else { 100 }).unwrap();
        assert_eq!(deaths.frame.is_some(), selected);
        assert_eq!(game.random.seed, seed + 1);
    }
    assert!(
        (450..550).contains(&chosen),
        "50% should produce both variants: {chosen}"
    );
    assert_eq!(variants.len(), 3);
    assert!(
        variants.values().all(|&count| (100..=220).contains(&count)),
        "{variants:?}"
    );
    for action in [
        MOT_DEAD_DROWN,
        MOT_DAMAGE_LAUNCH_BACK,
        MOT_DEAD_STEALTH_BACK,
    ] {
        kill(&mut game, action);
        deaths.reset();
        deaths.tick(&game, 100).unwrap();
        assert!(deaths.frame.is_none());
    }
    kill(&mut game, MOT_DEAD);
    for mode in 0..4 {
        deaths.reset();
        game.cinematic.active = mode == 0;
        game.roster.get_mut(game.player_id).scripted = mode == 1;
        game.moveset.kind = (mode == 2).then_some(0x80);
        game.roster.get_mut(game.player_id).map.height = if mode == 3 { 2000 } else { 0 };
        deaths.tick(&game, 100).unwrap();
        assert!(deaths.frame.is_none());
    }
}
