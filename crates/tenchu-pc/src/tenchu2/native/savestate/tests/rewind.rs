use super::*;

fn geometry(playback: &Playback<DiscSource>) -> (usize, usize) {
    match &playback.content {
        Content::Gameplay(session) => (
            session.assets.level.map.as_ptr() as usize,
            session
                .assets
                .models
                .values()
                .next()
                .unwrap()
                .parts
                .as_ptr() as usize,
        ),
        Content::Story { story, .. } => (
            story.level.map.as_ptr() as usize,
            story
                .resources
                .models
                .values()
                .next()
                .unwrap()
                .parts
                .as_ptr() as usize,
        ),
        _ => unreachable!(),
    }
}

pub(super) fn check_seek(
    playback: &mut Playback<DiscSource>,
    drawing: &mut ContentDrawing,
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
) {
    let saved =
        capture_rewind(playback, drawing, settings, tools, &mut Default::default()).unwrap();
    let pointers = geometry(playback);
    for _ in 0..10 {
        playback.step(0).unwrap();
        drawing
            .update(&playback.content, renderer, &tools.debug, settings)
            .unwrap();
    }
    restore_rewind(playback, &saved, settings, tools, renderer, drawing).unwrap();
    assert_eq!(geometry(playback), pointers);
    assert!(
        postcard::to_allocvec(&snapshot(playback, drawing, settings, tools).unwrap()).unwrap()
            == saved.bytes().unwrap()
    );
    drawing
        .update(&playback.content, renderer, &tools.debug, settings)
        .unwrap();
}

