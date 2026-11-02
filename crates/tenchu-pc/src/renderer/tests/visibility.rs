use super::*;

#[test]
#[ignore = "requires a GPU; depth rejection must preserve cutouts, coplanar order, backfaces and blended effects"]
fn visibility_pass_preserves_ordered_surfaces_and_effects() {
    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0, 0x03e0, 0x801f][(x / 3 + y / 4) % 3];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut cutout = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        true,
        false,
        false,
    );
    for v in &mut cutout {
        v.uv = [
            (v.position[0] + 600.0) / 80.0,
            (v.position[1] + 600.0) / 80.0,
        ];
    }
    let mut vertices = cutout.clone();
    vertices.extend(rectangle(1000, 200, None, false, false, false));
    vertices.extend(rectangle(1000, 40, None, false, false, false));
    vertices.extend(rectangle(800, 255, None, false, true, false));
    let mut alpha = rectangle(900, 80, None, false, false, false);
    for v in &mut alpha {
        v.flags |= render_mesh::ALPHA;
        v.opacity = 0.4;
        v.position[0] -= 400.0;
    }
    vertices.extend_from_slice(&alpha);
    let terrain = renderer.upload(&vertices);
    let mut actor = renderer.upload(&[]);
    let mut effects = render_effects::Geometry {
        vertices: cutout,
        batches: vec![render_effects::Batch {
            blend: 1,
            range: 0..6,
        }],
    };
    effects.append(render_effects::Geometry {
        vertices: alpha,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..6,
        }],
    });
    let effects = renderer.upload_effects(effects);
    let mut graphics = crate::settings::Graphics::default();
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 100;
    for samples in renderer.sample_counts.clone() {
        graphics.samples = samples;
        for post in [false, true] {
            graphics.post.enabled = post;
            renderer.configure(&graphics);
            for x in [-500.0, 0.0, 500.0] {
                let mut moving = rectangle(750, 100, None, false, false, false);
                for v in &mut moving {
                    v.position[0] = v.position[0] * 0.3 + x;
                    v.position[2] += v.position[1] * 0.2;
                }
                renderer.update_indexed_mesh(&mut actor, &mut indexed_geometry(&moving));
                renderer.skip_prepass = true;
                renderer
                    .draw(&view, camera, &[&terrain, &actor], Some(&effects))
                    .unwrap();
                let expected = read_pixels(&renderer, &target);
                renderer.skip_prepass = false;
                renderer
                    .draw(&view, camera, &[&terrain, &actor], Some(&effects))
                    .unwrap();
                assert_eq!(
                    read_pixels(&renderer, &target),
                    expected,
                    "samples {samples}, HDR {post}, actor {x}"
                );
            }
        }
    }
}
