use super::*;
use glam::Vec3;
use tenchu_assets::model::{Corner, Face, Geometry, Part};

fn floor() -> Vec<Part> {
    vec![Part {
        parent: -1,
        child: -1,
        position: [0; 3],
        geometry: vec![Geometry {
            vertices: vec![
                [-1000, 0, -1000],
                [2000, 0, -1000],
                [-1000, 0, 2000],
                [2000, 0, 2000],
            ],
            joints: vec![],
            normals: vec![],
            faces: vec![Face {
                mode: 0x28,
                flags: 1,
                corners: (0..4)
                    .map(|i| Corner {
                        vertex: i,
                        normal: None,
                        color: [128; 3],
                        uv: [0; 2],
                    })
                    .collect(),
                texture: None,
                double_sided: true,
                translucent: false,
            }],
        }],
    }]
}
fn surfaces(parts: &[Part]) -> crate::visuals::surfaces::Surfaces {
    let mut vertices = vec![];
    for part in parts {
        crate::render_mesh::append_geometry(
            &mut vertices,
            &part.geometry,
            tenchu_core::math::Matrix {
                translation: part.position.map(|v| i32::from(v) * 10).into(),
                ..Default::default()
            },
            None,
        );
    }
    crate::visuals::surfaces::Surfaces::new(&vertices)
}
fn source_node(t2: bool) -> Node {
    Node {
        y: 0,
        dy: 0,
        bounds: [0, 0, if t2 { 100 } else { 99 }, if t2 { 100 } else { 99 }],
        attributes: 0,
        division: u16::MAX,
    }
}

#[test]
fn sculpted_floor_matches_collision_footprint_and_keeps_neighbours() {
    for t2 in [false, true] {
        let original = source_node(t2);
        let mut parts = floor();
        let edited = Node {
            y: -100,
            ..original
        };
        assert!(mesh::sculpt(&mut parts, original, edited, t2).unwrap() > 0);
        let geometry = surfaces(&parts);
        let cast = |x, z| {
            geometry
                .cast(glam::vec3(x, -3000., z), glam::vec3(x, 3000., z))
                .unwrap()
                .position
                .y
        };
        assert_eq!(cast(500., 500.), -1000.);
        assert_eq!(cast(-500., 500.), 0.);
        assert_eq!(cast(1500., 500.), 0.);
        let wall = geometry
            .cast(
                glam::vec3(-500., -500., 500.),
                glam::vec3(500., -500., 500.),
            )
            .unwrap();
        assert_eq!(wall.position.x, 0.);
        let mut parts = floor();
        let ramp = Node {
            y: -100,
            dy: -100,
            attributes: if t2 { 0x8000 } else { 0x4000 },
            ..original
        };
        mesh::sculpt(&mut parts, original, ramp, t2).unwrap();
        let ramp = surfaces(&parts)
            .cast(
                glam::vec3(500., -3000., 500.),
                glam::vec3(500., 3000., 500.),
            )
            .unwrap();
        assert!((ramp.position.y + 1500.).abs() < 0.01);
        let mut parts = floor();
        mesh::sculpt(
            &mut parts,
            original,
            Node {
                division: 0xfffe,
                ..original
            },
            t2,
        )
        .unwrap();
        let geometry = surfaces(&parts);
        assert!(
            geometry
                .cast(
                    glam::vec3(100., -3000., 100.),
                    glam::vec3(100., 3000., 100.)
                )
                .is_none()
        );
        assert_eq!(
            geometry
                .cast(
                    glam::vec3(500., -3000., 500.),
                    glam::vec3(500., 3000., 500.)
                )
                .unwrap()
                .position
                .y,
            0.
        );
    }
}

#[test]
fn sculpt_caps_mesh_edges_inside_a_larger_collision_footprint() {
    let mut parts = floor();
    let mut terminator = parts[0].geometry[0].faces[0].clone();
    terminator.flags = 0;
    parts[0].geometry[0].faces.push(terminator.clone());
    let original = Node {
        bounds: [-200, -200, 299, 299],
        ..source_node(false)
    };
    mesh::sculpt(
        &mut parts,
        original,
        Node {
            y: -100,
            ..original
        },
        false,
    )
    .unwrap();
    let hit = surfaces(&parts)
        .cast(
            glam::vec3(-1500., -500., 500.),
            glam::vec3(-500., -500., 500.),
        )
        .unwrap();
    assert_eq!(hit.position.x, -1000.);
    assert_eq!(parts[0].geometry[0].faces.last(), Some(&terminator));
}

