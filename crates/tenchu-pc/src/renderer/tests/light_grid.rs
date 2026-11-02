use super::*;
use crate::lighting::Light;
use bytemuck::Zeroable;
use glam::Vec3;

#[test]
#[ignore = "requires a GPU; diffuse and reflected lighting must respect every local light budget"]
fn overlapping_lights_accumulate_across_cell_groups() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut graphics = crate::settings::Graphics {
        fog: false,
        weapon_reflections: 100,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.shadows = false;
    graphics.lighting.ambient = 20;
    let light = Light {
        emission_radius: 0.0,
        position: Vec3::new(0.0, -300.0, 0.0),
        radius: 3000.0,
        color: [0.03, 0.02, 0.01],
    };
    for metal in [false, true] {
        let mut vertices = rectangle(1000, 128, None, false, false, true);
        if metal {
            for vertex in &mut vertices {
                vertex.flags |= render_mesh::WEAPON_METAL;
            }
        }
        let mesh = renderer.upload(&vertices);
        let capture = |renderer: &mut Renderer, lights: &[Light]| {
            renderer
                .draw_lit(
                    &view,
                    camera,
                    SceneMeshes::new(&[], &[&mesh]),
                    None,
                    lights,
                    &[],
                )
                .unwrap();
            read_pixels(renderer, &target)
        };
        for count in [0, 1, 3, 4, 5, 8, 9, 15, 16] {
            graphics.lighting.count = count;
            renderer.configure(&graphics);
            let actual = capture(&mut renderer, &[light; crate::lighting::MAX_LIGHTS]);
            graphics.lighting.count = 1;
            renderer.configure(&graphics);
            let expected = capture(
                &mut renderer,
                &[Light {
                    color: light.color.map(|c| c * f32::from(count)),
                    ..light
                }],
            );
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "{count} overlapping lights, metal={metal}"
            );
        }
    }
}

#[test]
#[ignore = "requires a GPU; local light budgets illuminate distant regions independently of source priority"]
fn distant_regions_keep_their_lights_when_priority_changes() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let points = [
        [-80_000.0, -10_000.0, 0.0],
        [80_000.0, -10_000.0, 0.0],
        [-80_000.0, 10_000.0, 0.0],
        [80_000.0, 10_000.0, 0.0],
    ];
    let wall = renderer.upload(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: points[i],
        color: [0.5; 3],
        opacity: 1.0,
        flags: render_mesh::DOUBLE_SIDED,
        ..Vertex::zeroed()
    }));
    let camera = Mat4::orthographic_rh(-60_000.0, 60_000.0, -60_000.0, 60_000.0, 1.0, 200_000.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, -100_000.0), Vec3::Z, Vec3::NEG_Y);
    let lights = [-40_000.0, 40_000.0].map(|x| Light {
        emission_radius: 0.0,
        position: Vec3::new(x, 0.0, -3000.0),
        radius: 12_000.0,
        color: [1.0; 3],
    });
    let mut grid = crate::renderer::light_grid::Grid::new(&renderer.device);
    assert!(grid.update(&renderer.queue, &lights, 1));
    assert!(!grid.update(&renderer.queue, &[lights[1], lights[0]], 1));
    let faded = lights.map(|light| Light {
        color: [0.5; 3],
        ..light
    });
    assert!(!grid.update(&renderer.queue, &faded, 1));
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.shadows = false;
    graphics.lighting.count = 1;
    graphics.lighting.ambient = 20;
    renderer.configure(&graphics);
    let capture = |renderer: &mut Renderer, lights: &[Light]| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&wall], &[]),
                None,
                lights,
                &[],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let reference = capture(&mut renderer, &lights);
    let reversed = capture(&mut renderer, &[lights[1], lights[0]]);
    assert_eq!(
        reference, reversed,
        "Player-relative shadow priority must not change which regions are lit"
    );
    let unlit = capture(&mut renderer, &[]);
    for x in [10, 53] {
        let pixel = (32 * 64 + x) * 4;
        assert!(
            reference[pixel] > unlit[pixel] + 50,
            "Distant region at x={x} lost its lamp"
        );
    }
    graphics.lighting.count = 0;
    renderer.configure(&graphics);
    assert_eq!(capture(&mut renderer, &lights), unlit);
}
