use super::*;
use project::{Cue, Event, Trigger};

fn project() -> Project {
    Project::new(Mission::new(0, Objective::ReachLord, 140).unwrap())
}

#[test]
#[ignore = "requires TENCHU_DISC; all original challenges and themes must survive Studio authoring storage"]
fn studio_preserves_all_retail_challenges_and_theme_rules() {
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let tables = Tables::load(&assets).unwrap();
    let missions = tables.presets(&assets).unwrap();
    assert_eq!(missions.len(), 16);
    for mission in missions {
        let original = mission.encode().unwrap();
        let p = Project::new(mission);
        let restored = Project::decode(&p.encode().unwrap()).unwrap();
        Theme::load(&assets, restored.mission.theme)
            .unwrap()
            .validate(&restored.mission)
            .unwrap();
        assert_eq!(restored.mission.encode().unwrap(), original);
    }
    for theme in 0..12 {
        let original =
            Mission::new(theme, Objective::ReachLord, tables.appearance[theme][2]).unwrap();
        Theme::load(&assets, theme)
            .unwrap()
            .validate(&original)
            .unwrap();
        assert_eq!(
            Project::decode(&Project::new(original.clone()).encode().unwrap())
                .unwrap()
                .mission,
            original
        );
    }
}

#[test]
fn project_roundtrip_preserves_retail_and_extensions_and_rejects_dangling_references() {
    let mut p = project();
    p.mission.title = "試練".into();
    p.mission.record(901, "Ninja".into());
    p.actors.insert(2, "disc:16".into());
    p.mission.placements.push(tenchu_assets::trial::Placement {
        kind: 140,
        point: Point { x: 3, z: 4 },
        route: vec![],
    });
    p.events.push(Event {
        trigger: Trigger::Defeated(2),
        ..Event::default()
    });
    p.graphics.weather.mode = crate::settings::WeatherMode::Rainstorm;
    let bytes = p.encode().unwrap();
    assert_eq!(Project::decode(&bytes).unwrap(), p);
    assert_eq!(
        Mission::decode(&p.mission.encode().unwrap()).unwrap(),
        p.mission
    );
    let before = p.clone();
    assert!(p.remove_actor(2).is_err());
    assert_eq!(p, before);
    p.events.clear();
    p.remove_actor(2).unwrap();
    assert!(p.actors.is_empty());
    p.version = 2;
    assert!(p.encode().is_err());
    p.version = 1;
    p.events.push(Event {
        trigger: Trigger::Enter {
            min: [10, 0, 0],
            max: [0, 0, 0],
        },
        ..Event::default()
    });
    assert!(p.encode().is_err());
    assert!(Project::decode(b"{\"format\":\"tenchu-level-studio\",\"version\":1}").is_err());
    let mut p = project();
    p.actors.insert(1, "disc:16".into());
    assert!(
        p.encode().is_err(),
        "Gohda's source objective identity must be retained"
    );
}

#[test]
fn history_groups_brush_strokes_and_keeps_graphics_separate_from_records() {
    let mut p = project();
    p.mission.record(901, "Ninja".into());
    let original = p.clone();
    let mut history = History::default();
    history.begin(&p);
    p.graphics.sky.time = 100;
    assert!(history.commit(&mut p));
    assert_eq!(p.mission.records, original.mission.records);
    assert!(history.undo(&mut p));
    assert_eq!(p, original);
    history.begin(&p);
    for i in 0..10 {
        history.begin(&p);
        p.mission.map[i].ground = 1;
    }
    assert!(history.commit(&mut p));
    assert!(p.mission.records.is_empty());
    let painted = p.clone();
    assert!(history.undo(&mut p));
    assert_eq!(p, original);
    assert!(!history.can_undo());
    assert!(history.redo(&mut p));
    assert_eq!(p, painted);
    history.undo(&mut p);
    history.begin(&p);
    p.mission.title = "Branch".into();
    history.commit(&mut p);
    assert!(!history.can_redo());
}

#[test]
fn cue_queues_follow_logic_ticks_and_flags_without_render_or_recursive_execution() {
    let events = vec![
        Event {
            cues: vec![
                Cue {
                    delay: 0,
                    action: Action::Flag {
                        name: "start".into(),
                        value: true,
                    },
                },
                Cue {
                    delay: 2,
                    action: Action::TimeOfDay(300),
                },
                Cue {
                    delay: 2,
                    action: Action::Weather(crate::settings::WeatherMode::Rain),
                },
            ],
            ..Event::default()
        },
        Event {
            trigger: Trigger::Flag {
                name: "start".into(),
                value: true,
            },
            cues: vec![Cue {
                delay: 0,
                action: Action::TimeOfDay(200),
            }],
            ..Event::default()
        },
    ];
    let run = |renders: usize| {
        let mut runtime = Runtime::default();
        let mut output = vec![];
        for tick in 0..8 {
            output.extend(
                runtime
                    .step(
                        &events,
                        Observation {
                            tick,
                            player: [0; 3],
                            defeated: &[],
                        },
                    )
                    .into_iter()
                    .map(|action| (tick, action)),
            );
            for _ in 0..renders {
                assert!(
                    runtime
                        .step(
                            &events,
                            Observation {
                                tick,
                                player: [0; 3],
                                defeated: &[]
                            }
                        )
                        .is_empty()
                );
            }
        }
        output
    };
    let output = run(0);
    assert_eq!(output, run(7));
    assert_eq!(
        output.iter().map(|(tick, _)| *tick).collect::<Vec<_>>(),
        [0, 1, 2, 4]
    );
    assert_eq!(output[2].1, Action::TimeOfDay(300));
}

