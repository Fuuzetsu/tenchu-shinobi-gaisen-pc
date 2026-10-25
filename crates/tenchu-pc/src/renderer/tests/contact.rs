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