#[test]
fn sculpt_preserves_faces_outside_or_touching_the_selected_footprint() {
    for t2 in [false, true] {
        let original = source_node(t2);
        let edited = Node {
            y: -100,
            division: 0xfffe,
            ..original
        };
        let mut selected = floor();
        mesh::sculpt(&mut selected, original, edited, t2).unwrap();
        let mut parts = floor();
        for position in [
            [-200, 0, 0],
            [200, 0, 0],
            [0, 0, -200],
            [0, 0, 200],
            [i16::MIN, 0, i16::MIN],
            [i16::MAX, 0, i16::MAX],
            [0, -100, 0],
            [0, 100, 0],
        ] {
            let mut part = floor().remove(0);
            part.position = position;
            parts.push(part);
        }
        let untouched = parts[1..].to_vec();
        mesh::sculpt(&mut parts, original, edited, t2).unwrap();
        assert_eq!(parts[0], selected[0]);
        assert_eq!(parts[1..], untouched);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and a connected T2 disc; campaign source authoring fixtures, not ordinary-play validation"]
fn campaign_disc_import_edits_geometry_collision_and_roundtrips() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut sources = vec![
        Source::Tenchu {
            stage: 0,
            layout: 1,
        },
        Source::Tenchu {
            stage: 1,
            layout: 1,
        },
    ];
    #[cfg(not(target_arch = "wasm32"))]
    for (edition, path) in crate::tenchu2::menu::known_discs() {
        let disc = crate::source::open_disc(&path).unwrap();
        let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
        let layout = layouts.iter().find(|l| l.map == 0).unwrap();
        sources.push(Source::Tenchu2 {
            map: layout.map,
            sequence: layout.sequence,
            edition,
        });
    }
    assert!(sources.len() > 2, "connect a T2 disc");
    for source in sources {
        let cache = Cache::load(&assets, source.clone(), 0).unwrap();
        cache.initial.validate().unwrap();
        let original = cache.build(&cache.initial).unwrap();
        let math = tenchu_core::math::Math {
            tables: tenchu_assets::executable::Executable::new(&assets.executable)
                .unwrap()
                .math_tables()
                .unwrap(),
        };
        let geometry = crate::visuals::surfaces::Surfaces::stage(&original, &math, None);
        let mut candidates: Vec<_> = cache
            .nodes
            .iter()
            .filter(|(_, n)| {
                n.division == u16::MAX
                    && n.attributes & 0xc000 == 0
                    && n.bounds[0] < n.bounds[2]
                    && n.bounds[1] < n.bounds[3]
            })
            .collect();
        candidates.sort_by_key(|(_, n)| {
            let x = (i64::from(n.bounds[0]) + i64::from(n.bounds[2])) * 5
                - i64::from(cache.initial.start[0]);
            let z = (i64::from(n.bounds[1]) + i64::from(n.bounds[3])) * 5
                - i64::from(cache.initial.start[2]);
            x * x + z * z
        });
        let mut tested = false;
        for (&key, &node) in candidates {
            let corners = viewport::corners(node, source.t2());
            let center = (corners[0] + corners[2]) * 0.5;
            if geometry
                .cast(center - Vec3::Y * 150., center + Vec3::Y * 150.)
                .is_none()
            {
                continue;
            }
            let original_height = tenchu_core::terrain::level(
                &original.terrain,
                &mut tenchu_core::terrain::Cursor::new(&original.terrain),
                (center - Vec3::Y * 100.0)
                    .to_array()
                    .map(|v| v.round() as i32)
                    .into(),
                0,
            )
            .unwrap();
            if original_height != i32::from(node.y) * 10 {
                continue;
            }
            let mut campaign = cache.initial.clone();
            campaign.terrain.push(Edit {
                key,
                original: node,
                node: Node {
                    y: node.y - 100,
                    ..node
                },
                geometry: true,
            });
            let mut parts = (*original.map).clone();
            if mesh::sculpt(&mut parts, node, campaign.terrain[0].node, source.t2()).unwrap() == 0 {
                continue;
            }
            let edited = cache.build(&campaign).unwrap();
            let hit = crate::visuals::surfaces::Surfaces::stage(&edited, &math, None)
                .cast(center - Vec3::Y * 1150., center - Vec3::Y * 850.);
            if hit.is_none() {
                continue;
            }
            let p = (center - Vec3::Y * 1100.)
                .to_array()
                .map(|v| v.round() as i32);
            let y = tenchu_core::terrain::level(
                &edited.terrain,
                &mut tenchu_core::terrain::Cursor::new(&edited.terrain),
                p.into(),
                0,
            )
            .unwrap();
            assert!(
                (y - (i32::from(node.y) - 100) * 10).abs() <= 10,
                "{}: floor {y}, node {node:?}, point {p:?}",
                source.label()
            );
            if let Some(collision) = &cache.native_collision {
                let collision = collision
                    .edited(&[(key[0], key[1], campaign.terrain[0].node)])
                    .unwrap();
                let y = tenchu_core::tenchu2::terrain::level(
                    &collision,
                    &mut Default::default(),
                    p.into(),
                    tenchu_core::tenchu2::terrain::Query::Surface,
                )
                .unwrap()
                .unwrap();
                assert!((y - (i32::from(node.y) - 100) * 10).abs() <= 10);
            }
            let mut project =
                Project::new(Mission::new(0, Objective::EliminateEnemies, 16).unwrap());
            project.campaign = Some(campaign);
            let reopened = Project::decode(&project.encode().unwrap()).unwrap();
            assert_eq!(reopened.campaign, project.campaign);
            assert!(Arc::ptr_eq(&cache.mission.map, &original.map));
            assert_eq!(original.terrain, cache.mission.terrain);
            let mut game = tenchu_core::game::Game::new(
                edited,
                &tenchu_assets::executable::Executable::new(&assets.executable).unwrap(),
                0,
                1,
            )
            .unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            eprintln!(
                "{}: {} actors, {} props, edited {:?}; saved/reopened and 30 playtest ticks",
                source.label(),
                cache.initial.actors.len(),
                cache.initial.props.len(),
                key
            );
            tested = true;
            break;
        }
        assert!(
            tested,
            "{}: no editable visible floor verified",
            source.label()
        );
    }
}

impl Editor {
    pub(crate) fn campaign_test_selection(&mut self, key: [usize; 2]) {
        self.campaign_surface = Some(key);
    }

    pub(crate) fn campaign_test_floor(&self, game: &tenchu_core::game::Game) -> (Edit, Vec3) {
        let campaign = self.project.campaign.as_ref().unwrap();
        let cache = self.campaign_cache.as_ref().unwrap();
        let geometry = crate::visuals::surfaces::Surfaces::stage(&game.mission, &game.math, None);
        let start = Vec3::from_array(campaign.start.map(|v| v as f32));
        let mut nodes: Vec<_> = cache
            .nodes
            .iter()
            .filter(|(_, n)| {
                n.division == u16::MAX
                    && n.attributes & 0xc000 == 0
                    && n.bounds[0] < n.bounds[2]
                    && n.bounds[1] < n.bounds[3]
            })
            .collect();
        nodes.sort_by(|(_, a), (_, b)| {
            let distance = |n: &Node| {
                let q = viewport::corners(*n, campaign.source.t2());
                ((q[0] + q[2]) * 0.5 - start).length_squared()
            };
            distance(a).total_cmp(&distance(b))
        });
        for (&key, &node) in nodes {
            let quad = viewport::corners(node, campaign.source.t2());
            let point = (quad[0] + quad[2]) * 0.5;
            let original_height = tenchu_core::terrain::level(
                &game.mission.terrain,
                &mut tenchu_core::terrain::Cursor::new(&game.mission.terrain),
                (point - Vec3::Y * 100.0)
                    .to_array()
                    .map(|v| v.round() as i32)
                    .into(),
                0,
            )
            .unwrap();
            if original_height == i32::from(node.y) * 10
                && geometry
                    .cast(point - Vec3::Y * 100., point + Vec3::Y * 100.)
                    .is_some()
            {
                let mut parts = (*game.mission.map).clone();
                let edit = Edit {
                    key,
                    original: node,
                    node: Node {
                        y: node.y - 100,
                        ..node
                    },
                    geometry: true,
                };
                if mesh::sculpt(&mut parts, node, edit.node, campaign.source.t2())
                    .is_ok_and(|n| n > 0)
                {
                    return (edit, point);
                }
            }
        }
        panic!("No visible source floor");
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and connected T2 discs; all campaign source-layout construction, not ordinary-play validation"]
fn campaign_all_disc_areas_construct_editable_templates() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let executable = tenchu_assets::executable::Executable::new(&assets.executable).unwrap();
    let library = crate::visuals::appearance::library::Library::load(&assets).unwrap();
    let mut sources = vec![];
    for stage in assets.stages().unwrap() {
        for layout in 1..=3 {
            for character in 0..=1 {
                sources.push((
                    Source::Tenchu {
                        stage: stage.id,
                        layout,
                    },
                    character,
                ));
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    for (edition, path) in crate::tenchu2::menu::known_discs() {
        let disc = crate::source::open_disc(&path).unwrap();
        let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
        let maps = tenchu_assets::tenchu2::level::Level::available(&disc).unwrap();
        for layout in layouts.iter().filter(|l| maps.contains(&l.map)) {
            sources.push((
                Source::Tenchu2 {
                    map: layout.map,
                    sequence: layout.sequence,
                    edition,
                },
                0,
            ));
        }
    }
    let count = sources.len();
    for (source, character) in sources {
        let label = format!("{} / hero {character}", source.label());
        let cache =
            Cache::load(&assets, source, character).unwrap_or_else(|e| panic!("{label}: {e}"));
        cache
            .initial
            .validate()
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let mission = cache
            .build(&cache.initial)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let mut game = tenchu_core::game::Game::new(mission, &executable, character, 1)
            .unwrap_or_else(|e| panic!("{label}: {e}"));
        let mut used = vec![game.player_id];
        let mut imported = Default::default();
        for (i, actor) in cache
            .initial
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.enabled)
        {
            let id = game
                .roster
                .order
                .iter()
                .copied()
                .find(|id| {
                    !used.contains(id)
                        && game.roster.get(*id).kind == actor.spawn.kind
                        && game.roster.get(*id).position.x == actor.spawn.position[0]
                        && game.roster.get(*id).position.z == actor.spawn.position[2]
                })
                .unwrap_or_else(|| panic!("{label}: missing actor {}", actor.name));
            used.push(id);
            if let Some(model) = cache.appearances.get(&(i + 1)) {
                let source = crate::visuals::appearance::library::import_actor(
                    &mut game.mission,
                    &library,
                    model,
                    &mut imported,
                )
                .unwrap_or_else(|e| panic!("{label}: {model}: {e}"));
                game.replace_actor(id, source.actor, source.think)
                    .unwrap_or_else(|e| panic!("{label}: {model}: {e}"));
            }
        }
        assert_eq!(
            game.roster.order.len(),
            used.len(),
            "{label}: unexpected runtime cast"
        );
        for _ in 0..3 {
            game.step(0).unwrap_or_else(|e| panic!("{label}: {e}"));
        }
        eprintln!(
            "{label}: {} enabled actors, {} original props",
            used.len() - 1,
            cache.initial.props.len()
        );
    }
    eprintln!("{count} campaign source/hero/layout templates verified");
}

#[test]
fn campaign_height_edits_keep_placements_and_undo_as_one_change() {
    let mut project = Project::new(Mission::new(0, Objective::EliminateEnemies, 16).unwrap());
    project.campaign = Some(Campaign {
        source: Source::Tenchu {
            stage: 0,
            layout: 1,
        },
        start: [500, 0, 500],
        facing: 0,
        actors: vec![Actor {
            enabled: true,
            story: false,
            name: "Guard".into(),
            source_kind: 16,
            spawn: Enemy {
                slot: 0,
                kind: 16,
                think: 0x3453,
                position: [1500, 0, 500],
                facing: 0,
                patrol: vec![([500, 0, 500], 0)],
            },
        }],
        props: vec![Placement {
            part: 0,
            cloned: true,
            position: [500, -5000, 500],
            facing: 0,
        }],
        pickups: vec![Pickup {
            kind: 0,
            position: [500, 0, 500],
        }],
        terrain: vec![],
    });
    let before = project.clone();
    let mut history = super::super::history::History::default();
    history.begin(&project);
    let node = source_node(false);
    let campaign = project.campaign.as_mut().unwrap();
    campaign.edit_surface(
        Edit {
            key: [0, 0],
            original: node,
            node: Node { y: -100, ..node },
            geometry: true,
        },
        true,
    );
    campaign.edit_surface(
        Edit {
            key: [0, 0],
            original: node,
            node: Node { y: -150, ..node },
            geometry: true,
        },
        true,
    );
    assert_eq!(campaign.start, [500, -1500, 500]);
    assert_eq!(campaign.actors[0].spawn.position, [1500, 0, 500]);
    assert_eq!(campaign.actors[0].spawn.patrol[0].0, [500, -1500, 500]);
    assert_eq!(campaign.props[0].position, [500, -5000, 500]);
    assert_eq!(campaign.pickups[0].position, [500, -1500, 500]);
    assert_eq!(campaign.terrain.len(), 1);
    history.commit(&mut project);
    let saved = Project::decode(&project.encode().unwrap()).unwrap();
    assert_eq!(saved.campaign, project.campaign);
    assert!(history.undo(&mut project));
    assert_eq!(project, before);
    assert!(history.redo(&mut project));
    assert_eq!(project.campaign, saved.campaign);
    project.campaign.as_mut().unwrap().edit_surface(
        Edit {
            key: [0, 0],
            original: node,
            node,
            geometry: true,
        },
        true,
    );
    assert_eq!(project.campaign, before.campaign);
}
