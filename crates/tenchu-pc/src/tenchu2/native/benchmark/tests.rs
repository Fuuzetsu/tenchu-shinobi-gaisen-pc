use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native scenes, frame cadence, reports and normal session pixels survive benchmark completion"]
fn native_benchmark_scenes_and_restore() {
    use super::super::{campaign::Content, presentation::ContentDrawing};
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut playback = Playback::load(disc, 9, 2, 71, Some([0; 21]), None).unwrap();
    for _ in 0..30 {
        playback.step(0).unwrap();
    }
    let mut settings = Settings::default();
    settings.set_visual_preset(crate::settings::presets::MODERN_HIGH);
    settings.graphics.asset_pack = false;
    settings.graphics.weather.mode = crate::settings::WeatherMode::Rain;
    let original = settings.clone();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [640, 480],
        playback.vram(),
    ))
    .unwrap();
    renderer.configure(&settings.graphics);
    let mut drawing =
        ContentDrawing::new(&playback.content, &settings.graphics, &mut renderer).unwrap();
    for _ in 0..30 {
        playback.step(0).unwrap();
        drawing.tick(&playback.content, &settings.graphics).unwrap();
        drawing
            .update(
                &playback.content,
                &mut renderer,
                &Default::default(),
                &settings,
            )
            .unwrap();
    }
    let before = playback.capture_state().unwrap();
    let visual_before = postcard::to_allocvec(&drawing.capture_state().unwrap()).unwrap();
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native benchmark verification"),
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
    let view = target.create_view(&Default::default());
    drawing
        .update(
            &playback.content,
            &mut renderer,
            &Default::default(),
            &settings,
        )
        .unwrap();
    drawing
        .draw(&playback.content, &view, &mut renderer)
        .unwrap();
    let initial = output.join("benchmark-normal-before.png");
    crate::capture::read_texture(&renderer, &target, &initial).unwrap();
    let mut benchmarks = Benchmarks::default();
    benchmarks
        .start(
            &playback.disc,
            shared::Options {
                seconds: 2,
                output: Some(output.join("report.json")),
                ..Default::default()
            },
            &settings,
            &mut renderer,
            &adapter.get_info(),
        )
        .unwrap();
    let mut last = Instant::now();
    let mut frames = 0;
    loop {
        let mut timing = CpuFrame {
            interval: last.elapsed().as_secs_f32() * 1000.0,
            ..Default::default()
        };
        last = Instant::now();
        if !benchmarks
            .advance(&playback.disc, &mut renderer, &mut timing)
            .unwrap()
        {
            break;
        }
        let scene = benchmarks.scene.as_ref().unwrap();
        let run = benchmarks.ui.run.as_mut().unwrap();
        let Content::Gameplay(session) = &scene.content else {
            unreachable!()
        };
        assert_eq!(session.world.tick, 30 + (run.frame / 2) as u32);
        let phase = Instant::now();
        scene
            .drawing
            .draw(&scene.content, &view, &mut renderer)
            .unwrap();
        timing.draw.submit = phase.elapsed().as_secs_f32() * 1000.0;
        renderer.device.poll(wgpu::Maintain::Wait);
        run.record(timing, renderer.timings.as_ref());
        if run.frame == 90 {
            crate::capture::read_texture(
                &renderer,
                &target,
                &output.join(format!("{}.png", scene.clip.id)),
            )
            .unwrap();
        }
        frames += 1;
    }
    assert_eq!(frames, 480);
    let finished = benchmarks.finish(None).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(output.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["scenes"].as_array().unwrap().len(), 4);
    for scene in report["scenes"].as_array().unwrap() {
        assert_eq!(scene["frame"]["samples"], 60);
    }
    let mut presets = crate::settings::library::Manager::new(output.join("presets.json"));
    let store = crate::settings::Store {
        path: output.join("settings.json"),
    };
    restore(
        &playback,
        &mut drawing,
        &mut renderer,
        &mut settings,
        &mut presets,
        &store,
        Some(finished),
    )
    .unwrap();
    assert_eq!(
        postcard::to_allocvec(&drawing.capture_state().unwrap()).unwrap(),
        visual_before
    );
    drawing
        .update(
            &playback.content,
            &mut renderer,
            &Default::default(),
            &settings,
        )
        .unwrap();
    drawing
        .draw(&playback.content, &view, &mut renderer)
        .unwrap();
    let restored = output.join("benchmark-normal-restored.png");
    crate::capture::read_texture(&renderer, &target, &restored).unwrap();
    assert!(
        std::fs::read(initial).unwrap() == std::fs::read(restored).unwrap(),
        "normal pixels changed"
    );
    assert!(
        playback.capture_state().unwrap() == before,
        "benchmark changed normal campaign"
    );
    assert_eq!(settings, original);
    assert!(!store.path.exists() && !output.join("presets.json").exists());
    // An interrupted optimisation also leaves an active seed and its return context intact.
    playback.prepare_randomizer(None).unwrap();
    let seeded = crate::randomizer::Run::new(crate::randomizer::Document::for_source(
        crate::randomizer::Source::Tenchu2,
    ));
    playback.start_randomizer(seeded).unwrap();
    drawing
        .replace(
            &playback.content,
            playback.vram(),
            &settings.graphics,
            &mut renderer,
            false,
        )
        .unwrap();
    let seeded_before = playback.capture_state().unwrap();
    benchmarks
        .start(
            &playback.disc,
            shared::Options {
                seconds: 2,
                auto_target: Some(30),
                ..Default::default()
            },
            &settings,
            &mut renderer,
            &adapter.get_info(),
        )
        .unwrap();
    for _ in 0..4 {
        assert!(
            benchmarks
                .advance(&playback.disc, &mut renderer, &mut CpuFrame::default())
                .unwrap()
        );
    }
    let finished = benchmarks.finish(Some("Cancelled by user".into())).unwrap();
    assert!(finished.recommendation.is_none());
    restore(
        &playback,
        &mut drawing,
        &mut renderer,
        &mut settings,
        &mut presets,
        &store,
        Some(finished),
    )
    .unwrap();
    assert!(playback.capture_state().unwrap() == seeded_before);
    assert_eq!(settings, original);
    assert!(!store.path.exists() && !output.join("presets.json").exists());
    playback.end_randomizer();
    assert!(playback.capture_state().unwrap() == before);
}
