use super::super::*;
use crate::level_editor::{Command, campaign::Source};
use winit::platform::x11::EventLoopBuilderExtX11;

fn frame(app: &mut Application, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut command = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440., 1000.),
            )),
            time: Some(ctx.input(|i| i.time) + 1.0 / 30.0),
            events,
            ..Default::default()
        },
        |ctx| command = app.studio.show(ctx, &app.assets),
    );
    if let Some(command) = command {
        app.apply_studio(command).unwrap();
    }
    app.refresh_studio().unwrap();
    output
}

fn find(output: &egui::FullOutput, needle: &str) -> egui::Pos2 {
    fn text(shape: &egui::Shape, needle: &str) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(t) if t.galley.job.text == needle => {
                Some(t.visual_bounding_rect().center())
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text(s, needle)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|s| text(&s.shape, needle))
        .unwrap_or_else(|| panic!("Missing UI text: {needle}"))
}

fn click(app: &mut Application, ctx: &egui::Context, text: &str) {
    // Let egui finish the panel's appearance animation before locating it.
    let mut output = frame(app, ctx, vec![]);
    for _ in 0..8 {
        output = frame(app, ctx, vec![]);
    }
    let pos = find(&output, text);
    frame(app, ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn capture(app: &mut Application, output: &std::path::Path, name: &str) {
    for _ in 0..3 {
        app.update_overlay();
    }
    app.view
        .as_mut()
        .unwrap()
        .capture(
            &app.campaign.game,
            &app.debug,
            &output.join(format!("{name}.png")),
        )
        .unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC, connected T2 disc, TENCHU_STUDIO_CAPTURE and X11 GPU; real egui authoring actions, not ordinary-play validation"]
fn studio_browser_and_tile_edit_regressions() {
    let output = PathBuf::from(std::env::var_os("TENCHU_STUDIO_CAPTURE").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut app = super::super::tests::application(Default::default());
    if std::env::var_os("TENCHU_STUDIO_USER_SETTINGS").is_some() {
        app.settings = crate::settings::Store::new().unwrap().load().unwrap();
    } else {
        app.settings
            .graphics
            .set_preset(crate::settings::presets::MODERN_HIGH);
        app.settings.graphics.fog = false;
    }
    let original = postcard::to_allocvec(&app.campaign.game).unwrap();
    let original_settings = app.settings.clone();
    let mut builder = EventLoop::builder();
    builder.with_x11().with_any_thread(true);
    let events = builder.build().unwrap();
    #[allow(deprecated)]
    let window = events
        .create_window(
            Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(1440, 1000)),
        )
        .unwrap();
    app.view = Some(
        pollster::block_on(View::from_window(
            Arc::new(window),
            &app.assets,
            &app.campaign.game,
            &app.settings,
        ))
        .unwrap(),
    );
    let path = output.join("regression-library.json");
    app.studio = crate::level_editor::Editor::new(path.clone());
    app.apply_studio(Command::Open).unwrap();
    let ctx = egui::Context::default();
    click(&mut app, &ctx, "Projects");
    click(&mut app, &ctx, "Open lighting & events example");
    assert_eq!(app.studio.project.mission.title, "Storm arrives");
    let mut roots = std::collections::BTreeSet::new();
    crate::render_mesh::visit_character_parts(&app.campaign.game, true, |key, _, _| {
        if let crate::visuals::interpolation::Part::Actor { root, .. } = key {
            roots.insert(root);
        }
    });
    assert_eq!(roots.len(), app.campaign.game.roster.order.len());
    capture(&mut app, &output, "lighting-example");
    let tiles: Vec<_> = app
        .studio
        .theme
        .as_ref()
        .unwrap()
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, t)| t.flags & 0x3000 != 0)
        .map(|(i, t)| (i as u8, t.is_ornament()))
        .collect();
    assert!(!tiles.is_empty());
    for (id, ornament) in tiles.into_iter().take(3) {
        let cell = &mut app.studio.project.mission.map[10 * 32 + 10];
        if ornament {
            cell.ornament = id;
        } else {
            cell.ground = id;
        }
        app.studio.revision += 1;
        let start = Instant::now();
        app.refresh_studio().unwrap();
        eprintln!(
            "TILE EDIT {id} ornament={ornament}: {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.
        );
        assert_eq!(
            app.campaign.game.mission.trial.as_ref().unwrap().mission,
            app.studio.project.mission
        );
    }
    click(&mut app, &ctx, "Projects");
    click(&mut app, &ctx, "Campaign maps · T1 / T2");
    click(&mut app, &ctx, "Browse / refresh connected discs");
    let stage = app
        .assets
        .stages()
        .unwrap()
        .into_iter()
        .find(|s| s.id == 0)
        .unwrap();
    click(&mut app, &ctx, &format!("T1 · {} · layout 1", stage.name));
    assert_eq!(
        app.studio.project.campaign.as_ref().map(|c| &c.source),
        Some(&Source::Tenchu {
            stage: 0,
            layout: 1
        }),
        "{}",
        app.studio.notice
    );
    assert_eq!(app.campaign.game.mission.stage.directory, stage.directory);
    assert!(app.campaign.game.trial.is_none());
    capture(&mut app, &output, "campaign-t1-ui");
    click(&mut app, &ctx, "Projects");
    let output_ui = frame(&mut app, &ctx, vec![]);
    let pos = find(&output_ui, "Find campaign area or sequence…");
    for pressed in [true, false] {
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    frame(&mut app, &ctx, vec![egui::Event::Text("Japan".into())]);
    let (edition, path2) = crate::tenchu2::menu::known_discs()
        .into_iter()
        .find(|(e, _)| *e == tenchu_assets::tenchu2::Edition::Japan)
        .unwrap();
    let disc = crate::source::open_disc(&path2).unwrap();
    let layout = tenchu_assets::tenchu2::layout::Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.map == 0)
        .unwrap();
    let source = Source::Tenchu2 {
        map: 0,
        sequence: layout.sequence,
        edition,
    };
    click(&mut app, &ctx, &source.label());
    assert_eq!(
        app.studio.project.campaign.as_ref().map(|c| &c.source),
        Some(&source),
        "{}",
        app.studio.notice
    );
    assert!(app.campaign.game.trial.is_none());
    capture(&mut app, &output, "campaign-t2-ui");
    app.apply_studio(Command::Play).unwrap();
    for _ in 0..4 {
        app.game_tick(0);
    }
    assert!(app.fault.is_none(), "{:?}", app.fault);
    app.apply_studio(Command::Return).unwrap();
    app.apply_studio(Command::Close).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    assert_eq!(app.settings, original_settings);
    std::fs::remove_file(path).unwrap();
}
