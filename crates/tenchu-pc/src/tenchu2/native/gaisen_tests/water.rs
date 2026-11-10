use super::{doors::Sounds, *};
use tenchu_core::tenchu2::{
    physics::IN_WATER,
    terrain::{self, Cursor, Query},
};

fn fixture(
    disc: &Disc<impl ReadAt>,
    source: &MissionAssets,
    hero: usize,
    depth: i16,
) -> (World, Assets, Position) {
    use tenchu_assets::tenchu2::campaign::{Kind, Objective, Region};
    let (mut assets, mut layout) = Assets::gaisen(disc, source, hero, false).unwrap();
    assets
        .level
        .collision
        .set_gaisen_water_depth(depth)
        .unwrap();
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
    let mut nodes = Vec::new();
    for row in &source.terrain.rows {
        for i in 0..row.count {
            let node = source.terrain.node(row.list + i * 16).unwrap();
            if node.attributes & 4 != 0 {
                nodes.push(node);
            }
        }
    }
    nodes.sort_by_key(|n| {
        std::cmp::Reverse(
            i64::from(n.bounds[2] - n.bounds[0]) * i64::from(n.bounds[3] - n.bounds[1]),
        )
    });
    for node in nodes {
        let mut point = Position::new(
            (i32::from(node.bounds[0]) + i32::from(node.bounds[2])) * 5,
            i32::from(node.y) * 10 - 10,
            (i32::from(node.bounds[1]) + i32::from(node.bounds[3])) * 5,
        );
        let mut cursor = Cursor::default();
        let Some(surface) =
            terrain::level(&initial.terrain, &mut cursor, point, Query::Uncached).unwrap()
        else {
            continue;
        };
        if cursor.material & 0xf00 != 0x200 {
            continue;
        }
        point.y = surface;
        let mut world = initial.clone();
        if world.teleport_player(point, false, &assets).is_ok()
            && world.actors[world.player].actor.terrain.material & 0xf00 == 0x200
        {
            return (world, assets, point);
        }
    }
    panic!("No usable water surface in stage {}", source.stage.id);
}

#[test]
#[ignore = "requires both discs; original wooden planks remain dry native floors, including the boards beside the training pond"]
fn gaisen_wood_is_not_water() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    for stage in [0, 5, 6, 8, 9, 10] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let (mut assets, mut layout) = Assets::gaisen(&disc, &source, 0, false).unwrap();
        layout.actors.clear();
        let initial = World::new(
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
        let mut fixture = None;
        let mut exposed = 0;
        for row in &source.terrain.rows {
            for i in 0..row.count {
                let node = source.terrain.node(row.list + i * 16).unwrap();
                if node.attributes & 12 != 8 {
                    continue;
                }
                count += 1;
                for [fx, fz] in [[2, 2], [1, 1], [3, 1], [1, 3], [3, 3]] {
                    let mut point = Position::new(
                        (i32::from(node.bounds[0]) * 4
                            + i32::from(node.bounds[2] - node.bounds[0]) * fx)
                            * 10
                            / 4,
                        i32::from(node.y.min(node.y.wrapping_add(node.dy))) * 10 - 10,
                        (i32::from(node.bounds[1]) * 4
                            + i32::from(node.bounds[3] - node.bounds[1]) * fz)
                            * 10
                            / 4,
                    );
                    let mut original = tenchu_core::terrain::Cursor::new(&source.terrain);
                    let floor =
                        tenchu_core::terrain::level(&source.terrain, &mut original, point, 0)
                            .unwrap();
                    if floor == tenchu_core::terrain::NONE || original.attribute & 12 != 8 {
                        continue;
                    }
                    let mut cursor = Cursor::default();
                    assert_eq!(
                        terrain::level(&initial.terrain, &mut cursor, point, Query::Uncached)
                            .unwrap(),
                        Some(floor)
                    );
                    assert_eq!(
                        cursor.material & 0xf00,
                        0,
                        "stage {stage}: plank is not dry"
                    );
                    exposed += 1;
                    if fixture.is_none() {
                        point.y = floor;
                        let mut world = initial.clone();
                        if world.teleport_player(point, false, &assets).is_ok() {
                            fixture = Some(world);
                        }
                    }
                }
            }
        }
        let mut world = fixture.unwrap_or_else(|| panic!("stage {stage}: no usable wooden floor"));
        for _ in 0..90 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            assert_eq!(world.actors[world.player].actor.flags & IN_WATER, 0);
        }
        eprintln!("stage {stage}: {exposed} exposed plank samples; native standing stays dry");
    }
    assert_eq!(count, 133);
}

