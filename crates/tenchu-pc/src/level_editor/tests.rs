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
