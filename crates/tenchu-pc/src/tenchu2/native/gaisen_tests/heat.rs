use super::{doors::Sounds, *};
use tenchu_assets::tenchu2::{
    campaign::{Kind, Objective, Region},
    level::objects::{Static, StaticKind},
};
use tenchu_core::tenchu2::terrain::{self, Cursor, Query};

fn fixture(
    disc: &Disc<impl ReadAt>,
    source: &MissionAssets,
    hero: usize,
) -> (World, Assets, Position) {
    let (mut assets, mut layout) = Assets::gaisen(disc, source, hero, false).unwrap();
    layout.actors.clear();
    assets.level.scene.statics.clear();
    assets.level.scene.dynamic.clear();
    assets.campaign.sequences[0].kind = Kind::Gameplay(Objective::Reach {
        region: Region {
            center: [320, 0, 320],
            radius: 1,
        },
        unseen: false,
    });
    let initial = World::new(
        &layout,
        Options {
            hero,
            difficulty: 2,
            seed: 1,
            inventory: Default::default(),
        },
        &assets,
    )
    .unwrap();
    for row in &source.terrain.rows {
        for index in 0..row.count {
            let node = source.terrain.node(row.list + index * 16).unwrap();
            if node.attributes & 0x100 == 0 {
                continue;
            }
            let mut position = Position::new(
                (i32::from(node.bounds[0]) + i32::from(node.bounds[2])) * 5,
                i32::from(node.y) * 10 - 1,
                (i32::from(node.bounds[1]) + i32::from(node.bounds[3])) * 5,
            );
            let mut cursor = Cursor::default();
            let Some(floor) =
                terrain::level(&initial.terrain, &mut cursor, position, Query::Uncached).unwrap()
            else {
                continue;
            };
            if cursor.material & 0xfff != 0x401 {
                continue;
            }
            position.y = floor;
            let mut world = initial.clone();
            if world.teleport_player(position, false, &assets).is_ok()
                && world.actors[world.player].actor.terrain.material & 0xfff == 0x401
                && world.view().effect(position, 0, &assets.resources.math)
            {
                return (world, assets, position);
            }
        }
    }
    panic!("No exposed hot floor in stage {}", source.stage.id);
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; compares actual damaging floors with the native T2 flame collider for each hero and hot-floor stage"]
fn hot_gaisen_floors_use_native_fire_damage() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for stage in [2, 3, 7, 9] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        for hero in 0..3 {
            let (mut world, mut assets, position) = fixture(&disc, &source, hero);
            let (_, mut reference_assets, _) = fixture(&disc, &source, hero);
            reference_assets.level.scene.statics.push(Static {
                position: position.to_array(),
                rotation: [0; 3],
                kind: StaticKind::Effect(0),
            });
            let mut reference = world.clone();
            reference.environment.triggers[1].kind = -1;
            reference.environment.static_phases = vec![0];
            let health = world.actors[world.player].actor.health;
            let mut damage_tick = None;
            for tick in 0..90 {
                world
                    .step([0; 2], &mut assets, &mut Sounds::default())
                    .unwrap();
                reference
                    .step([0; 2], &mut reference_assets, &mut Sounds::default())
                    .unwrap();
                let actual = &world.actors[world.player].actor;
                let expected = &reference.actors[reference.player].actor;
                assert_eq!(
                    postcard::to_allocvec(actual).unwrap(),
                    postcard::to_allocvec(expected).unwrap(),
                    "stage {stage} hero {hero} tick {tick}"
                );
                assert_eq!(world.random, reference.random);
                if actual.health < health {
                    damage_tick = Some(tick);
                    break;
                }
            }
            let tick = damage_tick.unwrap_or_else(|| {
                panic!("stage {stage} hero {hero}: native fire caused no damage at {position:?}")
            });
            eprintln!(
                "stage {stage} hero {hero} hot floor {position:?}: native contact at tick {tick}, HP {} -> {}",
                health, world.actors[world.player].actor.health
            );
            let snapshot = postcard::to_allocvec(&world).unwrap();
            for _ in 0..150 {
                world
                    .step([0; 2], &mut assets, &mut Sounds::default())
                    .unwrap();
            }
            let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
            for _ in 0..150 {
                restored
                    .step([0; 2], &mut assets, &mut Sounds::default())
                    .unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&world).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; hot-floor contact requires grounded feet, and native invulnerability keeps its health behavior"]
fn hot_floors_do_not_ignite_actors_above_them() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let source = MissionAssets::load(&game, 7, 0, 1).unwrap();
    let (mut world, mut assets, _) = fixture(&disc, &source, 0);
    world.actors[world.player].actor.position.y -= 1000;
    for _ in 0..100 {
        world
            .update_environment(&assets, &mut Sounds::default())
            .unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(world.environment.heat_phases, [0]);
    assert!(world.environment.fire.coordinate.is_none());
    world.actors[world.player].actor.position.y += 1000;
    world.actors[world.player].actor.invulnerable = true;
    let health = world.actors[world.player].actor.health;
    for _ in 0..160 {
        world
            .step([0; 2], &mut assets, &mut Sounds::default())
            .unwrap();
    }
    assert!(world.environment.fire.coordinate.is_some());
    assert_eq!(world.actors[world.player].actor.health, health);
}
