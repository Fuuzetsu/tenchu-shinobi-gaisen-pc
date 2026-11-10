use super::*;
use tenchu_core::{
    math::{Matrix, Vector3},
    tenchu2::{collision::BODY, environment::Context},
};

#[derive(Default)]
struct Sounds(Vec<u8>);

impl environment::Events for Sounds {
    fn emit(&mut self, event: environment::Event, _: &mut Random) -> Result<()> {
        if let environment::Event::Sound { id, .. } = event {
            self.0.push(id);
        }
        Ok(())
    }
}

fn advance(world: &mut World, assets: &Assets, events: &mut Sounds) {
    let mut context = Context {
        map: &mut world.terrain,
        cursor: &mut world.cursor,
        collision: &mut world.collision,
        scene: &mut world.scene,
        math: &assets.resources.math,
        random: &mut world.random,
        events,
    };
    world
        .environment
        .update_contacts(
            &assets.level.scene.bounds,
            (world.player, world.actors[world.player].actor.position),
            |_| true,
            &mut context,
        )
        .unwrap();
    world.environment.update_triggers(&mut context).unwrap();
    world.collision.compute(|m| world.scene.world(m)).unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; opens every original door through native body contacts, including missing siblings and snapshot continuation"]
fn imported_doors_open_with_native_contacts() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    let mut types = std::collections::BTreeSet::new();
    for stage in [0, 2, 3, 5, 6, 7, 9] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let (assets, layout) = Assets::gaisen(&disc, &source, 0, false).unwrap();
        let mut initial = World::new(
            &layout,
            Options {
                hero: 0,
                difficulty: 2,
                seed: 1,
                inventory: Default::default(),
            },
            &assets,
        )
        .unwrap();
        initial
            .environment
            .surface(
                environment::SurfaceActor {
                    index: initial.player,
                    position: Position::ZERO,
                    height: 1000,
                    health: 100,
                    visible: true,
                    focused: true,
                },
                &tenchu_core::tenchu2::terrain::Probe {
                    floor: Some(0),
                    material: tenchu_assets::tenchu2::collision::Gaisen::material(0x100),
                    ..Default::default()
                },
                false,
                &mut environment::SurfaceContext {
                    map: &mut initial.terrain,
                    cursor: &mut initial.cursor,
                    math: &assets.resources.math,
                    random: &mut initial.random,
                    events: &mut Sounds::default(),
                },
            )
            .unwrap();
        assert!(
            initial.environment.triggers.iter().all(|t| t.clock == 0),
            "ACM damage material must not activate an unrelated door"
        );
        for (index, spawn) in source
            .misc
            .spawns
            .iter()
            .filter(|s| matches!(s.kind, tenchu_assets::misc::Kind::Door { .. }))
            .enumerate()
        {
            let tenchu_assets::misc::Kind::Door { rotation, kind } = spawn.kind else {
                unreachable!()
            };
            types.insert(kind);
            count += 1;
            let leaves = [index * 2, index * 2 + 1];
            let mut world = initial.clone();
            world.collision = Default::default();
            for (side, &leaf) in leaves.iter().enumerate() {
                let object = &world.environment.objects[leaf];
                let source_model = source.misc.doors[kind].models[side];
                assert_eq!(object.model.is_some(), source_model.is_some());
                if let Some(model) = object.model {
                    assert_eq!(
                        assets.level.objects[model].geometry,
                        source.common_models[&source_model.unwrap()]
                    );
                    assert_eq!(
                        assets.object_sources[&model],
                        format!("common/{}", source_model.unwrap())
                    );
                    world
                        .environment
                        .enable_collision(
                            leaf,
                            &assets.level.scene.bounds,
                            &mut world.collision,
                            &assets.resources.math,
                        )
                        .unwrap();
                }
            }
            let touching = leaves
                .into_iter()
                .find(|&i| world.environment.objects[i].model.is_some())
                .unwrap();
            let center = world.scene.models[world.environment.objects[touching].coordinate]
                .local
                .translation;
            let body = world.scene.add(
                None,
                Matrix {
                    translation: center.wrapping_add(Position::new(100, 0, 100)),
                    ..Default::default()
                },
            );
            let slot = world.collision.insert(body).unwrap();
            slot.flags = BODY;
            slot.size = Vector3::new(100, 500, 100);
            slot.owner = Some(world.player);
            world.collision.compute(|m| world.scene.world(m)).unwrap();
            let mut sounds = Sounds::default();
            let mut snapshot = Vec::new();
            for tick in 0..40 {
                advance(&mut world, &assets, &mut sounds);
                if tick == 8 {
                    snapshot = postcard::to_allocvec(&world).unwrap();
                }
            }
            assert_eq!(sounds.0, [0x28], "stage {stage} door {index}");
            let trigger = assets.level.scene.dynamic[leaves[0]].trigger.unwrap();
            assert_eq!(world.environment.triggers[trigger].clock, 0);
            let mut turns = Vec::new();
            for leaf in leaves {
                let object = &world.environment.objects[leaf];
                assert!(world.collision.index(object.coordinate).is_none());
                let turn = object.yaw.wrapping_sub(rotation) & 4095;
                if object.model.is_some() {
                    // Retail updates on the terminal tick too: 33 steps, 1056 units.
                    assert!(
                        matches!(turn, 1056 | 3040),
                        "stage {stage} door {index}: {turn}"
                    );
                    turns.push(turn);
                    assert_eq!(object.kind, -1);
                } else {
                    assert_eq!(turn, 0);
                }
            }
            if turns.len() == 2 {
                assert_eq!((turns[0] + turns[1]) & 4095, 0);
            }
            let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
            for _ in 9..40 {
                advance(&mut restored, &assets, &mut Sounds::default());
            }
            assert_eq!(
                postcard::to_allocvec(&restored).unwrap(),
                postcard::to_allocvec(&world).unwrap(),
                "stage {stage} door {index} snapshot"
            );
        }
    }
    assert_eq!(types, (0..11).collect());
    eprintln!(
        "Opened {count} imported doors, all 11 source templates; body contacts and snapshot continuation agree"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; captures single and paired original doors before and after native contact opening"]
fn imported_door_rendering() {
    let Some(output) = std::env::var_os("TENCHU_GAISEN_REVIEW") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir_all(&output).unwrap();
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for stage in [0, 2] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let mut session = Session::gaisen(
            &disc,
            &source,
            Options {
                hero: 0,
                difficulty: 2,
                seed: 1,
                inventory: Default::default(),
            },
            false,
        )
        .unwrap();
        let spawn = source
            .misc
            .spawns
            .iter()
            .find(|s| matches!(s.kind, tenchu_assets::misc::Kind::Door { .. }))
            .unwrap();
        let tenchu_assets::misc::Kind::Door { rotation, .. } = spawn.kind else {
            unreachable!()
        };
        let mut camera = crate::debug::camera::Camera {
            position: glam::Vec3::from_array(spawn.position.map(|v| v as f32)),
            yaw: f32::from(rotation) * std::f32::consts::TAU / 4096.0,
            pitch: 0.0,
            fov: 45.0,
        };
        camera.position -= camera.forward() * 6500.0;
        camera.position.y -= 1500.0;
        session.world.camera.view = camera.source_view();
        session.world.camera.eye = camera.position.to_array().map(|v| v as i32).into();
        session.world.camera.look = spawn.position.into();
        session.world.actors[session.world.player].actor.position = spawn.position.into();
        for opened in [false, true] {
            if opened {
                session.world.collision = Default::default();
                for leaf in 0..2 {
                    if session.world.environment.objects[leaf].model.is_some() {
                        session
                            .world
                            .environment
                            .enable_collision(
                                leaf,
                                &session.assets.level.scene.bounds,
                                &mut session.world.collision,
                                &session.assets.resources.math,
                            )
                            .unwrap();
                    }
                }
                let leaf = session
                    .world
                    .environment
                    .objects
                    .iter()
                    .take(2)
                    .find(|o| o.model.is_some())
                    .unwrap();
                let body = session.world.scene.add(
                    None,
                    Matrix {
                        translation: session.world.scene.models[leaf.coordinate]
                            .local
                            .translation
                            .wrapping_add(Position::new(100, 0, 100)),
                        ..Default::default()
                    },
                );
                let slot = session.world.collision.insert(body).unwrap();
                slot.flags = BODY;
                slot.size = Vector3::new(100, 500, 100);
                slot.owner = Some(session.world.player);
                session
                    .world
                    .collision
                    .compute(|m| session.world.scene.world(m))
                    .unwrap();
                for _ in 0..40 {
                    advance(&mut session.world, &session.assets, &mut Sounds::default());
                }
            }
            session.frame.scenery = session
                .world
                .update_environment(&session.assets, &mut Sounds::default())
                .unwrap();
            let drawn = session
                .frame
                .scenery
                .models
                .iter()
                .filter(|m| matches!(m.source, environment::scenery::Source::Object(0 | 1)))
                .count();
            assert_eq!(drawn, if stage == 0 { 2 } else { 1 });
            render::capture(
                &session,
                &crate::settings::Settings::default(),
                &output.join(format!(
                    "door-stage-{stage}-{}.png",
                    if opened { "open" } else { "closed" }
                )),
            )
            .unwrap();
        }
    }
}
