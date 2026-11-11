use super::native_world::{Recorded, fixture};
use tenchu_assets::terrain::Node;
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{
        actor::{State, Velocity},
        ai::{ForcedInput, Program, sight::Visibility},
        inventory::Item,
        rig::WeaponMode,
        world::{Assets, Placement, World},
    },
};

fn ready(world: &mut World, assets: &Assets, guard: usize) {
    let agent = &mut world.actors[guard];
    agent.actor.flags = 0x12;
    agent.actor.target = Some(world.player);
    agent.actor.position = Position::new(0, 0, -6000);
    agent.actor.previous = agent.actor.position;
    agent.actor.rotation.y = 2048;
    agent.actor.pad = Default::default();
    agent
        .actor
        .change_motion(0x100, Velocity::Stop, &assets.resources)
        .unwrap()
        .unwrap();
    agent
        .actor
        .set_weapon(
            &mut world.instances[guard].rig,
            guard,
            &mut world.collision,
            &assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    agent.sight.distance = 6000;
    agent.sight.angle = 0;
    agent.sight.player_angle = 0;
    agent.surroundings = Default::default();
    world.ai.visibility = Visibility::Unchanged;
    world.publish(guard, assets);
}

fn aiming(world: &mut World, assets: &Assets) {
    let actor = &mut world.actors[world.player].actor;
    actor.position = Position::ZERO;
    actor.previous = actor.position;
    actor.rotation.y = 0;
    actor.inventory.active = Item::try_from(4).unwrap();
    actor.inventory.selected = Some(actor.inventory.active);
    actor
        .change_motion(0x802, Velocity::Stop, &assets.resources)
        .unwrap()
        .unwrap();
    world.publish(world.player, assets);
}

fn armed_fixture() -> (World, Assets, usize) {
    let (mut world, assets, guard) = fixture();
    world.script = 0;
    world.tick = 10;
    world.actors[guard]
        .behavior
        .assign(0x1113, world.map, world.difficulty, &mut world.random)
        .unwrap();
    ready(&mut world, &assets, guard);
    aiming(&mut world, &assets);
    (world, assets, guard)
}

fn wall(world: &mut World, bounds: [i16; 4]) {
    world.terrain.cells[0].nodes.insert(
        0,
        Node {
            y: 0,
            dy: i16::MAX,
            bounds,
            attributes: 0x100,
            division: u16::MAX,
        },
    );
    world.cursor = Default::default();
}

fn decide(world: &mut World, assets: &Assets, actor: usize) -> u16 {
    world
        .decide_actor(actor, [0; 2], assets, &mut Recorded::default())
        .unwrap()
}

fn imported_terrain(world: &mut World) {
    let mut nodes: Vec<_> = world.terrain.cells[0]
        .nodes
        .iter()
        .copied()
        .map(|mut node| {
            node.attributes = if node.attributes & 0x100 != 0 {
                0x2003
            } else {
                0x2001
            };
            node.bounds[2] -= 1;
            node.bounds[3] -= 1;
            node
        })
        .collect();
    if nodes.len() == 2 {
        // ACM rows are sorted, non-overlapping rectangles, unlike MCD's ordered blockers.
        let floor = nodes.pop().unwrap();
        let [x1, z1, x2, z2] = floor.bounds;
        let [a, b, c, d] = nodes[0].bounds;
        nodes.extend(
            [
                [x1, z1, a - 1, z2],
                [c + 1, z1, x2, z2],
                [a, z1, c, b - 1],
                [a, d + 1, c, z2],
            ]
            .map(|bounds| Node { bounds, ..floor }),
        );
        nodes.sort_by_key(|node| node.bounds[1]);
    }
    let map = tenchu_assets::terrain::AreaMap::from_nodes(vec![(0, nodes)]).unwrap();
    world.terrain = tenchu_assets::tenchu2::collision::Collision::from_gaisen(&map).unwrap();
    world.cursor = Default::default();
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; optional immediate dodge uses real source motions, separate cooldowns and exact saved continuation"]
fn native_quick_shuriken_evasion_moves_and_restores_individual_cooldowns() {
    let (base, mut assets, guard) = armed_fixture();
    let mut actions = std::collections::BTreeSet::new();
    for seed in 1..=8 {
        let mut world = base.clone();
        world.random = Random::new(seed);
        world.ai.quick_shuriken_dodge = true;
        if seed % 2 == 0 {
            imported_terrain(&mut world);
        }
        if seed <= 4 {
            world.camera.mode = 20;
            world.items.aim.direction = Position::new(0, 0, -4096);
            world.actors[world.player].actor.rotation.y = 2048;
        }
        assert_eq!(decide(&mut world, &assets, guard), 0);
        let action = world.actors[guard].actor.motion.action;
        assert!(matches!(action, 0x113 | 0x114), "{action:x}");
        actions.insert(action);
        let origin = world.actors[guard].actor.position;
        for _ in 0..8 {
            world
                .control_actor(guard, &mut assets, &mut Recorded::default())
                .unwrap();
            world.animate_actor(guard, &assets).unwrap();
            world
                .move_actor(guard, &assets, &mut Recorded::default())
                .unwrap();
        }
        assert_ne!(world.actors[guard].actor.position.x, origin.x);
    }
    assert_eq!(actions.len(), 2);

    for tick in [10, u32::MAX - 10] {
        let mut world = base.clone();
        // The added fighter must not inherit the first fighter's optional cooldown.
        world.actors[world.player]
            .actor
            .change_motion(0, Velocity::Stop, &assets.resources)
            .unwrap();
        let kind = world.actors[guard].actor.kind;
        let second = world
            .place_actor(
                Placement {
                    kind,
                    definition: kind,
                    behavior: 0x1113,
                    position: Position::new(1000, 0, -6000),
                    yaw: 2048,
                    snap: false,
                },
                &assets,
            )
            .unwrap();
        aiming(&mut world, &assets);
        ready(&mut world, &assets, second);
        world.tick = tick;
        world.ai.quick_shuriken_dodge = true;
        decide(&mut world, &assets, guard);
        decide(&mut world, &assets, second);
        assert!(matches!(
            world.actors[second].actor.motion.action,
            0x113 | 0x114
        ));
        let bytes = postcard::to_allocvec(&world).unwrap();
        let mut restored: World = postcard::from_bytes(&bytes).unwrap();
        for elapsed in [1, 29, 30] {
            for world in [&mut world, &mut restored] {
                ready(world, &assets, guard);
                world.tick = tick.wrapping_add(elapsed);
                let mut source = world.clone();
                source.ai.quick_shuriken_dodge = false;
                decide(&mut source, &assets, guard);
                decide(world, &assets, guard);
                if elapsed < 30 {
                    source.ai.quick_shuriken_dodge = true;
                    assert!(
                        postcard::to_allocvec(world).unwrap()
                            == postcard::to_allocvec(&source).unwrap(),
                        "cooldown elapsed {elapsed}"
                    );
                } else {
                    assert!(matches!(
                        world.actors[guard].actor.motion.action,
                        0x113 | 0x114
                    ));
                }
            }
            assert!(
                postcard::to_allocvec(&world).unwrap() == postcard::to_allocvec(&restored).unwrap()
            );
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; ineligible aims preserve all source AI/RNG/cursor state and never interrupt committed actions"]
fn native_quick_shuriken_evasion_preserves_ineligible_source_decisions() {
    let (base, assets, guard) = armed_fixture();
    for case in 0..17 {
        let mut original = base.clone();
        match case {
            0 => original.actors[original.player].actor.inventory.active = Item::GRAPPLE,
            1 => {
                original.actors[original.player]
                    .actor
                    .change_motion(0, Velocity::Stop, &assets.resources)
                    .unwrap();
            }
            2 => original.actors[original.player].actor.rotation.y = 2048,
            3 => original.actors[guard].actor.rotation.y = 0,
            4 => original.actors[guard].actor.position.z = -2000,
            5 => original.actors[guard].actor.position.z = -15000,
            6 => original.actors[guard].actor.position.y = -2500,
            7 => wall(&mut original, [-100, -350, 100, -250]),
            8 => {
                original.actors[guard]
                    .actor
                    .change_motion(0x900, Velocity::Motion, &assets.resources)
                    .unwrap()
                    .unwrap();
            }
            9 => {
                original.actors[guard]
                    .actor
                    .change_motion(0xa21, Velocity::Motion, &assets.resources)
                    .unwrap()
                    .unwrap();
            }
            10 => {
                original.actors[guard]
                    .actor
                    .change_motion(0x700, Velocity::Motion, &assets.resources)
                    .unwrap()
                    .unwrap();
            }
            11 => original.script = 1,
            12 => {
                original.actors[guard].behavior.forced = Some(ForcedInput {
                    held: 0,
                    remaining: 10,
                })
            }
            13 => original.actors[guard].behavior.programs.combat = Program::Disabled,
            14 => {
                original.actors[guard]
                    .actor
                    .set_weapon(
                        &mut original.instances[guard].rig,
                        guard,
                        &mut original.collision,
                        &assets.resources,
                        WeaponMode::Sheathe,
                    )
                    .unwrap();
            }
            15 => original.actors[original.player].actor.health = 0,
            16 => {
                original.camera.mode = 20;
                original.items.aim.direction = Position::new(0, 0, 4096);
            }
            _ => unreachable!(),
        }
        let mut edited = original.clone();
        edited.ai.quick_shuriken_dodge = true;
        assert_eq!(
            decide(&mut edited, &assets, guard),
            decide(&mut original, &assets, guard),
            "case {case}"
        );
        edited.ai.quick_shuriken_dodge = false;
        assert!(
            postcard::to_allocvec(&edited).unwrap() == postcard::to_allocvec(&original).unwrap(),
            "case {case}"
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; strict corpse sight adds the animated root ray without bypassing native scan/range/facing/difficulty rules"]
fn native_strict_corpse_sight_observes_the_animated_body() {
    let (mut base, assets, guard) = fixture();
    base.script = 0;
    let kind = base.actors[guard].actor.kind;
    let corpse = base
        .place_actor(
            Placement {
                kind,
                definition: kind,
                behavior: 0x1113,
                position: Position::new(0, 0, -6000),
                yaw: 0,
                snap: false,
            },
            &assets,
        )
        .unwrap();
    base.kill_actor(corpse, &assets).unwrap();
    base.actors[corpse].actor.motion.loops = i16::MIN;
    base.actors[corpse].actor.motion.pose.translation[0] = Position::new(4000, -500, 0);
    base.actors[corpse].actor.flags &= !0x80;
    base.publish(corpse, &assets);
    let observer = &mut base.actors[guard].actor;
    observer.position = Position::ZERO;
    observer.previous = observer.position;
    observer.rotation.y = 0;
    observer.flags = 0x10;
    observer.state = State::Normal;
    base.ai.corpses.observer = guard - 1;
    base.ai.global.perception.alert = Default::default();
    let body = base
        .scene
        .world(base.instances[corpse].rig.joints[0])
        .unwrap()
        .translation;
    assert_eq!(body, Position::new(4000, -500, -6000));
    for case in 0..14 {
        for strict in [false, true] {
            let mut world = base.clone();
            world.ai.strict_corpse_sight = strict;
            match case % 7 {
                1 => wall(&mut world, [130, -400, 300, -200]),
                2 => wall(&mut world, [-100, -350, 100, -250]),
                3 => world.actors[guard].actor.rotation.y = 2048,
                4 => world.difficulty = 3,
                5 => world.actors[corpse].actor.motion.loops = 0,
                6 => world.ai.sight_percent = 1,
                _ => {}
            }
            if case >= 7 {
                imported_terrain(&mut world);
            }
            decide(&mut world, &assets, corpse);
            let found = world.actors[corpse].actor.flags & 0x80 != 0;
            assert_eq!(
                found,
                case % 7 == 0 || case % 7 == 1 && !strict,
                "case {case}, strict {strict}"
            );
            if found {
                assert_eq!(world.actors[guard].actor.flags & 0x83, 0x81);
                assert!(world.ai.global.perception.alert.remaining > 0);
            }
            if case % 7 == 5 {
                assert_eq!(world.ai.corpses.observer, guard - 1);
            } else {
                assert_eq!(world.ai.corpses.observer, guard);
            }
            assert_eq!(
                world.random, base.random,
                "optional sight used gameplay RNG"
            );
        }
    }
}