#[test]
#[ignore = "requires both discs; every native hero enters original T1 water surfaces, with native depth categories and no edited ACM"]
fn gaisen_water_uses_native_depth_and_swimming() {
    use tenchu_core::tenchu2::{actor::State, physics::Depth};
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for stage in [0, 1, 4, 6, 7, 8, 9] {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        for hero in 0..3 {
            let (mut world, mut assets, point) = fixture(&disc, &source, hero, 4000);
            for _ in 0..150 {
                world
                    .step([0; 2], &mut assets, &mut Sounds::default())
                    .unwrap();
            }
            let player = &world.actors[world.player].actor;
            assert_eq!(
                world.water.depth,
                Some(Depth::Swimming),
                "stage {stage} hero {hero} at {point:?}, action {:x}, position {:?}",
                player.motion.action,
                player.position
            );
            assert_eq!(world.water.bottom - world.water.surface, 4000);
            assert_eq!(player.state, State::Swimming);
            assert_eq!(player.health, 100);
            assert_eq!(player.breath, 100);
            assert_eq!(world.terrain.gaisen.as_ref().unwrap().map, source.terrain);
            if hero == 0 {
                capture(&disc, &source, &world, "surface");
            }
            eprintln!(
                "stage {stage}, hero {hero}: native surface swimming at {:?}",
                player.position
            );
        }
    }
    let source = MissionAssets::load(&game, 1, 0, 1).unwrap();
    for (depth, expected) in [(400, Depth::Shallow), (800, Depth::Wading)] {
        let (mut world, mut assets, _) = fixture(&disc, &source, 0, depth);
        for _ in 0..60 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        assert_eq!(world.water.depth, Some(expected));
        assert_eq!(world.water.bottom - world.water.surface, i32::from(depth));
    }
}

#[test]
#[ignore = "requires both discs; native diving, breath loss and resurfacing work in an imported pool and survive a mid-dive snapshot"]
fn gaisen_diving_and_breath_resume_from_snapshots() {
    use tenchu_core::tenchu2::{input, physics::Depth};
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let source = MissionAssets::load(&game, 1, 0, 1).unwrap();
    for hero in 0..3 {
        let (mut world, mut assets, point) = fixture(&disc, &source, hero, 6000);
        for _ in 0..150 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        let mut submerged = false;
        for _ in 0..160 {
            world
                .step([input::JUMP, 0], &mut assets, &mut Sounds::default())
                .unwrap();
            if world.water.depth == Some(Depth::Submerged) {
                submerged = true;
                break;
            }
        }
        assert!(
            submerged,
            "hero {hero}: dive at {point:?}, action {:x}, position {:?}",
            world.actors[world.player].actor.motion.action,
            world.actors[world.player].actor.position
        );
        let snapshot = postcard::to_allocvec(&world).unwrap();
        let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
        for _ in 0..150 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            restored
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        assert!(world.actors[world.player].actor.breath < 100);
        capture(&disc, &source, &world, "submerged");
        for tick in 0..200 {
            let held = input::DOWN | if tick % 8 < 4 { input::JUMP } else { 0 };
            world
                .step([held, 0], &mut assets, &mut Sounds::default())
                .unwrap();
            if world.water.depth == Some(Depth::Swimming) {
                break;
            }
        }
        assert_eq!(
            world.water.depth,
            Some(Depth::Swimming),
            "hero {hero}: resurfacing action {:x}, position {:?}",
            world.actors[world.player].actor.motion.action,
            world.actors[world.player].actor.position
        );
        for _ in 0..150 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        assert_eq!(world.actors[world.player].actor.breath, 100);
    }
}

fn capture(disc: &Disc<impl ReadAt>, source: &MissionAssets, world: &World, phase: &str) {
    let Some(output) = std::env::var_os("TENCHU_GAISEN_REVIEW") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir_all(&output).unwrap();
    let hero = usize::from(world.actors[world.player].actor.kind as u16);
    let (mut assets, _) = Assets::gaisen(disc, source, hero, false).unwrap();
    assets.level.scene.statics.clear();
    assets.level.scene.dynamic.clear();
    assets.level.collision = world.terrain.clone();
    let bank = Arc::new(gaisen_bank(disc, &assets, hero).unwrap());
    let mut session = Session::from_parts(world.clone(), assets, bank);
    session.frame.scenery = session
        .world
        .update_environment(&session.assets, &mut Sounds::default())
        .unwrap();
    let before = postcard::to_allocvec(&session.world).unwrap();
    for modern in [false, true] {
        let mut settings = crate::settings::Settings::default();
        if modern {
            settings
                .graphics
                .set_preset(crate::settings::presets::MODERN_HIGH);
        }
        render::capture(
            &session,
            &settings,
            &output.join(format!(
                "water-stage-{}-hero-{hero}-{phase}-{}.png",
                source.stage.id,
                if modern { "modern" } else { "original" }
            )),
        )
        .unwrap();
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    }
}
