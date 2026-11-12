use super::*;
use tenchu_core::tenchu2::{effects, visibility::View};

fn output() -> std::path::PathBuf {
    let path = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; original/HD sprite materials from both games survive VRAM switches"]
fn imported_particle_materials_keep_both_games_artwork() {
    let output = output();
    let session = tests::session(0);
    let graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    let source = &session.gaisen.as_ref().unwrap().director.source;
    for original in [false, true] {
        let (vram, other, sprites): (_, _, Vec<_>) = if original {
            (
                &source.mission.vram,
                &session.assets.sprites.vram,
                source
                    .mission
                    .images
                    .iter()
                    .copied()
                    .filter(|i| i.size.into_iter().all(|v| v > 0))
                    .collect(),
            )
        } else {
            (
                &session.assets.sprites.vram,
                &source.mission.vram,
                session
                    .assets
                    .sprites
                    .effects
                    .iter()
                    .chain(&session.assets.sprites.items)
                    .copied()
                    .collect(),
            )
        };
        for hd in [false, true] {
            let mut images = Vec::new();
            for cinematic in [false, true] {
                let path = output.join(format!(
                    "particle-materials-original-{original}-hd-{hd}-cinematic-{cinematic}.png"
                ));
                capture_frame(vram, &graphics, &path, |renderer| {
                    let mut textures =
                        sprites::Sprites::new(vram, sprites.iter().copied(), renderer);
                    let mut drawing =
                        Builder::new(Matrix::default(), &session.assets.level, &graphics);
                    for (i, &image) in sprites.iter().enumerate() {
                        drawing.screen_sprite(
                            [
                                ((i % 16) as f32 - 7.5) * 18.0,
                                ((i / 16) as f32 - 3.0) * 18.0,
                                4000.0,
                            ],
                            image.size.map(f32::from),
                            image.size.map(|v| 12.0 / f32::from(v)),
                            0,
                            [1.0; 3],
                            Some(image),
                            2,
                        );
                        if hd {
                            drawing.blood_original_hd();
                        }
                    }
                    if cinematic {
                        renderer.update_vram(other);
                        textures.retain(&mut drawing.surfaces, renderer)?;
                    }
                    let geometry = Geometry {
                        lamps: Vec::new(),
                        backgrounds: Vec::new(),
                        terrain: Vec::new(),
                        props: Vec::new(),
                        actors: Default::default(),
                        effects: render_effects::surface_geometry(drawing.surfaces),
                    };
                    Ok((
                        SceneDrawing::new(geometry, renderer, &session.assets.level),
                        Matrix::default(),
                        Vec::new(),
                    ))
                })
                .unwrap();
                images.push(
                    crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                        .unwrap()
                        .rgba,
                );
            }
            let changed = images[0]
                .chunks_exact(4)
                .zip(images[1].chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            let difference = images[0]
                .iter()
                .zip(&images[1])
                .map(|(&a, &b)| a.abs_diff(b))
                .max()
                .unwrap();
            // Original GPU VRAM decoding retains five-bit precision until output;
            // detached RGBA8 material channels have already rounded to eight bits.
            assert!(
                difference <= 1,
                "Original {original} HD {hd}: {changed} changed pixels, largest channel difference {difference}"
            );
        }
    }
}

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; actual native explosion particles freeze during cinematics and respect the original letterbox"]
fn imported_cinematic_keeps_native_particles_frozen_and_visible() {
    let output = output();
    let mut session = tests::session(0);
    let source = &session.gaisen.as_ref().unwrap().director.source;
    let inverse = crate::renderer::view_matrix(source.rendered_view).inverse();
    let position = inverse
        .transform_point3(glam::Vec3::new(0.0, 0.0, -4000.0))
        .to_array()
        .map(|v| v as i32)
        .into();
    session.world.camera.view = source.rendered_view;
    let view = View {
        eye: source.camera.position,
        look: position,
        focus: source.player().position,
        matrix: source.rendered_view,
        distance: 16000,
    };
    session
        .world
        .effects
        .explosion(
            position,
            0,
            &session.assets.resources.math,
            &mut session.world.random,
        )
        .unwrap();
    for _ in 0..8 {
        session.frame.effects = session
            .world
            .effects
            .update(
                &mut session.world.scene,
                view,
                &session.world.terrain,
                &mut session.world.cursor,
                &session.assets.resources.math,
            )
            .unwrap();
    }
    assert!(session.frame.effects.sprites.len() > 5);
    assert!(!session.frame.effects.emitters.is_empty());
    let particles = postcard::to_allocvec(&session.world.effects).unwrap();
    let sprites = session.frame.effects.sprites.clone();
    for _ in 0..12 {
        session.step(0).unwrap();
        assert!(session.cinematic());
        assert_eq!(
            particles,
            postcard::to_allocvec(&session.world.effects).unwrap()
        );
        assert_eq!(sprites, session.frame.effects.sprites);
    }
    let graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    let world = postcard::to_allocvec(&session.world).unwrap();
    let director = postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
    let mut images = Vec::new();
    for visible in [true, false] {
        if !visible {
            session.frame.effects = effects::Frame::default();
        }
        let path = output.join(format!("native-explosion-cinematic-visible-{visible}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, drawing.view, drawing.texts))
        })
        .unwrap();
        images.push(
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                .unwrap()
                .rgba,
        );
        assert_eq!(world, postcard::to_allocvec(&session.world).unwrap());
        assert_eq!(
            director,
            postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap()
        );
    }
    let changed = images[0]
        .chunks_exact(4)
        .zip(images[1].chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > 500,
        "native explosion is invisible ({changed} changed pixels)"
    );
    for row in (0..90).chain(630..720) {
        let span = row * 1280 * 4..(row + 1) * 1280 * 4;
        assert!(
            images[0][span.clone()] == images[1][span],
            "native particles covered cinematic letterbox row {row}"
        );
    }
}
