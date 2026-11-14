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
    app.view
        .as_ref()
        .unwrap()
        .overlay
        .context
        .style_mut(|style| style.animation_time = 0.0);
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
    if std::env::var_os("TENCHU_STUDIO_GEOMETRY_DIGESTS").is_some() {
        std::fs::write(
            output.join(format!("{name}.geometry.sha256")),
            geometry_digest(app),
        )
        .unwrap();
    }
}

fn geometry_digest(app: &Application) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(geometry(app)))
}

fn geometry(app: &Application) -> Vec<u8> {
    let game = &app.campaign.game;
    let mut vertices = crate::render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&app.view.as_ref().unwrap().models),
    );
    let indexed = crate::render_mesh::indexed_stage(
        &game.mission,
        &game.math,
        Some(&app.view.as_ref().unwrap().models),
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&indexed.expanded()),
        bytemuck::cast_slice::<_, u8>(&vertices),
        "indexed stage must preserve every triangle and attribute"
    );
    let shadows = crate::render_mesh::shadow_stage(
        &game.mission,
        &game.math,
        Some(&app.view.as_ref().unwrap().models),
    );
    let indexed_shadows = crate::render_mesh::indexed_shadow_stage(
        &game.mission,
        &game.math,
        Some(&app.view.as_ref().unwrap().models),
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&indexed_shadows.expanded()),
        bytemuck::cast_slice::<_, u8>(&shadows),
        "indexed shadows must retain translucent casters"
    );
    // Renderer material handles may be assigned in a different order after a
    // full reload. The geometry, UVs, colors and rendering flags must agree.
    for vertex in &mut vertices {
        vertex.material = 0;
    }
    bytemuck::cast_slice(&vertices).to_vec()
}

fn compare_full_rebuild(app: &mut Application) {
    let state = postcard::to_allocvec(&app.campaign.game).unwrap();
    let optimized = geometry(app);
    app.apply_studio(Command::Play).unwrap();
    app.apply_studio(Command::Return).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), state);
    assert_eq!(geometry(app), optimized);
}

fn profile_preview(app: &mut Application, output: &std::path::Path) {
    let state = postcard::to_allocvec(&app.campaign.game).unwrap();
    let focused = app.focused;
    app.focused = true;
    app.view.as_mut().unwrap().renderer.profiling = true;
    let mut cpu = Vec::new();
    let mut updates = Vec::new();
    let mut submissions = Vec::new();
    let mut gpu = Vec::new();
    let mut serial = 0;
    let frames = std::env::var("TENCHU_STUDIO_PROFILE_FRAMES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(120)
        .clamp(120, 3600);
    for frame in 0..frames + 30 {
        let start = Instant::now();
        app.advance_studio_preview(1.0 / 60.0);
        app.update_overlay();
        let update = start.elapsed().as_secs_f64() * 1000.;
        let view = app.view.as_mut().unwrap();
        let draw = view.draw(&app.campaign.game, &app.debug).unwrap().unwrap();
        view.renderer.device.poll(wgpu::Maintain::Wait);
        if frame < 30 {
            continue;
        }
        cpu.push(update + f64::from(draw.submit));
        updates.push(update);
        submissions.push(draw.submit);
        if let Some(sample) = view.renderer.timings.as_ref().and_then(|t| t.latest)
            && sample.serial != serial
        {
            serial = sample.serial;
            gpu.push(sample.milliseconds.iter().sum::<f32>());
        }
    }
    cpu.sort_by(f64::total_cmp);
    updates.sort_by(f64::total_cmp);
    submissions.sort_by(f32::total_cmp);
    gpu.sort_by(f32::total_cmp);
    eprintln!(
        "PREVIEW CPU p50/p95: {:.3}/{:.3} ms; GPU p50/p95: {:?}/{:?} ms",
        cpu[cpu.len() / 2],
        cpu[cpu.len() * 95 / 100],
        gpu.get(gpu.len() / 2),
        gpu.get(gpu.len() * 95 / 100)
    );
    eprintln!(
        "PREVIEW UPDATE/SUBMIT p95: {:.3}/{:.3} ms",
        updates[updates.len() * 95 / 100],
        submissions[submissions.len() * 95 / 100]
    );
    std::fs::write(
        output.join("preview-frame-times.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "cpu_ms_without_present_wait": cpu, "gpu_ms": gpu,
            "cpu_update_ms": updates, "cpu_submit_ms": submissions,
            "graphics": app.settings.graphics,
        }))
        .unwrap(),
    )
    .unwrap();
    app.focused = focused;
    app.view.as_mut().unwrap().renderer.profiling = false;
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), state);
}

