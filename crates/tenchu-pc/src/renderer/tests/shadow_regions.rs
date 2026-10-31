use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; partial shadow updates must erase old silhouettes while retaining fixed and wind-animated casters"]
fn moving_shadow_regions_match_full_redraws() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let quad = |center: Vec3, size: f32, flags| {
        let points = [
            center + Vec3::new(-size, 0.0, -size),
            center + Vec3::new(size, 0.0, -size),
            center + Vec3::new(-size, 0.0, size),
            center + Vec3::new(size, 0.0, size),
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: points[i].to_array(),
            color: [0.5; 3],
            flags: flags | render_mesh::DOUBLE_SIDED,
            opacity: 1.0,
            ..Vertex::zeroed()
        })
    };
    let ground = renderer.upload(&quad(Vec3::ZERO, 5000.0, render_mesh::SCENERY));
    let wall = renderer.upload(&quad(
        Vec3::new(1200.0, -900.0, 0.0),
        350.0,
        render_mesh::SCENERY,
    ));
    let tree = renderer.upload(&quad(
        Vec3::new(-1300.0, -900.0, 0.0),
        350.0,
        render_mesh::SCENERY | render_mesh::FOLIAGE | 255 << render_mesh::WIND_WEIGHT_SHIFT,
    ));
    let mut actor = renderer.upload(&[]);
    let mut lights = [crate::lighting::Light {
        position: Vec3::new(300.0, -4000.0, 200.0),
        color: [1.0; 3],
        radius: 15000.0,
    }];
    let camera = Mat4::orthographic_rh(-5000.0, 5000.0, -5000.0, 5000.0, 1.0, 20000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -9000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.resolution = 512;
    graphics.lighting.source_size = 0;
    graphics.motion.wind = 100;
    let draw = |renderer: &mut Renderer, lights: &[crate::lighting::Light], actor: &Mesh| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&ground], &[]),
                None,
                lights,
                &[&ground, &wall, &tree, actor],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut partial = 0;
    for mode in 0..3 {
        graphics.lighting.directional.intensity = if mode == 1 { 100 } else { 0 };
        graphics.lighting.directional.radius = 8;
        graphics.lighting.directional.resolution = 512;
        graphics.weather.mode = if mode == 2 {
            crate::settings::WeatherMode::Snow
        } else {
            crate::settings::WeatherMode::Original
        };
        renderer.configure(&graphics);
        renderer.weather.snow = f32::from(mode == 2);
        for frame in 0..18 {
            renderer.visual_time = frame as f32 * 0.3;
            let center = Vec3::new(-1800.0 + frame as f32 * 210.0, -1500.0, -900.0);
            let mut vertices = quad(center, 400.0, render_mesh::SCENERY).to_vec();
            match frame {
                4 | 5 => vertices.clear(),
                8 => vertices.extend(quad(lights[0].position, 600.0, 0)),
                9 => lights[0].position.x += 500.0,
                12 => actor = renderer.upload(&vertices),
                _ => (),
            }
            renderer.update_mesh(&mut actor, &mut vertices);
            let cached = draw(&mut renderer, &lights, &actor);
            if mode == 0
                && renderer.shadows.rendered_area
                    < renderer.shadows.rendered_faces as u64 * 512 * 512
            {
                partial += 1;
            }
            renderer.shadows.draw_all_faces = true;
            let reference = draw(&mut renderer, &lights, &actor);
            renderer.shadows.draw_all_faces = false;
            assert!(
                cached == reference,
                "Shadow mismatch in mode {mode}, frame {frame}"
            );
            assert!(draw(&mut renderer, &lights, &actor) == reference);
            assert_eq!(renderer.shadows.rendered_faces, 0);
        }
    }
    assert!(
        partial >= 10,
        "Moving casters still refresh entire shadow maps"
    );
}
