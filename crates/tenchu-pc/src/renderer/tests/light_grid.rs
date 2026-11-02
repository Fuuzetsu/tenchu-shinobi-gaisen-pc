use super::*;
use crate::lighting::Light;
use bytemuck::Zeroable;
use glam::Vec3;

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