fn edit_campaign_surface(app: &mut Application, offset: i16) -> f64 {
    let (key, original) = app.studio.first_campaign_surface().unwrap();
    let campaign = app.studio.project.campaign.as_mut().unwrap();
    campaign.edit_surface(
        crate::level_editor::campaign::Edit {
            key,
            original,
            node: tenchu_assets::terrain::Node {
                y: original.y.saturating_sub(offset),
                ..original
            },
            geometry: true,
        },
        true,
    );
    app.studio.revision += 1;
    let start = Instant::now();
    app.refresh_studio().unwrap();
    start.elapsed().as_secs_f64() * 1000.
}

fn profile_campaign_edits(app: &mut Application, output: &std::path::Path, name: &str) {
    let Some(count) = std::env::var("TENCHU_STUDIO_PROFILE_EDITS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
    else {
        return;
    };
    let count = count.clamp(3, 100);
    let state = postcard::to_allocvec(&app.campaign.game).unwrap();
    let mut times = Vec::with_capacity(count);
    for i in 0..count + 2 {
        let time = edit_campaign_surface(app, if i % 2 == 0 { 100 } else { 50 });
        if i >= 2 {
            times.push(time);
        }
        app.update_overlay();
        let view = app.view.as_mut().unwrap();
        view.draw(&app.campaign.game, &app.debug).unwrap().unwrap();
        view.renderer.device.poll(wgpu::Maintain::Wait);
    }
    edit_campaign_surface(app, 50);
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), state);
    std::fs::write(
        output.join(format!("{name}.edits.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "milliseconds": times,
            "geometry_sha256": geometry_digest(app),
        }))
        .unwrap(),
    )
    .unwrap();
    times.sort_by(f64::total_cmp);
    eprintln!(
        "REPEATED {name} EDITS: {count} samples, median {:.3} ms, range {:.3}..{:.3} ms",
        (times[(count - 1) / 2] + times[count / 2]) * 0.5,
        times[0],
        times[count - 1]
    );
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
    let _ = std::fs::remove_file(&path);
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
    click(&mut app, &ctx, "Characters");
    let camera = app.studio.camera;
    let actor = app.campaign.game.roster.order[2];
    let target = glam::Vec3::from_array(
        app.campaign
            .game
            .roster
            .get(actor)
            .position
            .to_array()
            .map(|v| v as f32),
    ) + glam::vec3(0., -900., 0.);
    let eye = target + glam::vec3(0., 0., 900.);
    let direction = (target - eye).normalize();
    app.studio.camera = crate::debug::camera::Camera {
        position: eye,
        yaw: direction.x.atan2(direction.z),
        pitch: (-direction.y).asin(),
        fov: 95.,
    };
    click(&mut app, &ctx, "Guides");
    app.campaign.game.roster.get_mut(actor).attribute |= tenchu_core::ids::ATTR_SUSPEND;
    capture(&mut app, &output, "character-before");
    app.campaign.game.roster.get_mut(actor).attribute &= !tenchu_core::ids::ATTR_SUSPEND;
    capture(&mut app, &output, "character-after");
    app.studio.camera = camera;
    click(&mut app, &ctx, "Guides");
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
    app.studio.project.graphics.weather.mode = crate::settings::WeatherMode::Rainstorm;
    app.apply_studio(Command::WeatherPreview(120)).unwrap();
    let weather = app
        .view
        .as_ref()
        .unwrap()
        .weather_preview_state(&app.settings);
    assert!(weather[0] > 0.0);
    let mut timings = Vec::new();
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
        timings.push(start.elapsed().as_secs_f64() * 1000.);
        eprintln!(
            "TILE EDIT {id} ornament={ornament}: {:.3} ms",
            start.elapsed().as_secs_f64() * 1000.
        );
        if std::env::var_os("TENCHU_STUDIO_FULL_REBUILD").is_none() {
            assert_eq!(
                app.view
                    .as_ref()
                    .unwrap()
                    .weather_preview_state(&app.settings),
                weather,
                "Weather accumulation or elapsed time reset during a terrain edit"
            );
        }
        assert_eq!(
            app.campaign.game.mission.trial.as_ref().unwrap().mission,
            app.studio.project.mission
        );
    }
    let start = Instant::now();
    for _ in 0..30 {
        app.refresh_studio().unwrap();
    }
    let idle_ms = start.elapsed().as_secs_f64() * 1000. / 30.;
    eprintln!("IDLE PREVIEW UPDATE: {idle_ms:.3} ms");
    std::fs::write(
        output.join("timings.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "full_rebuild": std::env::var_os("TENCHU_STUDIO_FULL_REBUILD").is_some(),
            "optimized_build": !cfg!(debug_assertions), "tile_ms": timings, "idle_ms": idle_ms
        }))
        .unwrap(),
    )
    .unwrap();
    compare_full_rebuild(&mut app);
    capture(&mut app, &output, "edited-terrain");
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
    eprintln!(
        "CAMPAIGN TERRAIN EDIT: {:.3} ms",
        edit_campaign_surface(&mut app, 50)
    );
    profile_campaign_edits(&mut app, &output, "campaign-t1");
    compare_full_rebuild(&mut app);
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
    eprintln!(
        "CAMPAIGN TERRAIN EDIT: {:.3} ms",
        edit_campaign_surface(&mut app, 50)
    );
    profile_campaign_edits(&mut app, &output, "campaign-t2");
    compare_full_rebuild(&mut app);
    click(&mut app, &ctx, "Projects");
    if std::env::var_os("TENCHU_STUDIO_SCAN_CAMPAIGNS").is_some() {
        let mut loaded = Vec::new();
        for (source, name) in app.studio.campaign_choices() {
            click(&mut app, &ctx, "Projects");
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("studio-campaign-search")));
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: Some(egui::Key::A),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        command: true,
                        ctrl: true,
                        ..Default::default()
                    },
                }],
            );
            frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::Text(name[3..].into()),
                    egui::Event::Key {
                        key: egui::Key::A,
                        physical_key: Some(egui::Key::A),
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            click(&mut app, &ctx, &name);
            assert_eq!(
                app.studio.project.campaign.as_ref().map(|c| &c.source),
                Some(&source),
                "{name}: {}",
                app.studio.notice
            );
            assert!(app.campaign.game.mission.trial.is_none());
            assert_eq!(app.studio.preview_revision, app.studio.revision);
            eprintln!("CAMPAIGN LOADED: {name}");
            loaded.push(name);
        }
        let library: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(
            library["projects"].as_array().unwrap().len() < 10,
            "Browsing untouched templates filled the library"
        );
        std::fs::write(
            output.join("campaign-browser-matrix.json"),
            serde_json::to_vec_pretty(&loaded).unwrap(),
        )
        .unwrap();
        click(&mut app, &ctx, "Projects");
    }
    click(&mut app, &ctx, "Open T1 + T2 garden example");
    let ornament = app
        .studio
        .project
        .mission
        .map
        .iter()
        .find(|c| c.ornament != 255)
        .unwrap()
        .ornament;
    app.studio.project.mission.map[11 * 32 + 19].ornament = ornament;
    app.studio.revision += 1;
    let start = Instant::now();
    app.refresh_studio().unwrap();
    eprintln!(
        "MIXED TERRAIN EDIT: {:.3} ms",
        start.elapsed().as_secs_f64() * 1000.
    );
    compare_full_rebuild(&mut app);
    capture(&mut app, &output, "mixed-edited-terrain");
    let ui = frame(&mut app, &ctx, vec![]);
    let limits = ui
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text.starts_with("Limits ·") => {
                Some(t.galley.job.text.clone())
            }
            _ => None,
        })
        .unwrap();
    click(&mut app, &ctx, &limits);
    click(&mut app, &ctx, "Extended limits");
    capture(&mut app, &output, "project-limits");
    assert_eq!(
        app.studio.project.mission.limits,
        tenchu_assets::trial::Limits::MAX
    );
    app.apply_studio(Command::Play).unwrap();
    app.toggle_menu();
    assert!(app.menu.open && app.is_suspended());
    capture(&mut app, &output, "playtest-f1");
    app.toggle_menu();
    for _ in 0..4 {
        app.game_tick(0);
    }
    assert!(app.fault.is_none(), "{:?}", app.fault);
    app.apply_studio(Command::Return).unwrap();
    if std::env::var_os("TENCHU_STUDIO_PROFILE_FRAMES").is_some() {
        click(&mut app, &ctx, "Projects");
        click(&mut app, &ctx, "Open lighting & events example");
        app.studio.project.graphics.weather.mode = crate::settings::WeatherMode::Rainstorm;
        app.apply_studio(Command::WeatherPreview(120)).unwrap();
        profile_preview(&mut app, &output);
    }
    app.apply_studio(Command::Close).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    assert_eq!(app.settings, original_settings);
    std::fs::remove_file(path).unwrap();
}
