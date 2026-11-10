use super::*;
use bytemuck::Zeroable;

fn key(root: usize) -> (Part, usize) {
    (
        Part::Actor {
            root,
            model: 2,
            kind: 1,
            inspection: false,
        },
        0,
    )
}

#[test]
fn springs_are_bounded_cadence_independent_and_reset_on_discontinuities() {
    let run = |samples: usize| {
        let mut motion = Motion::default();
        let options = settings::Motion {
            hair: true,
            ..Default::default()
        };
        let mut snapshots = Vec::new();
        for tick in 0..600 {
            let base = if tick >= 120 { 10000.0 } else { 0.0 };
            let phase = if tick < 240 {
                (tick as f32 * 0.21).sin() * 120.0
            } else {
                0.0
            };
            let input = Input {
                key: key(if tick >= 180 { 3 } else { 1 }),
                geometry: if tick >= 200 { 9 } else { 4 },
                anchor: Vec3::new(base + phase, 0.0, 0.0),
                limit: 15.0,
            };
            motion.update(tick, &[input], &options);
            for sample in 0..samples {
                let frame = motion.sample(sample as f32 / samples as f32);
                for pose in frame.values() {
                    assert!(
                        pose.offsets
                            .iter()
                            .all(|p| p.is_finite() && p.length() <= 15.001)
                    );
                }
            }
            let p = motion.sample(1.0).values().next().unwrap().offsets[0];
            if [0, 120, 180, 200].contains(&tick) {
                assert_eq!(p, Vec3::ZERO);
            }
            snapshots.push(p);
        }
        assert!(snapshots[20..100].iter().any(|p| p.length() > 2.0));
        assert!(snapshots.last().unwrap().length() < 0.001);
        motion.update(
            800,
            &[Input {
                key: key(3),
                geometry: 9,
                anchor: Vec3::ZERO,
                limit: 15.0,
            }],
            &options,
        );
        assert_eq!(motion.sample(0.5)[&key(3).0].offsets[0], Vec3::ZERO);
        motion.update(801, &[], &settings::Motion::default());
        assert!(motion.sample(1.0).is_empty());
        snapshots
    };
    assert_eq!(run(1), run(5));
}

#[test]
fn deformation_keeps_anchors_uvs_materials_and_normals_valid() {
    for rules in rules(&Pack::default(), false).values() {
        assert!(rules.len() <= 4);
        for rule in rules {
            rule.validate().unwrap();
        }
    }
    let rule = Rule {
        kind: Kind::Hair,
        axis: [0.0, 1.0, 0.0],
        start: 0.0,
        end: 100.0,
        anchor: [0.0; 3],
        limit: 20.0,
        absolute: false,
        materials: vec![],
    };
    let rules = vec![BoundRule {
        rule: rule.clone(),
        materials: Some(vec![7]),
    }];
    let pose = Pose {
        geometry: 0,
        offsets: [
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
        ],
    };
    let deformation = Deformation::new(&rules, &pose, Matrix::default()).unwrap();
    for (y, expected) in [(0.0, 0.0), (50.0, 5.0), (100.0, 10.0)] {
        let mut vertex = Vertex {
            position: [0.0, y, 0.0],
            normal: [1.0, 0.0, 0.0],
            uv: [0.3, 0.8],
            material: 7,
            ..Vertex::zeroed()
        };
        deformation.vertex(&mut vertex);
        assert_eq!(vertex.position, [expected, y, 0.0]);
        assert_eq!(vertex.uv, [0.3, 0.8]);
        assert_eq!(vertex.material, 7);
        assert!((Vec3::from(vertex.normal).length() - 1.0).abs() < 1e-5);
        if y == 50.0 {
            assert!(vertex.normal[1] < -0.1);
        }
    }
    let mut weapon = Vertex {
        position: [0.0, 100.0, 0.0],
        material: 9,
        ..Vertex::zeroed()
    };
    deformation.vertex(&mut weapon);
    assert_eq!(weapon.position, [0.0, 100.0, 0.0]);
    let mut invalid = rule;
    invalid.limit = 100.0;
    assert!(invalid.validate().is_err());
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; T2 hair/cloth rules follow swapped bodies on either Gaisen hero without changing gameplay"]
fn imported_t2_secondary_motion_uses_the_same_rules_in_gaisen() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = appearance::library::Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let graphics = settings::Graphics {
        motion: settings::Motion {
            hair: true,
            cloth: true,
            equipment: true,
            ..Default::default()
        },
        ..Default::default()
    };
    for character in 0..2 {
        for id in [
            "tenchu2-jp:rikimaru",
            "tenchu2-jp:ayame",
            "tenchu2-jp:tatumaru",
            "tenchu2-jp:ayamea",
        ] {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            let mut models = Models::default()
                .with_player(
                    &game,
                    id,
                    &Pack::default(),
                    &Pack::default(),
                    &graphics,
                    Some(&library),
                )
                .unwrap();
            let mut motion = Motion::default();
            let mut changed = false;
            for tick in 0..60 {
                game.step(if tick < 30 { tenchu_core::input::UP } else { 0 })
                    .unwrap();
                let before = crate::session::tests::state(&game);
                motion.tick(&game, &models, &graphics.motion, None);
                models.motion = motion.sample(1.0);
                if tick % 10 == 0 {
                    let moving =
                        crate::render_mesh::characters_presented(&game, Some(&models), true, None);
                    let frame = std::mem::take(&mut models.motion);
                    let original =
                        crate::render_mesh::characters_presented(&game, Some(&models), true, None);
                    models.motion = frame;
                    assert_eq!(moving.len(), original.len());
                    changed |= moving
                        .iter()
                        .zip(&original)
                        .any(|(a, b)| a.position != b.position);
                }
                assert_eq!(before, crate::session::tests::state(&game));
            }
            assert!(changed, "{id} on player {character} has no authored motion");
        }
    }
}
