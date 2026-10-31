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
        Some(camera.position),
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
                    crate::renderer::SceneMeshes::new(&[&mesh], &[&actors]),
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

#[test]
#[ignore = "requires TENCHU_DISC and GPU; captures and profiles real roof/wood surfaces without advancing gameplay"]
fn compare_surface_relief() {
    use crate::visuals::catalog::{Region, image_id};
    use glam::Vec3;
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::env::var_os("TENCHU_RELIEF_OUTPUT")
        .expect("Set TENCHU_RELIEF_OUTPUT to a private directory");
    let output = std::path::Path::new(&output);
    std::fs::create_dir_all(output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap())
            .unwrap();
    let game = crate::session::Options::default().load(&assets).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("Surface comparison: {}", adapter.get_info().name);
    let size = [1280, 720];
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        size,
        &game.mission.vram,
    ))
    .unwrap();
    renderer.profiling = true;
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Surface comparison"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let vertices = render_mesh::stage(&game.mission, &game.math, None);
    let mesh = renderer.upload(&vertices);
    let actors = renderer.upload(&render_mesh::characters(&game, None));
    let mut settings = crate::settings::Graphics::default();
    settings.set_preset(7);
    settings.samples = 1;
    settings.fog = false;
    settings.lighting.directional.azimuth = 225;
    // These select diagnostic views only; runtime relief has no texture identity rules.
    for (name, id) in [
        ("roof", "1e23cc33aba80f1403a3542164d1d9a6"),
        ("wood", "237a5cad019a2fe51b455b9efd1e53aa"),
    ] {
        let mut candidates = Vec::new();
        for triangle in vertices.chunks_exact(3) {
            if triangle[0].flags & render_mesh::TEXTURED == 0 {
                continue;
            }
            let mut bounds = [255, 255, 0, 0];
            for vertex in triangle {
                for axis in 0..2 {
                    bounds[axis] = bounds[axis].min(vertex.uv[axis] as u16);
                    bounds[axis + 2] = bounds[axis + 2].max(vertex.uv[axis] as u16);
                }
            }
            let region = Region::find(
                &game.mission.vram,
                tenchu_assets::model::Texture {
                    page: triangle[0].page as u16,
                    palette: triangle[0].palette as u16,
                },
                bounds,
            );
            if image_id(
                [region.rect[2], region.rect[3]],
                &region.pixels(&game.mission.vram),
            ) != id
            {
                continue;
            }
            let points: [Vec3; 3] = std::array::from_fn(|i| Vec3::from(triangle[i].position));
            let normal = -(points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize();
            let center = (points[0] + points[1] + points[2]) / 3.0;
            let distance = center.distance_squared(Vec3::from(
                game.player().position.to_array().map(|n| n as f32),
            ));
            if name != "roof" || normal.y < -0.1 {
                candidates.push((distance, center, normal));
            }
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (_, center, normal) = candidates
            .first()
            .copied()
            .expect("Representative surface missing");
        let mut tangent = normal.cross(Vec3::X).normalize_or_zero();
        if tangent.y > 0.0 {
            tangent = -tangent;
        }
        for (distance, view_name) in [(2300.0, "near"), (14000.0, "far")] {
            let eye = center + normal * distance + tangent * distance * 0.65;
            let camera = Mat4::perspective_rh(
                55.0_f32.to_radians(),
                size[0] as f32 / size[1] as f32,
                20.0,
                250000.0,
            ) * Mat4::look_at_rh(eye, center, Vec3::NEG_Y);
            eprintln!("{name}/{view_name}: eye {eye:?}, target {center:?}");
            let lights = [crate::lighting::Light {
                position: center + normal * 1800.0 + Vec3::X * 3000.0 - Vec3::Y * 2000.0,
                color: [1.0, 0.9, 0.8],
                radius: 12000.0,
            }];
            for mode in 0..3 {
                settings.surface_detail = mode;
                renderer.configure(&settings);
                let mut times = Vec::new();
                for frame in 0..40 {
                    renderer
                        .draw_lit(
                            &target.create_view(&Default::default()),
                            camera,
                            crate::renderer::SceneMeshes::new(&[&mesh], &[&actors]),
                            None,
                            &lights,
                            &[&mesh, &actors],
                        )
                        .unwrap();
                    renderer.device.poll(wgpu::Maintain::Wait);
                    if frame > 15
                        && let Some(sample) = renderer.timings.as_ref().and_then(|t| t.latest)
                    {
                        times.push(sample.milliseconds[1]);
                    }
                }
                eprintln!(
                    "{name}/{view_name} mode {mode}: scene {:.3} ms",
                    times.iter().sum::<f32>() / times.len().max(1) as f32
                );
                crate::capture::read_texture(
                    &renderer,
                    &target,
                    &output.join(format!("{name}-{view_name}-{mode}.png")),
                )
                .unwrap();
            }
        }
    }
}
