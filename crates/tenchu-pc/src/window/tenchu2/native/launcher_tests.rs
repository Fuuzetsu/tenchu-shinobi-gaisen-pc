use super::*;
use crate::tenchu2::menu::Event;

fn paint(app: &mut Application, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut event = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 720.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| event = app.tenchu2.show_launcher(ctx, &app.campaign),
    );
    if let Some(event) = event {
        app.apply_tenchu2(event).unwrap();
    }
    output
}

fn text_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    fn find(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, label))
}

fn click(app: &mut Application, ctx: &egui::Context, label: &str) {
    let output = paint(app, ctx, Vec::new());
    let position = text_rect(&output, label)
        .unwrap_or_else(|| panic!("missing launcher control {label}"))
        .center();
    for pressed in [true, false] {
        paint(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU2_JP_DISC and isolated TENCHU_T2_CONFIG/TENCHU_T2_SAVE; F6 pointer/controller rules, three heroes, equipment and both campaigns' save isolation"]
fn f6_launches_gaisen_stages_with_native_t2_rules() {
    std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate disc preferences");
    let save_path = std::env::var_os("TENCHU_T2_SAVE").expect("Isolate JP progress");
    let native_save = std::fs::read(&save_path).ok();
    let disc = std::env::var_os("TENCHU2_JP_DISC").unwrap();
    let mut app = crate::window::tests::application(Options {
        character: 1,
        layout: 0,
        ..Default::default()
    });
    app.campaign.screen = Screen::Launcher;
    app.campaign.saved = Some(Default::default());
    let gaisen_save = app.campaign.saved.as_ref().unwrap().encode();
    app.tenchu2.path = disc.to_string_lossy().into_owned();
    app.apply_tenchu2(Event::Connect).unwrap();
    app.apply_tenchu2(Event::Select(None)).unwrap();
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    for _ in 0..3 {
        paint(&mut app, &ctx, Vec::new());
    }
    click(&mut app, &ctx, "Tenchu 2 (JP)");
    assert!(app.tenchu2.imported());
    assert_eq!(app.campaign.options.layout, 1);
    click(&mut app, &ctx, "Tatsumaru");
    assert_eq!(app.tenchu2.gaisen_hero, 2);
    assert_eq!(app.campaign.options.character, 1);
    let output = paint(&mut app, &ctx, Vec::new());
    assert!(text_rect(&output, "Water depth").is_some());
    assert!(text_rect(&output, "Continue saved game").is_none());
    app.campaign.cursor = 2;
    app.action(Action::Right);
    assert_eq!(app.tenchu2.gaisen_hero, 0);
    app.campaign.cursor = 5;
    app.action(Action::Left);
    assert_eq!(app.campaign.options.layout, 3);
    app.campaign.cursor = 14;
    app.action(Action::Right);
    assert_eq!(app.tenchu2.gaisen_water_depth, 6000);
    app.campaign.cursor = 13;
    app.action(Action::Left);
    assert!(!app.tenchu2.imported());
    assert_eq!(app.campaign.options.character, 1);
    app.action(Action::Right);
    assert!(app.tenchu2.imported());
    app.campaign.options.difficulty = tenchu_core::ai::Difficulty::Hard;
    for hero in 0..3 {
        app.tenchu2.gaisen_hero = hero;
        app.campaign.options.mission = usize::from(hero != 2);
        app.campaign.options.layout = hero + 1;
        let source = app
            .campaign
            .stages
            .iter()
            .find(|s| s.order == app.campaign.options.mission as i32)
            .unwrap()
            .clone();
        app.campaign.cursor = 1;
        app.action(Action::Confirm);
        assert!(app.tenchu2.notice.is_empty(), "{}", app.tenchu2.notice);
        let playback = &mut app
            .native_tenchu2
            .as_mut()
            .expect("F6 did not enter native T2")
            .playback;
        assert_eq!(playback.run.hero, hero);
        assert_eq!(playback.run.difficulty, 3);
        assert!(matches!(playback.content, Content::Equipment(_)));
        for _ in 0..15 {
            playback.step(0).unwrap();
        }
        let Content::Equipment(equipment) = &mut playback.content else {
            unreachable!()
        };
        equipment.state.selected = 3;
        playback.step(0x2000).unwrap();
        playback.step(0).unwrap();
        playback.action = Some(NativeAction::Continue);
        for _ in 0..40 {
            playback.step(0).unwrap();
            if matches!(playback.content, Content::Gameplay(_)) {
                break;
            }
        }
        let Content::Gameplay(session) = &playback.content else {
            panic!("equipment did not enter imported gameplay")
        };
        assert_eq!(session.assets.level.gaisen.as_ref().unwrap().id, source.id);
        assert_eq!(session.world.map, 24 + source.id);
        assert_eq!(
            session.world.terrain.gaisen.as_ref().unwrap().water_depth,
            6000
        );
        let player = &session.world.actors[session.world.player].actor;
        assert_eq!(player.kind, hero as i16);
        assert_eq!(player.inventory.counts[4], 1);
        let mut start = source.start;
        start[1] = start[1].wrapping_sub(10);
        assert_eq!(player.position.to_array(), start);
        if hero == 0 {
            let Content::Gameplay(session) = &mut playback.content else {
                unreachable!()
            };
            let population = session.world.mission.population;
            session.world.actors[session.world.player]
                .actor
                .invulnerable = true;
            session.world.statistics.killed = population.guards + population.bosses;
            for _ in 0..200 {
                playback.step(0).unwrap();
                if matches!(playback.content, Content::Results(_)) {
                    break;
                }
            }
            assert!(matches!(playback.content, Content::Results(_)));
            playback.action = Some(NativeAction::Retry);
            for _ in 0..40 {
                playback.step(0).unwrap();
                if matches!(playback.content, Content::Equipment(_)) {
                    break;
                }
            }
            assert!(matches!(playback.content, Content::Equipment(_)));
            for _ in 0..15 {
                playback.step(0).unwrap();
            }
            playback.action = Some(NativeAction::Continue);
            for _ in 0..40 {
                playback.step(0).unwrap();
                if matches!(playback.content, Content::Gameplay(_)) {
                    break;
                }
            }
            let Content::Gameplay(session) = &playback.content else {
                panic!("retry equipment did not reload the stage")
            };
            assert_eq!(session.assets.level.gaisen.as_ref().unwrap().id, source.id);
            assert_eq!(
                session.world.terrain.gaisen.as_ref().unwrap().water_depth,
                6000
            );
            assert_eq!(session.world.actors[session.world.player].actor.health, 100);
        }
        for _ in 0..30 {
            app.step_native_tenchu2().unwrap();
        }
        app.leave_native_tenchu2().unwrap();
        assert!(app.native_tenchu2.is_none());
        assert!(app.tenchu2.imported());
        assert!(matches!(app.campaign.screen, Screen::Launcher));
        assert_eq!(app.campaign.saved.as_ref().unwrap().encode(), gaisen_save);
        assert_eq!(std::fs::read(&save_path).ok(), native_save);
    }
    app.tenchu2.discs.clear();
    assert!(
        app.apply_tenchu2(Event::GaisenTenchu2)
            .unwrap_err()
            .contains("Connect a Japanese")
    );
    assert!(app.native_tenchu2.is_none());
}
