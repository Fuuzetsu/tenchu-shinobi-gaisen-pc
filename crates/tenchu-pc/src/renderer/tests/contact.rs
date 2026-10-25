use super::*;

#[test]
#[ignore = "requires a GPU; cosmetic wet marks blend locally and spark emission remains bright in unlit scenes"]
fn contact_wet_marks_and_sparks_render_without_weather_or_lamps() {
    let (mut renderer, target) = fixture(&Vram::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let view = target.create_view(&Default::default());
    let source = rectangle(1000, 128, None, false, false, false);
    let mesh = renderer.upload(&source);
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 0;
    graphics.lighting.directional.intensity = 0;
    renderer.configure(&graphics);
    let draw = |renderer: &mut Renderer, flag| {
        let mut vertices = rectangle(990, 200, None, false, false, false);
        for vertex in &mut vertices {
            vertex.flags |= render_mesh::ALPHA | flag;
            vertex.opacity = 0.8;
        }
        let effect = renderer.upload_effects(render_effects::Geometry {
            vertices,
            batches: vec![render_effects::Batch {
                blend: 5,
                range: 0..6,
            }],
        });
        renderer
            .draw(&view, camera, &[&mesh], Some(&effect))
            .unwrap();
        read_pixel(renderer, &target)
    };
    let unlit = draw(&mut renderer, 0);
    let glow = draw(&mut renderer, render_mesh::EMITTER);
    assert!(
        glow[0] > unlit[0] + 80,
        "spark emission: {unlit:?} -> {glow:?}"
    );
    graphics.lighting.enabled = false;
    renderer.configure(&graphics);
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    let dry = read_pixel(&renderer, &target);
    let mut vertices = crate::visuals::surfaces::Surfaces::new(&source).project(
        &crate::visuals::surfaces::Stamp {
            center: glam::Vec3::new(0.0, 0.0, 1000.0),
            normal: glam::Vec3::NEG_Z,
            radius: 600.0,
            depth: 30.0,
            seed: 0,
            shape: Some((glam::Vec3::Y, 0.8)),
        },
    );
    for vertex in &mut vertices {
        vertex.flags = (vertex.flags & !render_mesh::STAIN) | render_mesh::WET_CONTACT;
    }
    let count = vertices.len() as u32;
    let effect = renderer.upload_effects(render_effects::Geometry {
        vertices,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..count,
        }],
    });
    renderer
        .draw(&view, camera, &[&mesh], Some(&effect))
        .unwrap();
    let wet = read_pixel(&renderer, &target);
    assert!(
        wet[0] < dry[0] && wet[0] > 20,
        "wet ground: {dry:?} -> {wet:?}"
    );
    renderer.draw(&view, camera, &[&mesh], None).unwrap();
    assert_eq!(read_pixel(&renderer, &target), dry);
}

#[test]
#[ignore = "requires TENCHU_DISC and a GPU; verifies shadow culling against full-stage rendering with animated foliage"]
fn culled_shadows_match_full_stage_with_foliage() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let mut graphics = crate::settings::Graphics::default();
    graphics.set_preset(8);
    graphics.samples = 1;
    graphics.lighting.shadow_count = 2;
    graphics.lighting.resolution = 256;
    graphics.lighting.directional.resolution = 256;
    let (mut renderer, target) = fixture(&game.mission.vram);
    renderer.configure(&graphics);
    renderer.visual_time = 1.4;
    let mut models =
        crate::visuals::models::Models::build(&game.mission, &Default::default(), &graphics)
            .unwrap();
    renderer.bind_models(&mut models);
    let stage = renderer.upload(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let actor = renderer.upload(&render_mesh::characters(&game, Some(&models)));
    let mut shadow = renderer.upload_shadow(&render_mesh::shadow_stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let mut bodies = renderer.upload_shadow(&render_mesh::shadow_models(&game, Some(&models)));
    let origin = glam::Vec3::from(game.player().position.map(|n| n as f32).to_array());
    let lights = [
        crate::lighting::Light {
            position: origin + glam::Vec3::new(4000.0, -4000.0, 0.0),
            color: [1.0, 0.8, 0.6],
            radius: 18000.0,
        },
        crate::lighting::Light {
            position: origin + glam::Vec3::new(-4000.0, -3000.0, 3000.0),
            color: [0.6, 0.8, 1.0],
            radius: 18000.0,
        },
    ];
    let camera = game_camera(game.rendered_view, 1.0);
    let view = target.create_view(&Default::default());
    renderer
        .draw_lit(
            &view,
            camera,
            &[&stage, &actor],
            None,
            &lights,
            &[&shadow, &bodies],
        )
        .unwrap();
    let culled = read_pixels(&renderer, &target);
    shadow.shadow_chunks.clear();
    bodies.shadow_chunks.clear();
    renderer
        .draw_lit(
            &view,
            camera,
            &[&stage, &actor],
            None,
            &lights,
            &[&shadow, &bodies],
        )
        .unwrap();
    assert!(
        culled == read_pixels(&renderer, &target),
        "culling changed visible shadow pixels"
    );
}
