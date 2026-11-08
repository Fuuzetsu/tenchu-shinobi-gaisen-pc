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
            let assets = Assets::load(&disc, hero, map)
                .unwrap_or_else(|error| panic!("hero {hero} map {map}: {error}"));
            for layout in selected {
                let mut world = World::new(
                    layout,
                    Options {
                        hero,
                        difficulty: 2,
                        sight_distance: 1500,
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