fn image(
    playback: &Playback<DiscSource>,
    drawing: &ContentDrawing,
    renderer: &mut Renderer,
    name: &str,
) -> Vec<u8> {
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let path = output.join(format!("{name}.png"));
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native T2 rewind verification"),
        size: wgpu::Extent3d {
            width: 640,
            height: 480,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    drawing
        .draw(
            &playback.content,
            &texture.create_view(&Default::default()),
            renderer,
        )
        .unwrap();
    crate::capture::read_texture(renderer, &texture, &path).unwrap();
    std::fs::read(path).unwrap()
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; backwards and forwards seeks reuse geometry, preserve pixels and continue identically"]
fn native_rewind_restores_without_rebuilding_matching_scenes() {
    for sequence in [9, 119, 191, 23] {
        let mut playback = start(sequence);
        if let Content::Gameplay(session) = &mut playback.content {
            session.world.actors[session.world.player]
                .actor
                .invulnerable = true;
        }
        for _ in 0..if sequence == 23 { 600 } else { 30 } {
            playback.step(0).unwrap();
        }
        let mut settings = Settings {
            quick_shuriken_dodge: true,
            strict_corpse_sight: true,
            ..Default::default()
        };
        playback.configure(&settings).unwrap();
        settings.graphics.asset_pack = false;
        settings.graphics.blood = crate::settings::Blood::modern();
        settings.graphics.weather.mode = crate::settings::WeatherMode::Snow;
        settings.graphics.motion.hair = true;
        settings.graphics.motion.cloth = true;
        settings.graphics.motion.equipment = true;
        settings.graphics.sky.enabled = true;
        let mut renderer = renderer(&playback, &settings);
        let graphics = renderer.graphics().clone();
        let mut drawing = ContentDrawing::new(&playback.content, &graphics, &mut renderer).unwrap();
        let mut tools = Tools::default();
        tools.sync(&playback.content, playback.generation);
        let mut history = crate::rewind::History::default();
        let mut expected_image = Vec::new();
        for tick in 0..=60 {
            playback.step(0).unwrap();
            drawing
                .update(&playback.content, &mut renderer, &tools.debug, &settings)
                .unwrap();
            let snapshot = capture_rewind(
                &playback,
                &drawing,
                &settings,
                &tools,
                &mut history.recorder,
            )
            .unwrap();
            history.push(tick, 30, snapshot, None).unwrap();
            if tick == 30 {
                expected_image = image(
                    &playback,
                    &drawing,
                    &mut renderer,
                    &format!("sequence-{sequence}-original"),
                );
            }
        }
        let pointers = geometry(&playback);
        let mut elapsed = std::time::Duration::ZERO;
        for index in [30, 10, 50, 30] {
            settings.quick_shuriken_dodge = false;
            settings.strict_corpse_sight = false;
            let saved = history.snapshot(index).unwrap();
            let started = std::time::Instant::now();
            restore_rewind(
                &mut playback,
                saved,
                &mut settings,
                &mut tools,
                &mut renderer,
                &mut drawing,
            )
            .unwrap();
            elapsed += started.elapsed();
            assert!(settings.quick_shuriken_dodge && settings.strict_corpse_sight);
            assert_eq!(geometry(&playback), pointers, "scene geometry was replaced");
            assert!(
                postcard::to_allocvec(&snapshot(&playback, &drawing, &settings, &tools).unwrap())
                    .unwrap()
                    == saved.bytes().unwrap()
            );
            drawing
                .update(&playback.content, &mut renderer, &tools.debug, &settings)
                .unwrap();
        }
        let reused_image = image(
            &playback,
            &drawing,
            &mut renderer,
            &format!("sequence-{sequence}-reused"),
        );
        assert!(
            expected_image == reused_image,
            "sequence {sequence}: retained renderer pixels differ"
        );
        eprintln!(
            "native rewind sequence={sequence}: decode+restore {:.3} ms/seek",
            elapsed.as_secs_f64() * 250.0
        );

        let saved: State<campaign::state::Owned> = history.snapshot(30).unwrap().decode().unwrap();
        let bytes = crate::savestate::encode(&saved).unwrap();
        let mut rebuilt = start(1);
        let mut rebuilt_settings = settings.clone();
        let mut rebuilt_tools = Tools::default();
        let mut rebuilt_renderer = super::renderer(&rebuilt, &settings);
        let mut rebuilt_drawing = restore(
            &mut rebuilt,
            &bytes,
            &mut rebuilt_settings,
            &mut rebuilt_tools,
            &mut rebuilt_renderer,
        )
        .unwrap();
        rebuilt_drawing
            .update(
                &rebuilt.content,
                &mut rebuilt_renderer,
                &rebuilt_tools.debug,
                &rebuilt_settings,
            )
            .unwrap();
        let rebuilt_image = image(
            &rebuilt,
            &rebuilt_drawing,
            &mut rebuilt_renderer,
            &format!("sequence-{sequence}-rebuilt"),
        );
        assert!(
            reused_image == rebuilt_image,
            "sequence {sequence}: rebuilt renderer pixels differ"
        );
        for _ in 0..30 {
            playback.step(0).unwrap();
            rebuilt.step(0).unwrap();
            drawing
                .update(&playback.content, &mut renderer, &tools.debug, &settings)
                .unwrap();
            rebuilt_drawing
                .update(
                    &rebuilt.content,
                    &mut rebuilt_renderer,
                    &rebuilt_tools.debug,
                    &rebuilt_settings,
                )
                .unwrap();
            assert!(
                postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap()
                    == postcard::to_allocvec(&rebuilt.snapshot().unwrap()).unwrap()
            );
            assert!(
                postcard::to_allocvec(&drawing.capture_state()).unwrap()
                    == postcard::to_allocvec(&rebuilt_drawing.capture_state()).unwrap()
            );
        }
        if let Content::Gameplay(session) = &mut playback.content {
            session.assets.models.values_mut().next().unwrap().parts[0].position[0] ^= 1;
            let pointers = geometry(&playback);
            restore_rewind(
                &mut playback,
                history.snapshot(30).unwrap(),
                &mut settings,
                &mut tools,
                &mut renderer,
                &mut drawing,
            )
            .unwrap();
            assert_ne!(
                geometry(&playback),
                pointers,
                "changed geometry reused old bindings"
            );
            drawing
                .update(&playback.content, &mut renderer, &tools.debug, &settings)
                .unwrap();
            assert!(
                image(
                    &playback,
                    &drawing,
                    &mut renderer,
                    &format!("sequence-{sequence}-fallback")
                ) == expected_image
            );
        }
    }
}
