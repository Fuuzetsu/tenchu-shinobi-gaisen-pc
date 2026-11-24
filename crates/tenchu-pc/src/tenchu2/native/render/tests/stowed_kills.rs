use super::*;
use crate::visuals::interpolation::{History, Part as PartId};

fn parts(
    session: &super::super::super::Session,
    models: &Models,
    pose: Option<&PoseFrame>,
) -> Result<BTreeMap<PartId, (usize, Matrix)>> {
    let mut parts = BTreeMap::new();
    models.visit_actors(
        &session.world,
        &session.assets,
        &mut session.world.scene.clone(),
        pose,
        |key, _, part, matrix, _| {
            parts.insert(key, (part.geometry.as_ptr() as usize, matrix));
        },
    )?;
    Ok(parts)
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; source counter phases, interpolated holsters, recovery and matched captures"]
fn native_counter_kills_stow_only_unarmed_phases() {
    use tenchu_core::tenchu2::{actor::Velocity, input::ATTACK};
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for hero in 0..2 {
        let (mut session, victim) = super::super::super::dismemberment_tests::fixture(hero);
        session.world.actors[victim]
            .actor
            .change_motion(0x700, Velocity::Stop, &session.assets.resources)
            .unwrap();
        let events = session
            .assets
            .resources
            .registry
            .stealth_events(hero, 0)
            .unwrap();
        use tenchu_assets::tenchu2::registry::StealthEffect;
        if hero == 0 {
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.frame < 100 && matches!(e.effect, StealthEffect::AttackerCue(14)))
                    .map(|e| e.frame)
                    .collect::<Vec<_>>(),
                [16, 41, 66]
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.frame == 130 && matches!(e.effect, StealthEffect::Blood { .. }))
            );
        } else {
            assert!(
                events
                    .iter()
                    .all(|e| !matches!(e.effect, StealthEffect::Blood { .. }))
            );
        }
        session.world.publish(victim, &session.assets);
        session.step_world(ATTACK, true).unwrap();
        let player = session.world.player;
        assert_eq!(session.world.actors[player].actor.motion.action, 0x926);
        let camera = crate::demo::framing::Path {
            eye: [3000.0, -1700.0, 2800.0],
            target: [0.0, -850.0, -500.0],
            end_eye: None,
            end_target: None,
            follow_player: false,
            fov: 42.0,
        }
        .sample(0.0);
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        let mut history = History::default();
        history.reset_to(interpolation::gameplay(&session.world, &session.assets).unwrap());
        let mut checked = 0;
        for _ in 0..220 {
            let previous_scene = session.world.scene.clone();
            session.step_world(0, true).unwrap();
            history.record(interpolation::gameplay(&session.world, &session.assets).unwrap());
            let actor = &session.world.actors[player].actor;
            if actor.motion.action != 0x926 {
                break;
            }
            let frame = actor.motion.frame;
            if ![45, 66, 67, 130].contains(&frame) {
                continue;
            }
            checked += 1;
            let before = postcard::to_allocvec(&session.world).unwrap();
            let mut images = Vec::new();
            for enabled in [false, true] {
                graphics.models.stow_unarmed_kills = enabled;
                let path = output.join(format!("native-counter-{hero}-frame{frame}-{enabled}.png"));
                capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                    let mut models =
                        Models::new(&session.assets, &session.world, &graphics, renderer)?;
                    for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
                        let pose = history.sample(u64::from(session.world.tick), alpha);
                        models.stow_unarmed_kills = false;
                        let original = parts(&session, &models, pose.as_ref())?;
                        models.stow_unarmed_kills = true;
                        let stowed = parts(&session, &models, pose.as_ref())?;
                        let affected = hero == 1 || frame <= 66;
                        if affected {
                            assert!(original != stowed);
                            let rig = &session.world.instances[player].rig;
                            for (key, value) in &original {
                                let weapon = matches!(key, PartId::Actor {root, model, ..}
                                    if *root == rig.root && !rig.joints.contains(model));
                                if !weapon {
                                    assert_eq!(
                                        stowed.get(key),
                                        Some(value),
                                        "body or victim changed"
                                    );
                                }
                            }
                            let mut weapons = rig.weapons;
                            weapons.swap(0, 3);
                            if hero == 1 {
                                weapons.swap(1, 4);
                            }
                            let mut actual = 0;
                            for root in weapons[..3].iter().flatten() {
                                let count = session.assets.weapons[&models.weapon_names[root]].len();
                                for index in 0..count {
                                    let model = root + 1 + index;
                                    let key = interpolation::part(
                                        rig.root,
                                        model,
                                        actor.definition_kind(),
                                    );
                                    assert!(stowed.contains_key(&key));
                                    if alpha == 0.0 || alpha == 1.0 {
                                        let mut source = if alpha == 0.0 {
                                            previous_scene.clone()
                                        } else {
                                            session.world.scene.clone()
                                        };
                                        assert_eq!(
                                            stowed[&key].1,
                                            source.world(model)?,
                                            "holster interpolation endpoint changed its authored mount"
                                        );
                                    }
                                    actual += 1;
                                }
                            }
                            assert!(actual > 0, "stowed grips disappeared");
                        } else {
                            assert!(original == stowed, "sword finish changed");
                        }
                    }
                    models.stow_unarmed_kills = enabled;
                    let mut geometry = models.geometry(
                        &session.world,
                        &session.assets,
                        &session.frame,
                        &graphics,
                        Presentation {
                            camera: Some(camera),
                            ..Default::default()
                        },
                    )?;
                    // The isolated source fixture uses a flat collision floor; omit unrelated scenery.
                    geometry.terrain.clear();
                    geometry.props.clear();
                    geometry.backgrounds.clear();
                    geometry.effects.vertices.clear();
                    geometry.effects.batches.clear();
                    Ok((
                        SceneDrawing::new(geometry, renderer, &session.assets.level),
                        camera.source_view(),
                        Vec::new(),
                    ))
                })
                .unwrap();
                images.push(std::fs::read(path).unwrap());
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
            }
            if hero == 1 || frame <= 66 {
                assert_ne!(images[0], images[1]);
            } else {
                assert_eq!(images[0], images[1]);
            }
            // Resuming the same source frame must make the same presentation choice.
            session.world = postcard::from_bytes(&before).unwrap();
        }
        assert_eq!(checked, 4);
        assert_ne!(session.world.actors[player].actor.motion.action, 0x926);
        assert!(session.world.actors[player].actor.weapon_drawn());
        assert_eq!(session.world.actors[victim].actor.health, 0);
        assert_eq!(session.world.statistics.stealth_kills, 1);
    }
}

