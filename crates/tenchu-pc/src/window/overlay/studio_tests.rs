use super::super::*;
use crate::level_editor::{Action as CueAction, AssetGame, Command, Cue, Event, Project, Scenery};
use winit::platform::x11::EventLoopBuilderExtX11;

#[test]
#[ignore = "requires TENCHU_DISC, connected T2 disc and X11 GPU; controlled Studio presentation captures, not ordinary-play validation"]
fn studio_mixed_assets_and_animated_preview_preserve_game_state() {
    let output = std::env::var_os("TENCHU_STUDIO_CAPTURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("tenchu-studio-captures"));
    std::fs::create_dir_all(&output).unwrap();
    let path = output.join("test-library.json");
    let _ = std::fs::remove_file(&path);
    let mut app = super::super::tests::application(Default::default());
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
                .with_inner_size(winit::dpi::PhysicalSize::new(1440, 900)),
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
    app.campaign.screen = Screen::Frontend;
    app.frontend = Some(
        crate::frontend::Frontend::new(
            &app.assets,
            &app.disc,
            &app.campaign,
            app.settings.frontend.clone(),
        )
        .unwrap(),
    );
    app.frontend
        .as_mut()
        .unwrap()
        .stage_showcase(
            &crate::frontend::showcase::Scene {
                page: crate::frontend::showcase::Menu::Title,
                language: "en".into(),
            },
            &app.assets,
            &mut app.campaign,
        )
        .unwrap();
    for _ in 0..20 {
        app.update_overlay();
    }
    app.view
        .as_mut()
        .unwrap()
        .capture(
            &app.campaign.game,
            &app.debug,
            &output.join("title-tools.png"),
        )
        .unwrap();
    app.frontend = None;
    app.campaign.screen = Screen::Playing;
    app.studio = crate::level_editor::Editor::new(path.clone());
    app.studio_action(Command::Open);
    assert!(app.studio.open, "{}", app.studio.notice);
    let mission = app
        .studio
        .tables
        .as_ref()
        .unwrap()
        .presets(&app.assets)
        .unwrap()[0]
        .clone();
    let mut project = Project::new(mission);
    project
        .graphics
        .set_preset(crate::settings::presets::MODERN_HIGH);
    app.studio
        .import(&project.encode().unwrap(), &app.assets)
        .unwrap();
    app.studio.camera = crate::level_editor::Editor::for_showcase().camera;
    let t1 = tenchu_assets::stage::MissionAssets::load(&app.assets, 0, 0, 1).unwrap();
    let disc =
        crate::source::open_disc(&crate::tenchu2::menu::disc_path().expect("Connect a T2 disc"))
            .unwrap();
    let t2 = tenchu_assets::tenchu2::level::Level::load(&disc, 2).unwrap();
    for (game, stage, parts, x) in [
        (AssetGame::Tenchu, 0, t1.objects.as_slice(), 29_000),
        (AssetGame::Tenchu2, 2, t2.objects.as_slice(), 37_000),
    ] {
        let part = parts
            .iter()
            .position(|part| !part.geometry.is_empty())
            .unwrap();
        app.studio.project.scenery.push(Scenery {
            custom: None,
            name: format!("{game:?} scenery"),
            game,
            stage,
            part,
            position: [x, 0, -35_000],
            yaw: 0,
        });
    }
    let body = app
        .view
        .as_ref()
        .unwrap()
        .model_library
        .as_ref()
        .unwrap()
        .bodies
        .iter()
        .find(|body| body.resource == "tenchu2/actors/rikimaru")
        .unwrap()
        .id
        .clone();
    app.studio.project.actors.insert(0, body.clone());
    app.studio.project.actors.insert(2, body);
    app.studio.project.events.push(Event {
        name: "Arrival".into(),
        cues: vec![
            Cue {
                delay: 0,
                action: CueAction::Message {
                    text: "Cross the courtyard before the storm arrives.".into(),
                    ticks: 120,
                },
            },
            Cue {
                delay: 60,
                action: CueAction::Weather(settings::WeatherMode::Rainstorm),
            },
        ],
        ..Event::default()
    });
    app.studio_action(Command::Rebuild);
    assert!(app.studio.open, "{}", app.studio.notice);
    assert_eq!(app.view.as_ref().unwrap().studio_scenery.len(), 2);
    let scene = postcard::to_allocvec(&app.campaign.game).unwrap();
    for (name, key, weather, time) in [
        (
            "terrain-day",
            egui::Key::Num1,
            settings::WeatherMode::Clear,
            780,
        ),
        (
            "scenery-night",
            egui::Key::Num3,
            settings::WeatherMode::Clear,
            1320,
        ),
        ("events", egui::Key::Num4, settings::WeatherMode::Clear, 780),
        (
            "environment-storm",
            egui::Key::Num5,
            settings::WeatherMode::Rainstorm,
            1080,
        ),
    ] {
        app.studio.project.graphics.sky.enabled = true;
        app.studio.project.graphics.sky.time = time;
        app.studio.project.graphics.weather.mode = weather;
        app.refresh_studio().unwrap();
        for _ in 0..120 {
            app.advance_studio_preview(1.0 / 30.0);
        }
        app.update_view();
        let view = app.view.as_mut().unwrap();
        assert_eq!(view.renderer.graphics(), &app.settings.graphics);
        view.overlay.opaque = false;
        for _ in 0..3 {
            view.overlay.custom(&view.window, &view.renderer, |ctx| {
                ctx.input_mut(|input| {
                    input.modifiers.alt = true;
                    input.events.push(egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::ALT,
                    });
                });
                app.studio.show(ctx, &app.assets);
            });
        }
        view.capture(
            &app.campaign.game,
            &app.debug,
            &output.join(format!("{name}.png")),
        )
        .unwrap();
        let file = std::io::BufReader::new(
            std::fs::File::open(output.join(format!("{name}.png"))).unwrap(),
        );
        let mut png = png::Decoder::new(file).read_info().unwrap();
        let mut pixels = vec![0; png.output_buffer_size()];
        let info = png.next_frame(&mut pixels).unwrap();
        let channels = info.color_type.samples();
        let lit = pixels
            .chunks_exact(channels)
            .enumerate()
            .filter(|(i, p)| {
                let x = *i as u32 % info.width;
                let y = *i as u32 / info.width;
                x > info.width / 2
                    && (info.height / 4..info.height * 3 / 4).contains(&y)
                    && p[..3].iter().any(|&c| c > 25)
            })
            .count();
        assert!(
            lit > (info.width * info.height / if time >= 1200 { 100 } else { 20 }) as usize,
            "Scene preview is mostly black: {name}"
        );
        assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), scene);
    }
    for mode in [settings::WeatherMode::Rain, settings::WeatherMode::Snow] {
        app.studio.project.graphics.weather.mode = mode;
        app.studio.project.graphics.weather.wetness = 100;
        app.studio.project.graphics.weather.snow = 100;
        app.studio_action(Command::WeatherPreview(120));
        let view = app.view.as_ref().unwrap();
        let sample = view.renderer.weather;
        assert!(
            if mode == settings::WeatherMode::Rain {
                sample.wet
            } else {
                sample.snow
            } > 0.99
        );
        assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), scene);
        app.studio_action(Command::WeatherPreview(0));
        let sample = app.view.as_ref().unwrap().renderer.weather;
        assert_eq!((sample.wet, sample.snow, sample.puddles), (0.0, 0.0, 0.0));
    }
    app.studio.open_mixed_example(&app.assets).unwrap();
    app.studio_action(Command::Rebuild);
    assert_eq!(app.view.as_ref().unwrap().studio_scenery.len(), 7);
    assert!(
        app.studio
            .project
            .actors
            .values()
            .any(|id| id.starts_with("tenchu2"))
    );
    for i in 0..app.studio.project.views.len() {
        app.studio.camera = app.studio.project.views[i].camera();
        app.refresh_studio().unwrap();
        app.update_view();
        let view = app.view.as_mut().unwrap();
        for _ in 0..3 {
            view.overlay.custom(&view.window, &view.renderer, |ctx| {
                app.studio.show(ctx, &app.assets);
            });
        }
        view.capture(
            &app.campaign.game,
            &app.debug,
            &output.join(format!("mixed-garden-{i}.png")),
        )
        .unwrap();
    }
    for game in [AssetGame::Tenchu, AssetGame::Tenchu2] {
        app.studio_action(Command::BrowseSounds(game, 0));
        let sounds = &app.studio.sound_catalog[&(game, 0)];
        assert!(!sounds.is_empty());
        let id = sounds[0].0;
        app.studio_action(Command::Preview(CueAction::Sound {
            game,
            bank: 0,
            id,
            volume: 100,
            pan: 64,
        }));
    }
    app.studio_action(Command::Play);
    assert!(app.studio.testing, "{}", app.studio.notice);
    for _ in 0..95 {
        app.game_tick(0);
    }
    assert!(app.studio.testing);
    assert_eq!(
        app.settings.graphics.weather.mode,
        settings::WeatherMode::Rainstorm
    );
    app.studio_action(Command::Return);
    let (assets, model, sound) = crate::level_editor::custom::tests::sample();
    app.studio.project.assets = assets;
    app.studio.project.scenery.push(Scenery {
        custom: Some(model),
        name: "Imported banner".into(),
        game: AssetGame::Tenchu,
        stage: 0,
        part: 0,
        position: [33_000, 0, -33_000],
        yaw: 0,
    });
    app.studio.project.events.push(Event {
        cues: vec![
            Cue {
                delay: 0,
                action: CueAction::CustomMusic {
                    asset: sound.clone(),
                },
            },
            Cue {
                delay: 1,
                action: CueAction::CustomSound {
                    asset: sound,
                    volume: 100,
                    pan: 32,
                },
            },
        ],
        ..Default::default()
    });
    let exported = app.studio.project.encode().unwrap();
    app.studio.import(&exported, &app.assets).unwrap();
    app.studio
        .camera
        .focus(glam::vec3(33_000.0, -1000.0, -33_000.0), false);
    for artwork in [false, true] {
        app.studio.project.graphics.asset_pack = artwork;
        app.studio.project.graphics.weather.mode = settings::WeatherMode::Clear;
        app.studio_action(Command::Rebuild);
        let view = app.view.as_mut().unwrap();
        let part = view.studio_scenery.last().unwrap().part;
        let geometry = &app.campaign.game.mission.objects[part].geometry;
        let mut count = 0;
        assert!(
            view.models
                .visit(geometry, Default::default(), true, |triangle| {
                    count += 1;
                    assert!(
                        triangle
                            .iter()
                            .all(|v| v.flags & crate::render_mesh::TEXTURED != 0)
                    );
                    assert!(triangle.iter().any(|v| v.position[1] == -2000.0));
                })
        );
        assert_eq!(count, 1);
        for _ in 0..3 {
            view.overlay.custom(&view.window, &view.renderer, |ctx| {
                ctx.input_mut(|input| {
                    input.modifiers.alt = true;
                    input.events.push(egui::Event::Key {
                        key: egui::Key::Num8,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::ALT,
                    });
                });
                app.studio.show(ctx, &app.assets);
            });
        }
        view.capture(
            &app.campaign.game,
            &app.debug,
            &output.join(format!("custom-assets-artwork-{artwork}.png")),
        )
        .unwrap();
    }
    app.studio_action(Command::Play);
    assert!(app.studio.testing, "{}", app.studio.notice);
    app.game_tick(0);
    app.studio_action(Command::Return);
    app.studio_action(Command::Close);
    assert!(app.studio_previous.is_none(), "{}", app.studio.notice);
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    assert_eq!(app.settings, original_settings);
    app.view
        .as_ref()
        .unwrap()
        .renderer
        .device
        .poll(wgpu::Maintain::Wait);
    std::fs::remove_file(path).unwrap();
}