#[test]
fn repeating_regions_require_reentry_and_do_not_stack_unbounded_queues() {
    let events = [Event {
        once: false,
        trigger: Trigger::Enter {
            min: [0; 3],
            max: [10; 3],
        },
        cues: vec![Cue {
            delay: 600,
            action: Action::Complete,
        }],
        ..Event::default()
    }];
    let mut runtime = Runtime::default();
    let mut fired = vec![];
    for tick in 0..1205 {
        let player = if tick % 2 == 0 { [10; 3] } else { [11; 3] };
        for _ in runtime.step(
            &events,
            Observation {
                tick,
                player,
                defeated: &[],
            },
        ) {
            fired.push(tick);
        }
    }
    assert_eq!(fired, [600, 1202]);
}

#[test]
fn malformed_library_never_overwrites_previous_projects() {
    let path = std::env::temp_dir().join(format!("studio-corrupt-{}.json", std::process::id()));
    crate::storage::atomic_write(&path, b"{bad project").unwrap();
    assert!(store::Library::load(&path).is_err());
    assert_eq!(crate::storage::read(&path).unwrap(), b"{bad project");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn terrain_stamps_rotate_both_layers_and_preserve_old_projects() {
    use tenchu_assets::trial::Cell;
    let mut p = project();
    p.mission.map[0] = Cell {
        ground: 1,
        ornament: 10 | 64,
    };
    p.mission.map[1] = Cell {
        ground: 2 | 192,
        ornament: 255,
    };
    p.mission.map[32] = Cell {
        ground: 255,
        ornament: 255,
    };
    let original =
        building::Stamp::copy(&p.mission, Point { x: 0, z: 0 }, Point { x: 1, z: 2 }).unwrap();
    let mut stamp = original.clone();
    stamp.rotate();
    assert_eq!((stamp.width, stamp.height), (3, 2));
    assert_eq!(
        stamp.cells[2],
        Cell {
            ground: 1 | 64,
            ornament: 10 | 128
        }
    );
    assert_eq!(stamp.cells[5].ground, 2);
    assert_eq!(stamp.cells[1].ground, 255);
    for _ in 0..3 {
        stamp.rotate();
    }
    assert_eq!(stamp, original);
    assert_eq!(
        building::Stamp::from_clipboard(&stamp.clipboard_text().unwrap()).unwrap(),
        stamp
    );
    assert!(building::Stamp::from_clipboard("tenchu-stamp-v1\n{}").is_err());
    assert!(building::Stamp::from_clipboard("ordinary text").is_err());
    p.stamps.push(stamp);
    p.views.push(building::View::new(
        "Courtyard".into(),
        Editor::new(PathBuf::new()).camera,
    ));
    p.mission.record(301, "Ninja".into());
    let before = p.clone();
    let mut history = History::default();
    history.begin(&p);
    p.views[0].name = "Courtyard, wide".into();
    history.commit(&mut p);
    assert_eq!(p.mission.records, before.mission.records);
    assert_eq!(Project::decode(&p.encode().unwrap()).unwrap(), p);
    let mut legacy = serde_json::to_value(&p).unwrap();
    legacy.as_object_mut().unwrap().remove("views");
    legacy.as_object_mut().unwrap().remove("stamps");
    let decoded = Project::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    assert!(decoded.stamps.is_empty() && decoded.views.is_empty());
    p.views[0].pitch = f32::NAN;
    assert!(p.validate().is_err());
    p.views.clear();
    p.stamps[0].cells.pop();
    assert!(p.validate().is_err());
}

#[test]
fn blocked_stamp_paste_is_atomic_and_success_uses_original_patrol_rules() {
    let theme = Theme {
        tiles: (0..10)
            .map(|kind| tenchu_assets::trial::Tile {
                flags: (kind + 1) << 4,
                textures: [-1; 10],
                ornament: None,
                name: String::new(),
            })
            .collect(),
        images: vec![],
        ornaments: vec![],
        vram: Default::default(),
    };
    let mut p = project();
    let actor = editing::add_actor(&mut p.mission, &theme, Point { x: 3, z: 3 }, 140).unwrap();
    for x in [4, 5] {
        editing::patrol(
            &mut p.mission,
            &theme,
            actor,
            Waypoint {
                point: Point { x, z: 3 },
                ..Default::default()
            },
        )
        .unwrap();
    }
    let mut stamp =
        building::Stamp::copy(&p.mission, Point { x: 0, z: 0 }, Point { x: 1, z: 0 }).unwrap();
    stamp.cells[1].ground = 255;
    let before = p.mission.clone();
    assert!(
        stamp
            .paste(&mut p.mission, &theme, Point { x: 2, z: 3 })
            .is_err()
    );
    assert_eq!(p.mission, before);
    assert!(
        stamp
            .paste(&mut p.mission, &theme, Point { x: 31, z: 0 })
            .is_err()
    );
    assert_eq!(p.mission, before);
    stamp
        .paste(&mut p.mission, &theme, Point { x: 4, z: 3 })
        .unwrap();
    assert_eq!(p.mission.placements[actor].route.len(), 2);
    assert_eq!(p.mission.cell(5, 3).unwrap().ground, 255);
    stamp.theme = 1;
    let before = p.mission.clone();
    assert!(
        stamp
            .paste(&mut p.mission, &theme, Point { x: 0, z: 0 })
            .is_err()
    );
    assert_eq!(p.mission, before);
}

#[test]
fn scenery_grounding_uses_source_surface_height_and_keeps_misses_unchanged() {
    let mut editor = Editor::new(PathBuf::new());
    editor.project.scenery.push(project::Scenery {
        name: "Test prop".into(),
        game: AssetGame::Tenchu,
        stage: 0,
        part: 0,
        position: [500, -1800, -500],
        yaw: 0,
    });
    editor.scenery_selection = Some(0);
    let node = tenchu_assets::terrain::Node {
        bounds: [0, -199, 199, 0],
        y: 0,
        dy: 0,
        division: u16::MAX,
        attributes: 0,
    };
    editor.surfaces.push(crate::debug::terrain::Surface {
        node,
        corners: [
            glam::vec3(0.0, 0.0, -2000.0),
            glam::vec3(2000.0, 0.0, -2000.0),
            glam::vec3(0.0, 0.0, 0.0),
            glam::vec3(2000.0, 0.0, 0.0),
        ],
        color: [1.0; 3],
    });
    editor.ground_scenery();
    assert_eq!(editor.project.scenery[0].position, [500, 0, -500]);
    editor.undo(false);
    assert_eq!(editor.project.scenery[0].position, [500, -1800, -500]);
    editor.project.scenery[0].position[0] = 3000;
    let before = editor.project.clone();
    editor.ground_scenery();
    assert_eq!(editor.project, before);
    assert!(editor.notice.contains("No solid terrain"));
}

#[test]
#[ignore = "requires TENCHU_DISC; actual egui clipboard events must reach terrain tools without raw keyboard events"]
fn studio_native_clipboard_events_copy_paste_and_rotate_terrain() {
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let mut editor = Editor::for_showcase();
    editor.ephemeral = false;
    editor.path =
        std::env::temp_dir().join(format!("studio-clipboard-{}.json", std::process::id()));
    editor.enter(&assets).unwrap();
    editor.open = true;
    editor.selection = Some((Point { x: 0, z: 0 }, Point { x: 1, z: 2 }));
    let ctx = egui::Context::default();
    let input = |events| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        )),
        events,
        ..Default::default()
    };
    let output = ctx.run(input(vec![egui::Event::Copy]), |ctx| {
        editor.show(ctx, &assets);
    });
    let copied = output
        .platform_output
        .commands
        .iter()
        .find_map(|command| {
            if let egui::OutputCommand::CopyText(text) = command {
                Some(text.clone())
            } else {
                None
            }
        })
        .expect("Native copy event must put the stamp on the clipboard");
    editor.clipboard = None;
    let _ = ctx.run(input(vec![egui::Event::Paste(copied)]), |ctx| {
        editor.show(ctx, &assets);
    });
    assert!(editor.tool == Tool::Stamp);
    assert_eq!(
        (
            editor.clipboard.as_ref().unwrap().width,
            editor.clipboard.as_ref().unwrap().height
        ),
        (2, 3)
    );
    let _ = ctx.run(
        input(vec![egui::Event::Key {
            key: egui::Key::R,
            physical_key: Some(egui::Key::R),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]),
        |ctx| {
            editor.show(ctx, &assets);
        },
    );
    assert_eq!(
        (
            editor.clipboard.as_ref().unwrap().width,
            editor.clipboard.as_ref().unwrap().height
        ),
        (3, 2)
    );
    assert!(!editor.path.exists());
}

#[test]
fn saving_records_current_build_without_inventing_the_original_build() {
    let mut original = project();
    let initial = original.build.created_with.clone();
    original.build.last_saved_with = Some(provenance::Build {
        version: "old".into(),
        revision: "old".into(),
    });
    let saved = Project::decode(&original.encode().unwrap()).unwrap();
    assert_eq!(saved.build.created_with, initial);
    assert_eq!(
        saved.build.last_saved_with,
        Some(provenance::Build::current())
    );
    let mut old = serde_json::to_value(&saved).unwrap();
    old.as_object_mut().unwrap().remove("build");
    let old = Project::decode(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(old.build, provenance::Provenance::default());
    let saved = Project::decode(&old.encode().unwrap()).unwrap();
    assert!(saved.build.created_with.is_none());
    assert_eq!(
        saved.build.last_saved_with,
        Some(provenance::Build::current())
    );
}
