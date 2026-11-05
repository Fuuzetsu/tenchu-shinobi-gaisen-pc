use super::*;
use crate::visuals::surfaces::{SourceRange, Stamp, Surfaces};
use bytemuck::Zeroable;
use glam::Vec3;
use std::sync::Arc;

#[test]
#[ignore = "requires a GPU; composition must preserve cutouts, overlapping receivers, rain and snow"]
fn blood_composition_keeps_receiver_boundaries_and_fallbacks() {
    let mut vram = Vram::default();
    for y in 0..16 {
        for x in 0..16 {
            vram.words[y * 1024 + x] = [0x7fff, 0, 31, 0x801f][(x / 4 + y / 4) % 4];
        }
    }
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = Mat4::orthographic_rh(-2000.0, 2000.0, -2000.0, 2000.0, 1.0, 15000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -5000.0, 0.0), Vec3::ZERO, Vec3::Z);
    let quad = |height: f32, slope: f32, tint: f32, flags: u32| {
        let p = [
            [-2000.0, -2000.0],
            [2000.0, -2000.0],
            [-2000.0, 2000.0],
            [2000.0, 2000.0],
        ];
        [0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: [p[i][0], height + p[i][0] * slope, p[i][1]],
            normal: Vec3::NEG_Y.to_array(),
            uv: [
                (p[i][0] + 2000.0) * 15.0 / 4000.0,
                (p[i][1] + 2000.0) * 15.0 / 4000.0,
            ],
            page: 256,
            color: [tint; 3],
            flags: render_mesh::TEXTURED | render_mesh::DOUBLE_SIDED | render_mesh::SCENERY | flags,
            opacity: 1.0,
            ..Vertex::zeroed()
        })
    };
    let mut options = crate::settings::Graphics {
        samples: 1,
        blood: crate::settings::Blood::modern(),
        ..Default::default()
    };
    options.post.enabled = true;
    options.lighting.enabled = true;
    options.lighting.shadows = false;
    for case in [
        "same",
        "two-floors",
        "slope",
        "foliage",
        "unknown",
        "distant-unknown",
        "varying-opacity",
    ] {
        let slope = if case == "slope" { 0.15 } else { 0.0 };
        let flags = if case == "foliage" {
            render_mesh::FOLIAGE
        } else {
            0
        };
        let mut vertices = quad(0.0, slope, 0.8, flags).to_vec();
        if case == "two-floors" {
            vertices.extend(quad(-0.25, 0.0, 0.2, 0));
        }
        let ground = renderer.upload(&vertices);
        let mut pages: Vec<_> = (0..12)
            .map(|i| {
                // Rebuilding and decoding identities must not alias distinct receivers.
                let mut projected = Surfaces::new(&vertices).project_decal(&Stamp {
                    center: Vec3::new(
                        (i % 3) as f32 * 450.0 - 450.0,
                        0.0,
                        (i / 3) as f32 * 450.0 - 450.0,
                    ),
                    normal: Vec3::NEG_Y,
                    radius: 1200.0,
                    depth: 800.0,
                    seed: i,
                    shape: None,
                });
                if case == "varying-opacity" {
                    for (index, vertex) in projected.vertices.iter_mut().enumerate() {
                        vertex.opacity *= if index % 3 == 0 { 0.25 } else { 0.75 };
                    }
                }
                let sources: Vec<SourceRange> = if case == "unknown" && i % 2 == 0 {
                    Vec::new()
                } else {
                    postcard::from_bytes(&postcard::to_allocvec(&projected.sources).unwrap())
                        .unwrap()
                };
                (Arc::new(projected.vertices), sources)
            })
            .collect();
        if case == "distant-unknown" {
            let mut vertices = pages[0].0.as_ref().clone();
            for vertex in &mut vertices {
                vertex.position[0] += 10000.0;
            }
            pages.push((Arc::new(vertices), Vec::new()));
        }
        for (_, sources) in &pages {
            for source in sources {
                let id = (1 << 22) - 1 - renderer.decal_sources.len() as u32;
                renderer
                    .decal_sources
                    .entry(source.source.clone())
                    .or_insert(id);
            }
        }
        renderer.set_projected_decals(
            pages
                .iter()
                .map(|(page, sources)| (page, sources.as_slice())),
        );
        for mode in [
            crate::settings::WeatherMode::Original,
            crate::settings::WeatherMode::Snow,
            crate::settings::WeatherMode::Rain,
        ] {
            options.weather.mode = mode;
            renderer.configure(&options);
            renderer.weather = crate::visuals::weather::SurfaceWeather {
                snow: if mode.snow() { 0.8 } else { 0.0 },
                wet: if mode.rain() { 0.7 } else { 0.0 },
                puddles: 0.0,
                time: 10.0,
                flash: 0.0,
            };
            let capture = |renderer: &mut Renderer| {
                renderer
                    .draw_lit(
                        &view,
                        camera,
                        SceneMeshes::new(&[&ground], &[]),
                        None,
                        &[],
                        &[&ground],
                    )
                    .unwrap();
                read_pixels(renderer, &target)
            };
            renderer.force_blood_composite = Some(false);
            let expected = capture(&mut renderer);
            renderer.force_blood_composite = Some(true);
            let actual = capture(&mut renderer);
            let error = actual
                .iter()
                .zip(&expected)
                .map(|(&a, &b)| a.abs_diff(b))
                .max()
                .unwrap();
            assert!(
                error
                    <= u8::from(matches!(
                        case,
                        "same" | "unknown" | "distant-unknown" | "varying-opacity"
                    )),
                "{case} {mode:?}: error {error}"
            );
            options.rendering.cached_blood = true;
            for samples in renderer.sample_counts.clone() {
                options.samples = samples;
                renderer.configure(&options);
                renderer.force_blood_composite = Some(false);
                for count in [pages.len(), 6, 0, pages.len()] {
                    renderer.set_projected_decals(
                        pages[..count]
                            .iter()
                            .map(|(page, sources)| (page, sources.as_slice())),
                    );
                    let cache = renderer.blood_cache.take();
                    let expected = capture(&mut renderer);
                    renderer.blood_cache = cache;
                    let actual = capture(&mut renderer);
                    assert_eq!(
                        actual,
                        capture(&mut renderer),
                        "rebuilding and reusing cached coverage differ"
                    );
                    assert_eq!(
                        renderer.blood_cache.as_ref().unwrap().useful(camera),
                        matches!(case, "same" | "distant-unknown") && count > 0
                    );
                    let error = actual
                        .iter()
                        .zip(&expected)
                        .map(|(&a, &b)| a.abs_diff(b))
                        .max()
                        .unwrap();
                    let mean = actual
                        .iter()
                        .zip(&expected)
                        .map(|(&a, &b)| f32::from(a.abs_diff(b)))
                        .sum::<f32>()
                        / actual.len() as f32;
                    let changed = actual
                        .chunks_exact(4)
                        .zip(expected.chunks_exact(4))
                        .filter(|(a, b)| a.iter().zip(*b).any(|(&a, &b)| a.abs_diff(b) > 6))
                        .count();
                    // MSAA can move a few cutout-edge samples; interiors must still match.
                    assert!(
                        (error <= 6 || samples > 1 && changed <= 8) && mean < 0.15,
                        "cached {case} {mode:?} {samples}x MSAA pages {count}: max {error}, mean {mean}, changed {changed}"
                    );
                }
            }
            options.samples = 1;
            options.rendering.cached_blood = false;
            renderer.configure(&options);
        }
    }
    renderer.force_blood_composite = None;
    let vertices = quad(0.0, 0.0, 0.8, 0);
    let projected = Surfaces::new(&vertices).project_decal(&Stamp {
        center: Vec3::ZERO,
        normal: Vec3::NEG_Y,
        radius: 1200.0,
        depth: 1.0,
        seed: 1,
        shape: None,
    });
    let page = Arc::new(projected.vertices);
    renderer.set_projected_decals([(&page, projected.sources.as_slice())]);
    assert!(!decals::composite::useful(&renderer, camera));
    let many: Vec<_> = (0..128).map(|_| Arc::new(page.as_ref().clone())).collect();
    renderer.set_projected_decals(many.iter().map(|p| (p, projected.sources.as_slice())));
    assert!(decals::composite::useful(&renderer, camera));
    assert!(!decals::composite::useful(
        &renderer,
        camera * Mat4::from_scale(Vec3::splat(0.001))
    ));
    options.samples = renderer
        .sample_counts
        .iter()
        .copied()
        .find(|&s| s > 1)
        .unwrap_or(1);
    if options.samples > 1 {
        renderer.configure(&options);
        assert!(!decals::composite::useful(&renderer, camera));
    }
    options.samples = 1;
    options.post.enabled = false;
    renderer.configure(&options);
    assert!(!decals::composite::useful(&renderer, camera));
    options.post.enabled = true;
    options.lighting.enabled = false;
    options.sky.enabled = false;
    options.weather.mode = crate::settings::WeatherMode::Original;
    renderer.configure(&options);
    assert!(!decals::composite::useful(&renderer, camera));
}
