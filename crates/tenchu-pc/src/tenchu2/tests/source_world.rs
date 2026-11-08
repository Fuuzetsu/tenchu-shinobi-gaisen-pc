use serde::Deserialize;
use tenchu_assets::tenchu2::{layout::Layout, loot::Loot};
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{
        inventory::Inventory,
        world::{Assets, Options, World},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ACTOR_TAIL_ORACLE from the Japanese actor update"]
fn native_actor_visibility_feet_head_tracking_and_effort_match_source() {
    use serde_json::Value;
    use tenchu_assets::{
        tenchu2::collision::{Cell, Collision},
        terrain::Node,
    };
    use tenchu_core::tenchu2::actor::State;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layout = Layout::available(&disc).unwrap().remove(0);
    let mut assets = Assets::load(&disc, 0, &layout).unwrap();
    let mut template = World::new(
        &layout,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 1,
            inventory: Inventory::default(),
        },
        &assets,
    )
    .unwrap();
    assert!(template.actors.len() >= 2);
    template.actors.truncate(2);
    template.instances.truncate(2);
    assets.level.collision = Collision {
        origin: [-3000, -3000],
        dimensions: [1, 1],
        cell_size: 6000,
        cells: vec![Cell {
            minimum: i16::MIN,
            maximum: i16::MAX,
            starts: [0; 4],
            nodes: vec![Node {
                y: 0,
                dy: i16::MAX,
                bounds: [-3000, -3000, 3000, 3000],
                attributes: 0,
                division: u16::MAX,
            }],
        }],
    };
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_ACTOR_TAIL_ORACLE") {
        let mut world = template.clone();
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let flag = |key: &str| row[key].as_bool().unwrap();
        let actor = &mut world.actors[0].actor;
        actor.kind = int("kind") as i16;
        actor.state = State::from_action(int("action") as i16).unwrap();
        actor.motion.action = int("action") as i16;
        actor.motion.loops = -1;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.position = super::source_physics::vector(&row["position"]);
        actor.rotation.y = int("yaw") as i16;
        actor.motion.pose.translation.fill(Position::default());
        actor.motion.pose.rotation.fill([0; 3]);
        actor.motion.pose.rotation[0][1] = int("root_yaw") as i16;
        actor.motion.pose.rotation[1][1] = int("torso_yaw") as i16;
        actor.motion.pose.rotation[2] = serde_json::from_value(row["head"].clone()).unwrap();
        actor.velocity = super::source_physics::vector(&row["velocity"]);
        actor.terrain.material = int("material") as u16;
        actor.target = flag("target").then_some(1);
        let target = &mut world.actors[1].actor;
        target.motion.loops = if flag("target_dead") { i16::MIN } else { 0 };
        target.flags = if flag("target_visible") { 0x100000 } else { 0 };
        target.position = super::source_physics::vector(&row["target_position"]);
        world.player = usize::from(!flag("player"));
        world.tracking = flag("tracked").then_some(1);
        world.water.bottom = int("water");
        world.effort = int("effort") as u16;
        world.travel = int("travel");
        world.publish(0, &assets);
        world.animate_actor(0, flag("visible"), &assets).unwrap();
        let expected = &row["expected"];
        assert_eq!(
            world.actors[0].actor.flags,
            expected["flags"].as_u64().unwrap() as u32,
            "case {count}"
        );
        assert_eq!(
            serde_json::to_value(world.actors[0].actor.motion.pose.rotation[2]).unwrap(),
            expected["head"],
            "case {count}"
        );
        assert_eq!(
            world.travel,
            expected["travel"].as_i64().unwrap() as i32,
            "case {count}"
        );
        assert_eq!(
            world.effort,
            expected["effort"].as_u64().unwrap() as u16,
            "case {count}"
        );
        assert_eq!(
            world.tracking.is_some(),
            expected["tracked"].as_bool().unwrap(),
            "case {count}"
        );
        let feet: Vec<_> = world.instances[0]
            .feet
            .iter()
            .flat_map(|foot| {
                [
                    foot.position.x,
                    foot.position.y,
                    foot.position.z,
                    foot.floor.unwrap_or(i32::MIN),
                ]
            })
            .collect();
        assert_eq!(
            serde_json::to_value(feet).unwrap(),
            expected["foot_points"],
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 10_000);
}

