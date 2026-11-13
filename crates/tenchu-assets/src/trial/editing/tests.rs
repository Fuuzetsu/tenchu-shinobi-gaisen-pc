use super::*;

fn theme() -> Theme {
    Theme {
        tiles: (0..10)
            .map(|kind| crate::trial::Tile {
                flags: (kind + 1) << 4,
                textures: [-1; 10],
                ornament: None,
                name: String::new(),
            })
            .chain([crate::trial::Tile {
                flags: 0x2410,
                textures: [-1; 10],
                ornament: Some(0),
                name: "Pit cover".into(),
            }])
            .collect(),
        images: vec![],
        ornaments: vec![],
        vram: Default::default(),
    }
}

#[test]
fn painting_preserves_starts_and_invalidates_routes_but_keeps_pit_covers() {
    let theme = theme();
    let mut mission = Mission::new(0, crate::trial::Objective::ReachLord, 140).unwrap();
    let start = mission.placements[0].point;
    let before = mission.clone();
    assert!(paint(&mut mission, &theme, start, false, 255).is_err());
    assert_eq!(mission, before);
    let actor = add_actor(&mut mission, &theme, Point { x: 3, z: 3 }, 140).unwrap();
    for x in [4, 5] {
        patrol(
            &mut mission,
            &theme,
            actor,
            Waypoint {
                point: Point { x, z: 3 },
                ..Default::default()
            },
        )
        .unwrap();
    }
    paint(&mut mission, &theme, Point { x: 5, z: 3 }, false, 8).unwrap();
    assert_eq!(mission.placements[actor].route.len(), 2);
    paint(&mut mission, &theme, Point { x: 4, z: 3 }, false, 8).unwrap();
    assert!(mission.placements[actor].route.is_empty());
    paint(&mut mission, &theme, Point { x: 4, z: 3 }, true, 10).unwrap();
    assert_eq!(mission.cell(4, 3).unwrap().ornament, 10);
    assert!(move_actor(&mut mission, &theme, actor, Point { x: 4, z: 3 }).is_err());
}

#[test]
fn patrol_ramps_require_source_facing_and_waits_keep_their_source_units() {
    let theme = theme();
    let mut mission = Mission::new(0, crate::trial::Objective::ReachLord, 140).unwrap();
    let actor = add_actor(&mut mission, &theme, Point { x: 3, z: 3 }, 140).unwrap();
    paint(
        &mut mission,
        &theme,
        Point { x: 4, z: 3 },
        false,
        5 | (1 << 6),
    )
    .unwrap();
    paint(&mut mission, &theme, Point { x: 5, z: 3 }, false, 1).unwrap();
    for x in [4, 5] {
        patrol(
            &mut mission,
            &theme,
            actor,
            Waypoint {
                point: Point { x, z: 3 },
                wait: 0,
                facing: 1,
            },
        )
        .unwrap();
    }
    patrol(
        &mut mission,
        &theme,
        actor,
        Waypoint {
            point: Point { x: 5, z: 3 },
            wait: 3,
            facing: 2,
        },
    )
    .unwrap();
    assert_eq!(mission.placements[actor].route[2].wait, 3);
    assert_eq!(mission.placements[actor].route[2].facing, 2);
    let before = mission.clone();
    assert!(
        patrol(
            &mut mission,
            &theme,
            actor,
            Waypoint {
                point: Point { x: 7, z: 3 },
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(mission, before);
    assert!(blocked(
        &theme,
        &mission.map,
        Point { x: -1, z: 0 },
        Point { x: 0, z: 0 },
        1
    ));
    assert_eq!(
        Mission::decode(&mission.encode().unwrap()).unwrap(),
        mission
    );
}

#[test]
fn extended_limits_cover_painting_characters_and_patrols_without_lossy_retail_export() {
    let theme = theme();
    let mut mission = Mission::new(0, crate::trial::Objective::EliminateEnemies, 140).unwrap();
    for x in 0..8 {
        add_actor(&mut mission, &theme, Point { x, z: 5 }, 140).unwrap();
    }
    assert!(add_actor(&mut mission, &theme, Point { x: 8, z: 5 }, 140).is_err());
    mission.limits = crate::trial::Limits::MAX;
    add_actor(&mut mission, &theme, Point { x: 8, z: 5 }, 140).unwrap();
    for _ in 0..70 {
        patrol(
            &mut mission,
            &theme,
            2,
            Waypoint {
                point: Point { x: 0, z: 5 },
                wait: 1,
                facing: 0,
            },
        )
        .unwrap();
    }
    mission.limits.ornaments = 64;
    for i in 0..64 {
        paint(
            &mut mission,
            &theme,
            Point {
                x: i % 32,
                z: i / 32,
            },
            true,
            10,
        )
        .unwrap();
    }
    let last = Point { x: 0, z: 2 };
    assert!(paint(&mut mission, &theme, last, true, 10).is_err());
    mission.limits.ornaments = 1024;
    paint(&mut mission, &theme, last, true, 10).unwrap();
    mission.validate().unwrap();
    assert!(mission.encode().unwrap_err().contains(".tenchu"));
    mission.placements.truncate(10);
    mission.placements[2].route.truncate(64);
    paint(&mut mission, &theme, last, true, 255).unwrap();
    let restored = Mission::decode(&mission.encode().unwrap()).unwrap();
    assert_eq!(restored.placements, mission.placements);
    assert_eq!(restored.map, mission.map);
    assert_eq!(restored.limits, crate::trial::Limits::default());
}
