use super::{doors::Sounds, *};
use tenchu_core::tenchu2::terrain::{self, Cursor, Query};

#[test]
#[ignore = "requires both discs; surveys original death floors and drops every native hero onto exposed hazards without health or trigger edits"]
fn imported_death_floors_reach_native_death() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut nodes = 0;
    let mut deaths = 0;
    for stage in 0..11 {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let mut points = BTreeSet::new();
        for row in &source.terrain.rows {
            for index in 0..row.count {
                let node = source.terrain.node(row.list + index * 16).unwrap();
                if node.attributes & 0x206 != 0x200 {
                    continue;
                }
                nodes += 1;
                for [fx, fz] in [[2, 2], [1, 1], [3, 1], [1, 3], [3, 3]] {
                    let point = Position::new(
                        (i32::from(node.bounds[0]) * 4
                            + i32::from(node.bounds[2] - node.bounds[0]) * fx)
                            * 10
                            / 4,
                        i32::from(node.y.min(node.y.wrapping_add(node.dy))) * 10 - 100,
                        (i32::from(node.bounds[1]) * 4
                            + i32::from(node.bounds[3] - node.bounds[1]) * fz)
                            * 10
                            / 4,
                    );
                    let mut cursor = tenchu_core::terrain::Cursor::new(&source.terrain);
                    let floor = tenchu_core::terrain::level(&source.terrain, &mut cursor, point, 0)
                        .unwrap();
                    if floor != tenchu_core::terrain::NONE && cursor.attribute & 0x206 == 0x200 {
                        points.insert([point.x, floor - 100, point.z]);
                    }
                }
            }
        }
        if points.is_empty() {
            continue;
        }
        for hero in 0..3 {
            let (mut assets, mut layout) = Assets::gaisen(&disc, &source, hero, false).unwrap();
            layout.actors.clear();
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
            let mut checked = 0;
            for point in &points {
                let point = Position::from(*point);
                let mut cursor = Cursor::default();
                assert_eq!(
                    terrain::level(&initial.terrain, &mut cursor, point, Query::Uncached).unwrap(),
                    Some(point.y + 100)
                );
                assert_eq!(cursor.material & 0xfff, 0x400);
                // Some narrow/masked patches have a safe floor at head height.
                // Keep the complete floor survey, but drop only onto exposed hazards.
                let mut world = initial.clone();
                if world.teleport_player(point, false, &assets).is_err()
                    || world.actors[world.player].actor.terrain.material & 0xfff != 0x400
                {
                    continue;
                }
                let mut invulnerable = world.clone();
                invulnerable.actors[invulnerable.player].actor.invulnerable = true;
                let original_terrain = postcard::to_allocvec(&world.terrain).unwrap();
                for _ in 0..60 {
                    world
                        .step([0; 2], &mut assets, &mut Sounds::default())
                        .unwrap();
                    invulnerable
                        .step([0; 2], &mut assets, &mut Sounds::default())
                        .unwrap();
                    if world.actors[world.player].actor.health == 0 {
                        break;
                    }
                }
                let player = &world.actors[world.player].actor;
                assert_eq!(player.health, 0, "stage {stage} hero {hero} {point:?}");
                assert_eq!(invulnerable.actors[invulnerable.player].actor.health, 100);
                assert_eq!(
                    postcard::to_allocvec(&world.terrain).unwrap(),
                    original_terrain
                );
                checked += 1;
                if checked == 1 {
                    failure_handoff(&game, &disc, stage, hero, Some(point));
                }
            }
            assert!(
                checked > 0,
                "stage {stage} hero {hero}: no exposed death floor"
            );
            deaths += checked;
            eprintln!("stage {stage} hero {hero}: {checked} native death-floor contacts");
        }
    }
    assert!(nodes > 0 && deaths > 0);
    eprintln!(
        "{nodes} authored death nodes surveyed; {deaths} native deaths; invulnerability retained"
    );
}

#[test]
#[ignore = "requires both discs; native long-fall deaths complete the imported failure route without changing health or animation"]
fn imported_long_falls_reach_mission_failure() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for hero in 0..3 {
        failure_handoff(&game, &disc, 1, hero, None);
    }
}

fn failure_handoff(
    game: &GameAssets<impl ReadAt>,
    disc: &Disc<impl ReadAt>,
    stage: usize,
    hero: usize,
    point: Option<Position>,
) {
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    let (mut director, mut world, mut assets) =
        super::director::fixture(game, disc, stage, i16::from(hero == 1), hero, 1);
    for _ in 0..3000 {
        if !director.update(&mut world, &assets, 0).unwrap() {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            world.advance_mission(&assets).unwrap();
            break;
        }
    }
    assert!(!director.presenting);
    if let Some(point) = point {
        world.teleport_player(point, false, &assets).unwrap();
    } else {
        world
            .teleport_player(Position::new(29040, -35000, 33960), false, &assets)
            .unwrap();
    }
    for _ in 0..1500 {
        if !director.update(&mut world, &assets, 0).unwrap() {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            world.advance_mission(&assets).unwrap();
        }
        if matches!(world.mission.phase, Phase::Finished { .. }) {
            break;
        }
    }
    assert!(
        matches!(
            world.mission.phase,
            Phase::Finished {
                outcome: Outcome::Dead,
                ..
            }
        ),
        "stage {stage} hero {hero}: fatal floor did not finish: {:?}, player {:?}, camera {}, source {:?}",
        world.mission.phase,
        (
            world.actors[world.player].actor.health,
            world.actors[world.player].actor.state,
            world.actors[world.player].actor.motion.action,
            world.actors[world.player].actor.motion.loops
        ),
        world.camera.mode,
        director.source.events
    );
}