#[test]
#[ignore = "requires both discs and GPU; native rear kills/Tatsumaru stay unchanged and mixed T1 equipment uses the actual source holsters"]
fn native_stowed_kills_preserve_other_styles_and_imported_mounts() {
    use crate::{settings::Settings, visuals::appearance::library::Library};
    use tenchu_core::tenchu2::{actor::Velocity, input::ATTACK, rig::WeaponMode};
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&gaisen).unwrap();
    let mut donor = crate::tenchu2::native::models::donor(&gaisen).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    for (hero, body, counter) in [
        (0, "", false),
        (1, "", false),
        (2, "", false),
        (2, "", true),
        (0, "disc:1", true),
        (1, "disc:0", true),
    ] {
        let (mut session, victim) = super::super::super::dismemberment_tests::fixture(hero);
        if !body.is_empty() {
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Default::default(),
                    &Settings {
                        player_model: body.into(),
                        weapon_model: body.into(),
                        model_moveset: false,
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        if counter {
            session.world.actors[victim]
                .actor
                .change_motion(0x700, Velocity::Stop, &session.assets.resources)
                .unwrap();
            session.world.publish(victim, &session.assets);
        }
        session.step_world(ATTACK, true).unwrap();
        let player = session.world.player;
        assert_eq!(
            session.world.actors[player].actor.motion.action,
            // Away from map12, source Tatsumaru converts Drawing to Back.
            if counter && hero != 2 { 0x926 } else { 0x921 },
            "hero {hero}, body {body}, counter {counter}"
        );
        for _ in 0..44 {
            session.step_world(0, true).unwrap();
        }
        let before = postcard::to_allocvec(&session.world).unwrap();
        let mut renderer = pollster::block_on(Renderer::new(
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            [64, 64],
            &session.assets.sprites.vram,
        ))
        .unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        let mut models =
            Models::new(&session.assets, &session.world, &graphics, &mut renderer).unwrap();
        let original = parts(&session, &models, None).unwrap();
        models.stow_unarmed_kills = true;
        let stowed = parts(&session, &models, None).unwrap();
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
        if body.is_empty() {
            assert!(
                original == stowed,
                "unrelated hero {hero} style {counter} changed"
            );
        } else {
            assert!(original != stowed, "mixed equipment did not stow");
            // Use the core's real equip operation as the attachment oracle; restore it
            // immediately afterward, since the option must never change gameplay state.
            session.world.actors[player]
                .actor
                .set_weapon(
                    &mut session.world.instances[player].rig,
                    player,
                    &mut session.world.collision,
                    &session.assets.resources,
                    WeaponMode::Sheathe,
                )
                .unwrap();
            models.stow_unarmed_kills = false;
            assert!(
                parts(&session, &models, None).unwrap() == stowed,
                "mixed body {body} does not use the source holsters"
            );
            session.world = postcard::from_bytes(&before).unwrap();
        }
    }
}
