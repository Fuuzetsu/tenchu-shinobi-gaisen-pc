use super::*;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; wind must deform both scene geometry and directional shadow casters and disable exactly"]
fn wind_moves_foliage_and_its_shadow_without_moving_roots() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let quad = |y: f32, size: f32, flags: u32| {
        let p = [
            [-size, y, -size],
            [size, y, -size],
            [-size, y, size],
            [size, y, size],
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: p[i],
            color: [0.6; 3],
            opacity: 1.0,
            flags: render_mesh::DOUBLE_SIDED | flags,
            ..Vertex::zeroed()
        })
    };
    let ground = renderer.upload(&quad(0.0, 1500.0, 0));
    let canopy = renderer.upload(&quad(
        -500.0,
        250.0,
        render_mesh::FOLIAGE | 255 << render_mesh::WIND_WEIGHT_SHIFT,
    ));
    let anchored = renderer.upload(&quad(-500.0, 250.0, render_mesh::FOLIAGE));
    let camera = Mat4::orthographic_rh(-1500.0, 1500.0, -1500.0, 1500.0, 1.0, 10000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -3000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let mut settings = crate::settings::Graphics::default();
    settings.sky.enabled = true;
    settings.fog = false;
    settings.lighting.directional.radius = 10;
    settings.lighting.directional.resolution = 1024;
    renderer.daylight = Some(crate::visuals::sky::Daylight::at(600.0, &settings.sky));
    renderer.configure(&settings);
    let capture = |renderer: &mut Renderer, mesh: &Mesh, cast: bool| {
        let scene = if cast { &ground } else { mesh };
        renderer
            .draw_lit(
                &view,
                camera,
                crate::renderer::SceneMeshes::new(&[scene], &[]),
                None,
                &[],
                &[&ground, mesh],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let original = capture(&mut renderer, &canopy, false);
    settings.motion.wind = 100;
    renderer.configure(&settings);
    let first = capture(&mut renderer, &canopy, false);
    let shadow = capture(&mut renderer, &canopy, true);
    assert!(
        shadow != capture(&mut renderer, &ground, true),
        "Canopy must cast a visible shadow"
    );
    let roots = capture(&mut renderer, &anchored, false);
    renderer.visual_time = 3.0;
    assert!(
        first != capture(&mut renderer, &canopy, false),
        "Breeze must move leaves"
    );
    assert!(
        shadow != capture(&mut renderer, &canopy, true),
        "Breeze must move their shadow"
    );
    assert!(
        roots == capture(&mut renderer, &anchored, false),
        "Roots must stay anchored"
    );
    settings.motion.wind = 0;
    renderer.configure(&settings);
    assert!(
        original == capture(&mut renderer, &canopy, false),
        "Disabling breeze must restore the original image"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_MOTION_REVIEW and a GPU; inspect actual hair, cloth, equipment and matching shadow geometry"]
fn secondary_motion_on_actual_characters_keeps_weapons_and_gameplay() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let output = std::path::PathBuf::from(
        std::env::var_os("TENCHU_MOTION_REVIEW").expect("Set TENCHU_MOTION_REVIEW"),
    );
    std::fs::create_dir_all(&output).unwrap();
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    for character in 0..2 {
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(3);
        graphics.models.characters = 1;
        graphics.motion = crate::settings::Motion {
            hair: true,
            cloth: true,
            equipment: true,
            ..Default::default()
        };
        graphics.fog = false;
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        review.configure(graphics.clone());
        let mut models = review.models(&game.mission).unwrap();
        let stage = review.renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let mut motion = crate::visuals::motion::Motion::default();
        let mut peak = 0.0;
        let mut best = None;
        for tick in 0..240 {
            use tenchu_core::input::{CROSS, SQUARE, UP};
            let buttons = if tick % 120 < 30 {
                UP
            } else if tick % 120 == 40 {
                CROSS
            } else if tick % 120 == 85 {
                SQUARE
            } else {
                0
            };
            game.step(buttons).unwrap();
            let state = crate::session::tests::state(&game);
            motion.tick(&game, &models, &graphics.motion);
            models.motion = motion.sample(1.0);
            for weapon in game.player().weapons.iter().flatten() {
                assert!(
                    !models.motion.keys().any(|part| matches!(
                        part,
                        crate::visuals::interpolation::Part::Actor { model, .. }
                            if *model == weapon.model
                    )),
                    "Equipped weapons must keep their original rigid transforms"
                );
            }
            let amount = models
                .motion
                .values()
                .flat_map(|p| p.offsets)
                .map(|p| p.length())
                .fold(0.0, f32::max);
            if amount > peak {
                peak = amount;
                let vertices = render_mesh::characters_presented(&game, Some(&models), true, None);
                let casters = render_mesh::shadow_models_presented(&game, Some(&models), None);
                let mut shared = Vec::new();
                render_mesh::update_shadow_models(&mut shared, &game, Some(&models), None, true);
                shared.extend_from_slice(&vertices);
                let canonical = |vertices: &[Vertex]| {
                    let mut bytes: Vec<[u32; 16]> = vertices
                        .iter()
                        .map(|v| {
                            let mut v = *v;
                            v.flags &= !render_mesh::FOG;
                            *bytemuck::from_bytes(bytemuck::bytes_of(&v))
                        })
                        .collect();
                    bytes.sort_unstable();
                    bytes
                };
                assert_eq!(canonical(&shared), canonical(&casters));
                let shadow_positions: std::collections::BTreeSet<_> = casters
                    .iter()
                    .map(|v| v.position.map(f32::to_bits))
                    .collect();
                assert!(
                    vertices
                        .iter()
                        .all(|v| shadow_positions.contains(&v.position.map(f32::to_bits)))
                );
                let frame = std::mem::take(&mut models.motion);
                let original = render_mesh::characters_presented(&game, Some(&models), true, None);
                models.motion = frame;
                assert_eq!(vertices.len(), original.len());
                let moved = vertices
                    .iter()
                    .zip(&original)
                    .filter(|(a, b)| Vec3::from(a.position).distance(Vec3::from(b.position)) > 0.01)
                    .count();
                let head = game.player().rendered[2];
                let center = Vec3::from(head.translation.map(|v| v as f32).to_array());
                let orientation = crate::visuals::models::rotation(head);
                let camera =
                    Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 10.0, 200000.0)
                        * Mat4::look_at_rh(
                            center + orientation * Vec3::new(450.0, 80.0, 800.0),
                            center + Vec3::Y * 130.0,
                            Vec3::NEG_Y,
                        );
                best = Some((tick, moved, camera, vertices, original));
            }
            assert_eq!(state, crate::session::tests::state(&game));
        }
        let (tick, moved, camera, vertices, original) = best.unwrap();
        assert!(
            peak > 1.0 && moved > 10,
            "character {character}: peak {peak}, moved {moved}"
        );
        eprintln!("character {character}, tick {tick}: peak {peak:.2}, moved {moved} vertices");
        for (label, vertices) in [("motion", vertices), ("rigid", original)] {
            let mesh = review.renderer.upload(&vertices);
            review
                .capture(
                    camera,
                    &[&stage, &mesh],
                    None,
                    true,
                    &output.join(format!("c{character}-{label}.png")),
                )
                .unwrap();
        }
    }
}
