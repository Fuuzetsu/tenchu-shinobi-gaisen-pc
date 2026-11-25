use super::*;
use crate::{level_editor::custom::tests::wav, tenchu2::menu::Event};
use tenchu_assets::tenchu2::Edition;

fn application() -> Application {
    // Never run persistence fixtures against the user's actual preferences.
    assert!(std::env::var_os("TENCHU_SANDBOX_MUSIC").is_some());
    let mut app = crate::window::tests::application(Default::default());
    app.tenchu2.refresh_discs();
    app.tenchu2.selected = Some(Edition::Japan);
    app.tenchu2.sandbox = true;
    app.tenchu2.music_tracks = 8;
    app.tenchu2.maps = vec![0];
    app.tenchu2.music = Some(3);
    app.tenchu2.theme = Default::default();
    app
}

fn sandbox(app: &mut Application, custom: bool, fallback: Option<usize>) {
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    let mut sandbox = crate::tenchu2::Sandbox::open(t2.as_ref(), Some(0), None, None).unwrap();
    sandbox.music.selected = fallback;
    sandbox.music.custom = custom.then(|| app.tenchu2.theme.selected().unwrap());
    app.campaign.start_sandbox(sandbox, &app.assets).unwrap();
}

fn labels(output: &egui::FullOutput) -> Vec<String> {
    fn collect(shape: &egui::Shape, text: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => text.push(t.galley.job.text.clone()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, text)),
            _ => (),
        }
    }
    let mut text = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

fn raw(size: [u32; 2]) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size[0] as f32, size[1] as f32),
        )),
        ..Default::default()
    }
}

fn launcher_labels(app: &mut Application) -> Vec<String> {
    let ctx = egui::Context::default();
    let mut text = Vec::new();
    for _ in 0..3 {
        text = labels(&ctx.run(raw([960, 720]), |ctx| {
            app.tenchu2.show_launcher(ctx, &app.campaign);
        }));
    }
    text
}

