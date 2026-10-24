use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a graphics adapter; the day cycle moves actual shadows, supports lighting on its own, and disabling it restores the original image"]
fn sun_and_moon_cycle_moves_shadows_and_restores_original_rendering() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let mut quad = |y: f32, size: f32| {
        let p = [
            [-size, y, -size],
            [size, y, -size],
            [-size, y, size],
            [size, y, size],
        ];
        renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.5; 3],
            flags: render_mesh::DOUBLE_SIDED,
            opacity: 1.0,
            ..Vertex::zeroed()
        }))
    };
    let ground = quad(0.0, 1500.0);
    let roof = quad(-500.0, 250.0);
    let camera = Mat4::orthographic_rh(-1500.0, 1500.0, -1500.0, 1500.0, 1.0, 10000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -3000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let capture = |renderer: &mut Renderer, occluded: bool| {
        let casters = if occluded {
            vec![&ground, &roof]
        } else {
            vec![&ground]
        };
        renderer
            .draw_lit(&view, camera, &[&ground], None, &[], &casters)
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer, false);
    let mut graphics = crate::settings::Graphics::default();
    graphics.sky.enabled = true;
    graphics.fog = false;
    graphics.lighting.directional.radius = 10;
    graphics.lighting.directional.resolution = 1024;
    renderer.configure(&graphics);
    let mut centers = Vec::new();
    for time in [540.0, 900.0] {
        renderer.daylight = Some(crate::visuals::sky::Daylight::at(time, &graphics.sky));
        let clear = capture(&mut renderer, false);
        let shaded = capture(&mut renderer, true);
        let shadow: Vec<_> = clear
            .chunks_exact(4)
            .zip(shaded.chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a[0] > b[0].saturating_add(15))
            .map(|(index, _)| (index % 64) as f32)
            .collect();
        assert!(shadow.len() > 30);
        centers.push(shadow.iter().sum::<f32>() / shadow.len() as f32);
    }
    assert!((centers[0] - centers[1]).abs() > 12.0, "{centers:?}");
    renderer.configure(&crate::settings::Graphics::default());
    assert_eq!(original, capture(&mut renderer, false));
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_SKY_REVIEW and a GPU; reviews time-of-day light on actual roof geometry with original and HD assets"]
fn day_cycle_on_actual_roof_geometry() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_SKY_REVIEW").expect("Set TENCHU_SKY_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let game = crate::session::Options::default().load(&assets).unwrap();
    let state = crate::session::tests::state(&game);
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let models = review.models(&game.mission).unwrap();
    let stage = review.renderer.upload(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
        * Mat4::look_at_rh(
            Vec3::new(-48300.0, -7600.0, -39400.0),
            Vec3::new(-44667.0, -5000.0, -34667.0),
            Vec3::NEG_Y,
        );
    for (name, minutes, shadows, shading) in [
        ("noon", 720.0, true, true),
        ("night", 0.0, true, true),
        ("noon-no-shadows", 720.0, false, true),
        ("noon-no-baked", 720.0, true, false),
    ] {
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(3);
        graphics.fog = false;
        graphics.shading = shading;
        graphics.sky.enabled = true;
        graphics.lighting.shadows = shadows;
        review.renderer.daylight = Some(crate::visuals::sky::Daylight::at(minutes, &graphics.sky));
        review.configure(graphics);
        review
            .capture(
                camera,
                &[&stage],
                None,
                true,
                &output.join(format!("{name}.png")),
            )
            .unwrap();
    }
    assert_eq!(state, crate::session::tests::state(&game));
}