#[derive(Deserialize)]
struct LootCase {
    kind: i16,
    seed: u32,
    inventory: [u8; 20],
    end_seed: u32,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_LOOT_ORACLE from the Japanese executable"]
fn native_actor_inventories_match_source_rng_and_item_slots() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let loot = Loot::load(&disc).unwrap();
    let mut count = 0;
    for case in super::source_control::cases::<LootCase>("TENCHU2_LOOT_ORACLE") {
        let mut random = Random::new(case.seed);
        let inventory = Inventory::for_actor(case.kind, &loot, &mut random);
        assert_eq!(
            inventory.counts, case.inventory,
            "kind {} / seed {}",
            case.kind, case.seed
        );
        assert_eq!(
            random.seed, case.end_seed,
            "kind {} / seed {}",
            case.kind, case.seed
        );
        count += 1;
    }
    assert_eq!(count, 13_000);
}

#[test]
#[ignore = "requires TENCHU2_DISC pointing at the Japanese disc"]
fn native_layouts_create_the_source_cast_and_live_weapon_rigs() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let mut worlds = 0;
    for hero in 0..3 {
        for map in 0..24 {
            let selected: Vec<_> = layouts
                .iter()
                .filter(|layout| {
                    layout.map == map
                        && match hero {
                            0 => layout.sequence < 101,
                            1 => (101..185).contains(&layout.sequence),
                            _ => layout.sequence >= 185,
                        }
                })
                .collect();
            if selected.is_empty() {
                continue;
            }
            for layout in selected {
                let assets = Assets::load(&disc, hero, layout)
                    .unwrap_or_else(|error| panic!("hero {hero} map {map}: {error}"));
                let mut world = World::new(
                    layout,
                    Options {
                        hero,
                        difficulty: 2,
                        seed: 1234,
                        inventory: Inventory::default(),
                    },
                    &assets,
                )
                .unwrap_or_else(|error| panic!("sequence {}: {error}", layout.sequence));
                let cast = assets.resources.registry.cast(hero, map).unwrap();
                let entries: Vec<_> = layout
                    .actors
                    .iter()
                    .filter(|actor| cast.contains(&actor.kind))
                    .collect();
                assert_eq!(world.actors.len(), entries.len() + 1);
                for (agent, source) in world.actors[1..].iter().zip(&entries) {
                    assert_eq!(agent.actor.kind, source.kind);
                    assert_eq!(agent.slot as usize, source.slot);
                    assert_eq!(agent.actor.position, Position::from(source.position));
                    assert_eq!(agent.actor.rotation.y, source.facing);
                    assert_eq!(agent.behavior.home, agent.actor.position);
                }
                for (agent, instance) in world.actors.iter().zip(&world.instances) {
                    assert_eq!(
                        instance.rig.joints.len(),
                        agent.actor.motion.pose.rotation.len()
                    );
                    let body = world.collision.get_mut(instance.rig.joints[0]).unwrap();
                    assert_eq!(body.size.x, agent.actor.width / 2);
                    for root in instance.rig.weapons.iter().flatten() {
                        world.scene.world(*root).unwrap();
                    }
                }
                world
                    .collision
                    .compute(|model| world.scene.world(model))
                    .unwrap();
                let restored: World =
                    serde_json::from_slice(&serde_json::to_vec(&world).unwrap()).unwrap();
                assert_eq!(
                    serde_json::to_value(&restored).unwrap(),
                    serde_json::to_value(&world).unwrap()
                );
                eprintln!(
                    "JP sequence {}: {} native actors",
                    layout.sequence,
                    world.actors.len()
                );
                worlds += 1;
            }
        }
    }
    assert_eq!(worlds, 56);
}