#[test]
#[ignore = "requires both discs and private TENCHU_SANDBOX_MUSIC; atomic import, F6 actions, embedded file/rewind themes and no-output-device sessions"]
fn sandbox_theme_import_actions_and_saved_sessions_are_independent() {
    let mut app = application();
    let path = crate::storage::UserFile::SandboxMusic.path().unwrap();
    app.install_sandbox_theme("Owned tone A.wav".into(), wav())
        .unwrap();
    let persistent_a = std::fs::read(&path).unwrap();
    let asset_a = app.tenchu2.theme.selected().unwrap();
    assert!(
        app.install_sandbox_theme("broken.mp3".into(), b"bad".to_vec())
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), persistent_a);
    assert_eq!(app.tenchu2.theme.selected().unwrap().id(), asset_a.id());
    let backup = path.with_extension("backup");
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let failed = app.apply_tenchu2(Event::RemoveTheme);
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(&backup, &path).unwrap();
    assert!(failed.is_err());
    assert_eq!(app.tenchu2.theme.selected().unwrap().id(), asset_a.id());
    assert_eq!(std::fs::read(&path).unwrap(), persistent_a);
    for row in [16, 17] {
        app.campaign.cursor = row;
        for action in [Action::Left, Action::Right] {
            assert!(app.tenchu2.navigate(action, &mut app.campaign).1.is_none());
        }
        for action in [Action::Confirm, Action::Start] {
            let event = app.tenchu2.navigate(action, &mut app.campaign).1.unwrap();
            assert!(matches!(
                (row, event),
                (16, Event::ImportTheme) | (17, Event::RemoveTheme)
            ));
        }
    }
    for (edition, scenery) in [(None, false), (Some(Edition::Japan), false)] {
        app.tenchu2.selected = edition;
        app.tenchu2.sandbox = scenery;
        for row in [6, 15, 16, 17] {
            app.campaign.cursor = row;
            assert!(
                app.tenchu2
                    .navigate(Action::Confirm, &mut app.campaign)
                    .1
                    .is_none()
            );
        }
        assert!(
            !launcher_labels(&mut app)
                .iter()
                .any(|s| s.contains("custom theme"))
        );
    }
    app.tenchu2.selected = Some(Edition::Japan);
    app.tenchu2.sandbox = true;
    app.campaign.cursor = 15;
    assert!(matches!(
        app.tenchu2.navigate(Action::Right, &mut app.campaign).1,
        Some(Event::ThemeEnabled(false))
    ));
    app.apply_tenchu2(Event::ThemeEnabled(false)).unwrap();
    assert!(app.tenchu2.theme.selected().is_none());
    assert!(app.tenchu2.theme.asset.is_some());
    app.apply_tenchu2(Event::ThemeEnabled(true)).unwrap();
    app.apply_tenchu2(Event::ThemeFallback(Some(4))).unwrap();

    sandbox(&mut app, true, Some(4));
    assert!(app.audio.is_none());
    let before_game = postcard::to_allocvec(&app.campaign.game).unwrap();
    let saved_a = app.capture_state().unwrap();
    let rewind_a = app.capture_rewind().unwrap();
    let decoded =
        serde_json::to_value(crate::audio::State::silent_sandbox(&app.campaign).unwrap()).unwrap();
    assert_eq!(decoded["sandbox"]["asset"]["name"], "Owned tone A.wav");
    assert_eq!(
        decoded["sandbox"]["fallback"]["channel"],
        app.campaign
            .sandbox
            .as_ref()
            .unwrap()
            .music
            .track()
            .unwrap()
            .channel
    );
    app.install_sandbox_theme("Owned tone B.wav".into(), {
        let mut bytes = wav();
        bytes[44..].reverse();
        bytes
    })
    .unwrap();
    app.apply_tenchu2(Event::ThemeFallback(Some(1))).unwrap();
    let persistent_b = std::fs::read(&path).unwrap();
    sandbox(&mut app, true, Some(1));
    app.restore_state(&saved_a).unwrap();
    assert_eq!(
        postcard::to_allocvec(&app.campaign.game).unwrap(),
        before_game
    );
    let music = &app.campaign.sandbox.as_ref().unwrap().music;
    assert_eq!(music.custom.as_ref().unwrap().id(), asset_a.id());
    assert_eq!(music.selected, Some(4));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        persistent_b,
        "Loading must not rewrite next-launch preferences"
    );
    assert!(
        launcher_labels(&mut app)
            .iter()
            .any(|s| s.contains("current stage retains: Owned tone A.wav"))
    );
    let retained = music_asset(&app);
    let decoded = retained.clip().unwrap();
    app.restore_rewind(&rewind_a).unwrap();
    assert!(std::sync::Arc::ptr_eq(&music_asset(&app), &retained));
    assert!(std::sync::Arc::ptr_eq(
        &music_asset(&app).clip().unwrap(),
        &decoded
    ));
    // Saving while previewing rewind uses its historical audio selection.
    let historical = app.capture_state().unwrap();
    app.apply_tenchu2(Event::RemoveTheme).unwrap();
    let removed = std::fs::read(&path).unwrap();
    app.restore_state(&historical).unwrap();
    assert_eq!(music_asset(&app).id(), asset_a.id());
    assert_eq!(std::fs::read(&path).unwrap(), removed);
    assert!(app.tenchu2.theme.asset.is_none());
    app.campaign.action(Action::Replay, &app.assets).unwrap();
    assert_eq!(music_asset(&app).id(), asset_a.id());
    assert_eq!(
        app.campaign.sandbox.as_ref().unwrap().music.selected,
        Some(4)
    );

    // Disc-only and ambience-only saves retain their session choice too.
    for fallback in [Some(2), None] {
        sandbox(&mut app, false, fallback);
        let saved = app.capture_state().unwrap();
        app.install_sandbox_theme("Next launch.wav".into(), wav())
            .unwrap();
        app.apply_tenchu2(Event::ThemeFallback(Some(1))).unwrap();
        let persistent = std::fs::read(&path).unwrap();
        sandbox(&mut app, true, Some(1));
        app.restore_state(&saved).unwrap();
        assert!(
            app.campaign
                .sandbox
                .as_ref()
                .unwrap()
                .music
                .custom
                .is_none()
        );
        assert_eq!(
            app.campaign.sandbox.as_ref().unwrap().music.selected,
            fallback
        );
        assert!(
            launcher_labels(&mut app)
                .iter()
                .any(|s| s.contains("current stage retains:"))
        );
        app.campaign.action(Action::Replay, &app.assets).unwrap();
        assert_eq!(
            app.campaign.sandbox.as_ref().unwrap().music.selected,
            fallback
        );
        assert_eq!(std::fs::read(&path).unwrap(), persistent);
    }
    std::fs::remove_file(path).unwrap();
}

