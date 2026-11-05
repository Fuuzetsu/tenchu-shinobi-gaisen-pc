use super::*;
use crate::lighting::Light;
use glam::Vec3;

fn quad(center: Vec3, extent: Vec3, color: [f32; 3]) -> Vec<Vertex> {
    let mut vertices = rectangle(0, 128, None, false, false, true);
    for v in &mut vertices {
        v.position = (center + Vec3::from(v.position) / 600.0 * extent).to_array();
        v.color = color;
    }
    vertices
}

#[test]
#[ignore = "requires GPU; flame animation must move shadows without invalidating cached shadow maps"]
fn dancing_flames_reuse_static_shadow_faces_and_leave_other_lights_steady() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let matrix = Mat4::orthographic_rh(-1000.0, 1000.0, -1000.0, 1000.0, 1.0, 5000.0)
        * Mat4::look_to_rh(Vec3::new(0.0, 0.0, -2000.0), Vec3::Z, Vec3::NEG_Y);
    let ground = renderer.upload(&quad(Vec3::ZERO, Vec3::splat(1000.0), [0.4; 3]));
    let blocker = renderer.upload(&quad(
        Vec3::new(250.0, -250.0, -500.0),
        Vec3::splat(160.0),
        [0.4; 3],
    ));
    let lamp = Light {
        flame: true,
        emission_radius: 0.0,
        position: Vec3::new(1000.0, -800.0, -1000.0),
        color: [1.0; 3],
        radius: 5000.0,
    };
    let draw = |renderer: &mut Renderer, lamp: Light, time| {
        renderer.visual_time = time;
        renderer
            .draw_lit(
                &view,
                matrix,
                SceneMeshes::new(&[&ground], &[]),
                None,
                &[lamp],
                &[&ground, &blocker],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.source_size = 0;
    graphics.lighting.ambient = 20;
    graphics.lighting.softness = 1;
    renderer.configure(&graphics);
    let original = draw(&mut renderer, lamp, 0.0);
    assert_eq!(draw(&mut renderer, lamp, 0.5), original);
    assert_eq!(renderer.shadows.rendered_faces, 0);
    graphics.lighting.flame_motion = 100;
    renderer.configure(&graphics);
    let a = draw(&mut renderer, lamp, 0.0);
    let b = draw(&mut renderer, lamp, 0.5);
    assert_ne!(a, b);
    assert!(
        a.chunks_exact(4)
            .zip(b.chunks_exact(4))
            .any(|(a, b)| a[0].abs_diff(b[0]) > 20),
        "shadow edges must move, beyond the gentle brightness flicker"
    );
    assert_eq!(
        renderer.shadows.rendered_faces, 0,
        "flickering never rerenders static casters"
    );
    assert_eq!(
        draw(
            &mut renderer,
            Light {
                flame: false,
                ..lamp
            },
            0.5
        ),
        original
    );
    graphics.lighting.flame_motion = 0;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, lamp, 0.5), original);
}

#[test]
#[ignore = "requires GPU; heat affects only background behind flames, never foreground/HUD, and clears without stale distortion"]
fn heat_shimmer_depth_and_toggle_survive_msaa_and_resolved_depth() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut vertices = Vec::new();
    for y in -10..10 {
        for x in -10..10 {
            let c = if (x + y) % 2 == 0 { 0.15 } else { 0.7 };
            vertices.extend(quad(
                Vec3::new(x as f32 * 120.0 + 60.0, y as f32 * 120.0 + 60.0, 2000.0),
                Vec3::splat(60.0),
                [c; 3],
            ));
        }
    }
    vertices.extend(quad(
        Vec3::new(400.0, 0.0, 500.0),
        Vec3::new(400.0, 2000.0, 0.0),
        [0.0, 0.5, 0.0],
    ));
    let mesh = renderer.upload(&vertices);
    let mut hud = rectangle(1000, 255, None, false, false, false);
    for v in &mut hud {
        v.position = [v.position[0] / 30.0 - 60.0, v.position[1] / 30.0, 0.01];
        v.color = [0.0, 0.0, 0.8];
        v.flags = render_mesh::SCREEN | render_mesh::UI;
    }
    let effects = renderer.upload_effects(render_effects::Geometry {
        batches: vec![render_effects::Batch {
            blend: 0,
            range: 0..hud.len() as u32,
        }],
        vertices: hud,
    });
    let light = Light {
        flame: true,
        emission_radius: 0.0,
        position: Vec3::new(-100.0, 600.0, 1000.0),
        color: [0.0; 3],
        radius: 5000.0,
    };
    let draw = |renderer: &mut Renderer, lights: &[Light], time| {
        renderer.visual_time = time;
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[&mesh], &[]),
                Some(&effects),
                lights,
                &[],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.shadows = false;
    graphics.lighting.ambient = 100;
    graphics.post.enabled = true;
    graphics.post.bloom = 0;
    graphics.post.sharpen = 0;
    graphics.post.vignette = 0;
    for samples in renderer
        .sample_counts
        .clone()
        .into_iter()
        .filter(|n| *n <= 4)
    {
        graphics.samples = samples;
        for ao in [0, 10] {
            graphics.post.occlusion = ao;
            graphics.lighting.heat_shimmer = 0;
            renderer.configure(&graphics);
            let original = draw(&mut renderer, &[light], 0.0);
            graphics.lighting.heat_shimmer = 100;
            renderer.configure(&graphics);
            let a = draw(&mut renderer, &[light], 0.0);
            let b = draw(&mut renderer, &[light], 0.35);
            let changes = a.iter().zip(&b).filter(|(x, y)| x != y).count();
            assert!(
                changes > 8,
                "heat didn't animate: {changes}, samples {samples}, ao {ao}"
            );
            for ((old, a), b) in original
                .chunks_exact(4)
                .zip(a.chunks_exact(4))
                .zip(b.chunks_exact(4))
            {
                if (old[1] > old[0] + 20 && old[1] > old[2] + 20)
                    || (old[2] > old[0] + 20 && old[2] > old[1] + 20)
                {
                    assert_eq!(old, a, "foreground or HUD distortion");
                    assert_eq!(old, b, "foreground or HUD distortion");
                }
            }
            assert_eq!(
                draw(&mut renderer, &[], 0.35),
                original,
                "removed flames leave no stale heat"
            );
            assert_eq!(
                draw(
                    &mut renderer,
                    &[Light {
                        flame: false,
                        ..light
                    }],
                    0.35
                ),
                original
            );
            graphics.lighting.heat_shimmer = 0;
            renderer.configure(&graphics);
            assert_eq!(draw(&mut renderer, &[light], 0.35), original);
        }
    }
}
