use super::{
    doors::{Sounds, advance},
    *,
};
use tenchu_assets::misc::Kind;
use tenchu_core::{
    math::{Matrix, Vector3},
    tenchu2::{
        collision::BODY,
        environment::{GAISEN_OPEN_PITFALL, GAISEN_PITFALL},
    },
};

fn options(hero: usize) -> Options {
    Options {
        hero,
        difficulty: 2,
        seed: 1,
        inventory: Default::default(),
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; checks all original pitfall placements, paired and absent leaves, terrain preservation and snapshot continuation"]
fn imported_pitfall_contacts_preserve_authored_holes() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    for stage in [2, 5, 6] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let (assets, layout) = Assets::gaisen(&disc, &source, 0, false).unwrap();
        let initial = World::new(&layout, options(0), &assets).unwrap();
        for (index, spawn) in source
            .misc
            .spawns
            .iter()
            .filter(|s| matches!(s.kind, Kind::Door { .. } | Kind::Pitfall { .. }))
            .enumerate()
        {
            let Kind::Pitfall { rotation, kind } = spawn.kind else {
                continue;
            };
            count += 1;
            let mut world = initial.clone();
            world.collision = Default::default();
            let terrain = postcard::to_allocvec(&world.terrain).unwrap();
            let leaves = [2 * index, 2 * index + 1];
            for (side, &leaf) in leaves.iter().enumerate() {
                let object = &world.environment.objects[leaf];
                assert_eq!(
                    object.model.is_some(),
                    source.misc.pitfalls[kind].models[side].is_some()
                );
                if object.model.is_some() {
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
            let coordinate = world.environment.objects[leaves[0]].coordinate;
            let body = world.scene.add(
                None,
                Matrix {
                    translation: world.scene.models[coordinate].local.translation,
                    ..Default::default()
                },
            );
            let slot = world.collision.insert(body).unwrap();
            slot.flags = BODY;
            slot.owner = Some(world.player);
            slot.size = Vector3::new(100, 500, 100);
            world.collision.compute(|m| world.scene.world(m)).unwrap();
            let mut sounds = Sounds::default();
            let mut snapshot = Vec::new();
            for tick in 0..12 {
                advance(&mut world, &assets, &mut sounds);
                if tick == 2 {
                    snapshot = postcard::to_allocvec(&world).unwrap();
                }
            }
            assert_eq!(sounds.0, [0x28]);
            for (side, &leaf) in leaves.iter().enumerate() {
                let object = &world.environment.objects[leaf];
                assert!(world.collision.index(object.coordinate).is_none());
                assert_eq!(object.yaw, rotation);
                if object.model.is_some() {
                    assert_eq!(object.kind, GAISEN_OPEN_PITFALL);
                    let model = environment::scenery::Model::object(leaf, object).unwrap();
                    assert_eq!(
                        model.rotation,
                        [0, rotation, if side == 0 { 1024 } else { -1024 }]
                    );
                }
            }
            let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
            for _ in 3..12 {
                advance(&mut restored, &assets, &mut Sounds::default());
            }
            assert_eq!(
                postcard::to_allocvec(&world).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            assert_eq!(postcard::to_allocvec(&world.terrain).unwrap(), terrain);
        }
    }
    assert_eq!(count, 11);
    eprintln!(
        "All 11 source pitfalls open without changing authored ACM or native collision floors; snapshots agree"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; drops every native hero through each T1 trap template and compares native physics with the visual mechanism absent"]
fn imported_pitfalls_leave_native_falling_unchanged() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (stage, record) in [(2, 43), (2, 45), (6, 2)] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let spawn = source
            .misc
            .spawns
            .iter()
            .find(|s| s.record == record)
            .unwrap();
        let Kind::Pitfall { rotation, .. } = spawn.kind else {
            unreachable!()
        };
        for hero in 0..3 {
            let mut session = Session::gaisen(&disc, &source, options(hero), false).unwrap();
            session
                .world
                .teleport_player(
                    Position::from(spawn.position).wrapping_sub(Position::new(0, 300, 0)),
                    false,
                    &session.assets,
                )
                .unwrap();
            session.frame.scenery = session
                .world
                .update_environment(&session.assets, &mut Sounds::default())
                .unwrap();
            let mut baseline = session.world.clone();
            for object in &mut baseline.environment.objects {
                if object.kind == GAISEN_PITFALL {
                    object.model = None;
                    baseline.collision.remove(object.coordinate);
                }
            }
            baseline
                .collision
                .compute(|m| baseline.scene.world(m))
                .unwrap();
            session
                .world
                .collision
                .compute(|m| session.world.scene.world(m))
                .unwrap();
            if stage == 2 && hero == 0 {
                capture(&mut session, spawn.position, rotation, record, false);
            }
            let terrain = postcard::to_allocvec(&session.world.terrain).unwrap();
            let mut snapshots = None;
            for tick in 0..60 {
                session.frame = session
                    .world
                    .step([0; 2], &mut session.assets, &mut Sounds::default())
                    .unwrap();
                baseline
                    .step([0; 2], &mut session.assets, &mut Sounds::default())
                    .unwrap();
                let player = &session.world.actors[session.world.player].actor;
                let reference = &baseline.actors[baseline.player].actor;
                assert_eq!(
                    (
                        player.position,
                        player.velocity,
                        player.health,
                        player.flags
                    ),
                    (
                        reference.position,
                        reference.velocity,
                        reference.health,
                        reference.flags
                    ),
                    "stage {stage} record {record} hero {hero} tick {tick}"
                );
                if tick == 3 {
                    snapshots = Some(postcard::to_allocvec(&session.world).unwrap());
                }
                if tick == 10 && stage == 2 && hero == 0 {
                    capture(&mut session, spawn.position, rotation, record, true);
                }
            }
            let player = &session.world.actors[session.world.player].actor;
            assert!(
                player.position.y > spawn.position[1] + 2000,
                "stage {stage} record {record} hero {hero} did not fall through: {:?}",
                player.position
            );
            if stage == 2 {
                assert_eq!(player.health, 0);
            }
            let opened = session
                .world
                .environment
                .objects
                .iter()
                .filter(|o| o.kind == GAISEN_OPEN_PITFALL && o.roll.abs() == 1024)
                .count();
            assert!(
                opened > 0,
                "stage {stage} record {record} hero {hero}: no opened trap"
            );
            assert_eq!(
                postcard::to_allocvec(&session.world.terrain).unwrap(),
                terrain
            );
            let mut restored: World = postcard::from_bytes(&snapshots.unwrap()).unwrap();
            for _ in 4..60 {
                restored
                    .step([0; 2], &mut session.assets, &mut Sounds::default())
                    .unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&session.world).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            eprintln!(
                "T1 stage {stage} trap {record}, T2 hero {hero}: {opened} leaves open; native fall {:?}, HP {}",
                player.position, player.health
            );
        }
    }
}

fn capture(session: &mut Session, position: [i32; 3], rotation: i16, record: usize, opened: bool) {
    let Some(output) = std::env::var_os("TENCHU_GAISEN_REVIEW") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir_all(&output).unwrap();
    let world = session.world.clone();
    let frame = std::mem::take(&mut session.frame);
    let mut camera = crate::debug::camera::Camera {
        position: glam::Vec3::from_array(position.map(|v| v as f32)),
        yaw: f32::from(rotation) * std::f32::consts::TAU / 4096.0,
        pitch: -1.05,
        fov: 45.0,
    };
    camera.position -= camera.forward() * 6500.0;
    session.world.camera.view = camera.source_view();
    session.world.camera.eye = camera.position.to_array().map(|v| v as i32).into();
    session.world.camera.look = position.into();
    session.world.actors[session.world.player].actor.position = position.into();
    session.frame.scenery = session
        .world
        .update_environment(&session.assets, &mut Sounds::default())
        .unwrap();
    let leaves: Vec<_> = session
        .frame
        .scenery
        .models
        .iter()
        .filter(|m| {
            let environment::scenery::Source::Object(index) = m.source else {
                return false;
            };
            let object = &session.world.environment.objects[index];
            matches!(object.kind, GAISEN_PITFALL | GAISEN_OPEN_PITFALL)
                && session.world.environment.triggers[object.link as usize].position
                    == Position::from(position)
        })
        .collect();
    assert_eq!(leaves.len(), if record == 43 { 1 } else { 2 });
    assert!(leaves.iter().all(|m| (m.rotation[2] != 0) == opened));
    render::capture(
        session,
        &crate::settings::Settings::default(),
        &output.join(format!(
            "pitfall-{record}-{}.png",
            if opened { "open" } else { "closed" }
        )),
    )
    .unwrap();
    session.world = world;
    session.frame = frame;
}