fn music_asset(app: &Application) -> std::sync::Arc<crate::level_editor::custom::Asset> {
    app.campaign
        .sandbox
        .as_ref()
        .unwrap()
        .music
        .custom
        .clone()
        .unwrap()
}

#[test]
#[ignore = "requires both discs and private TENCHU_SANDBOX_MUSIC/TENCHU_T2_CONFIG; invalid theme notice survives the initial auto-connect"]
fn sandbox_theme_invalid_preferences_notice_survives_disc_connection() {
    assert!(std::env::var_os("TENCHU_SANDBOX_MUSIC").is_some());
    assert!(std::env::var_os("TENCHU_T2_CONFIG").is_some());
    let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
    crate::tenchu2::menu::remember(t2.as_ref()).unwrap();
    let path = crate::storage::UserFile::SandboxMusic.path().unwrap();
    crate::storage::atomic_write(
        &path,
        br#"{"version":1,"fallback":2,"enabled":true,"asset":null}"#,
    )
    .unwrap();
    let mut app = crate::window::tests::application(Default::default());
    app.campaign.screen = Screen::Launcher;
    app.apply_tenchu2(Event::Open).unwrap();
    assert!(app.tenchu2.notice.contains("areas available"));
    assert!(app.tenchu2.theme_notice.contains("missing"));
    assert_eq!(app.tenchu2.music, Some(2));
    assert!(app.tenchu2.theme.asset.is_none());
    app.tenchu2.sandbox = true;
    assert!(
        launcher_labels(&mut app)
            .iter()
            .any(|s| s.contains("Custom theme unavailable"))
    );
    app.install_sandbox_theme("Owned tone.wav".into(), wav())
        .unwrap();
    assert!(app.tenchu2.theme_notice.is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires both discs, private TENCHU_SANDBOX_MUSIC and TENCHU_THEME_REVIEW; offscreen F6 UI-only evidence"]
fn sandbox_theme_launcher_capture() {
    use crate::renderer::Renderer;
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_THEME_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut app = application();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = [960, 720];
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, &Default::default())).unwrap();
    for name in [
        "sandbox-disc",
        "sandbox-custom",
        "sandbox-restored",
        "native",
        "gaisen",
    ] {
        app.tenchu2.selected = Some(Edition::Japan);
        app.tenchu2.sandbox = true;
        app.campaign.cursor = 6;
        app.tenchu2.theme = if name == "sandbox-disc" {
            Default::default()
        } else {
            Theme::imported("Owned tone.wav".into(), wav()).unwrap()
        };
        if name == "sandbox-restored" {
            sandbox(&mut app, true, Some(4));
            app.tenchu2.theme = Default::default();
        } else {
            app.campaign.sandbox = None;
        }
        if name == "native" {
            app.tenchu2.sandbox = false;
        }
        if name == "gaisen" {
            app.tenchu2.selected = None;
        }
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let mut overlay = egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
        for second in 0..3 {
            let mut input = raw(size);
            input.time = Some(f64::from(second));
            let frame = ctx.run(input, |ctx| {
                app.tenchu2.show_launcher(ctx, &app.campaign);
            });
            let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
            renderer.update_overlay_textures(&mut overlay, &mut Vec::new(), frame.textures_delta);
            if second != 2 {
                continue;
            }
            crate::capture::render_to_png(
                &mut renderer,
                size,
                format,
                &output.join(format!("{name}.png")),
                |renderer, target| {
                    let mut encoder = renderer.device.create_command_encoder(&Default::default());
                    {
                        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: target,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.04,
                                        g: 0.04,
                                        b: 0.04,
                                        a: 1.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                    }
                    renderer.queue.submit(Some(encoder.finish()));
                    renderer.draw_overlay(
                        target,
                        &mut overlay,
                        &jobs,
                        &egui_wgpu::ScreenDescriptor {
                            size_in_pixels: size,
                            pixels_per_point: 1.0,
                        },
                    );
                    Ok(())
                },
            )
            .unwrap();
        }
    }
}
