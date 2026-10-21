use super::*;
use crate::render_mesh;

#[test]
#[ignore = "requires TENCHU_DISC and GPU timestamp support; profiles a real mission at 720p"]
fn profile_mission_rendering() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options {
        mission: 5,
        ..Default::default()
    }
    .load(&assets)
    .unwrap();
    game.events = Default::default();
    for _ in 0..60 {
        game.step(0).unwrap();
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!(
        "Profiling {} ({:?})",
        adapter.get_info().name,
        adapter.get_info().backend
    );
    let size = std::env::var("TENCHU_PROFILE_SIZE")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some([w.parse().ok()?, h.parse().ok()?])
        })
        .unwrap_or([1280, 720]);
    eprintln!("Viewport {}x{}", size[0], size[1]);
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        size,
        &game.mission.vram,
    ))
    .unwrap();
    if renderer.timings.is_none() {
        eprintln!("Adapter has no GPU timestamps");
        return;
    }
    renderer.profiling = true;
    let target = renderer
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Performance fixture"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let pack = crate::visuals::pack::Pack::default();
    let mut options = crate::settings::Graphics::default();
    options.set_preset(2);
    options.fog = false;
    let mut models = crate::visuals::models::Models::build(&game.mission, &pack, &options).unwrap();
    let emitters = crate::visuals::lights::Emitters::build(&game, &pack, true);
    renderer.set_emitters(&emitters);
    renderer.bind_models(&mut models);
    let mesh = renderer.upload(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let actors = renderer.upload(&render_mesh::characters(&game, Some(&models)));
    let shadow = renderer.upload(&render_mesh::shadow_stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let shadow_actors = renderer.upload(&render_mesh::shadow_models(&game, Some(&models)));
    let effects = renderer.upload_effects(render_effects::build_options(
        &game,
        false,
        None,
        false,
        Some(&models),
    ));
    let mut camera = crate::debug::camera::Camera::from_game(&game);
    if std::env::var_os("TENCHU_PROFILE_FAR").is_some() {
        camera.position.y -= 12000.0;
        camera.pitch = -0.3;
    }
    let lights: Vec<_> = crate::lighting::sources(
        &game,
        &options.lighting,
        camera.position,
        None,
        emitters.sources(&game),
    )
    .into_iter()
    .map(|s| s.light)
    .collect();
    for (samples, occlusion) in [(1, 0), (4, 0), (1, 70), (4, 70)] {
        if !renderer.sample_counts.contains(&samples) {
            continue;
        }
        options.samples = samples;
        options.post.occlusion = occlusion;
        renderer.configure(&options);
        let mut measurements = Vec::new();
        for frame in 0..90 {
            let mut camera = camera;
            camera.yaw += (frame as f32 / 89.0 - 0.5) * 1.6;
            renderer
                .draw_lit(
                    &target,
                    camera.matrix(size[0] as f32 / size[1] as f32),
                    &[&mesh, &actors],
                    Some(&effects),
                    &lights,
                    &[&shadow, &shadow_actors],
                )
                .unwrap();
            renderer.device.poll(wgpu::Maintain::Wait);
            if frame > 15
                && let Some(sample) = renderer.timings.as_ref().and_then(|t| t.latest)
            {
                assert!(
                    sample
                        .milliseconds
                        .iter()
                        .all(|&ms| ms.is_finite() && ms >= 0.0)
                );
                measurements.push(sample.milliseconds);
            }
        }
        assert!(
            measurements.len() > 50,
            "asynchronous results must keep arriving"
        );
        let mut average = [0.0f32; 4];
        for sample in &measurements {
            for i in 0..4 {
                average[i] += sample[i] / measurements.len() as f32;
            }
        }
        eprintln!(
            "{samples}x MSAA, AO {occlusion}: shadow {:.3}, scene {:.3}, post {:.3}, HUD {:.3} ms",
            average[0], average[1], average[2], average[3]
        );
    }
}
