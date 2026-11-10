use super::*;
use crate::tenchu2::native::gaisen::Source;
use tenchu_assets::{game::GameAssets, misc::Kind, script::Language, stage::MissionAssets};
use tenchu_core::tenchu2::world::Options;

struct Silent;
impl tenchu_core::tenchu2::environment::Events for Silent {
    fn emit(
        &mut self,
        _: tenchu_core::tenchu2::environment::Event,
        _: &mut tenchu_core::random::Random,
    ) -> Result<()> {
        Ok(())
    }
}

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; authored ambience is visible under native HUD and fades, with Original/Modern settings and immutable paused rendering"]
fn imported_ambient_artwork_lighting_weather_and_fades() {
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let original = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut cases = BTreeMap::new();
    for stage in 0..11 {
        let mission = MissionAssets::load(&original, stage, 0, 1).unwrap();
        for object in &mission.misc.spawns {
            let name = match object.kind {
                Kind::Fire => "fire",
                Kind::Bonfire(_) => "bonfire",
                Kind::Puff(_) => "puff",
                Kind::Snowfall => "snow",
                Kind::Sprite(_) => "sprite",
                _ => continue,
            };
            cases
                .entry(name)
                .or_insert((stage, Position::from(object.position)));
        }
    }
    assert_eq!(cases.len(), 5);
    for (name, (stage, position)) in cases {
        let source = Source::open(path.as_ref(), stage, 0, 1, Language::English).unwrap();
        let mut session = source
            .session(
                &disc,
                Options {
                    hero: stage % 3,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
                4000,
            )
            .unwrap();
        session.gaisen.as_mut().unwrap().director.source.events = Default::default();
        session.step(0).unwrap();
        assert!(!session.cinematic());
        let center =
            glam::Vec3::from(position.to_array().map(|v| v as f32)) - glam::Vec3::Y * 600.0;
        let mut camera = crate::debug::camera::Camera {
            position: center,
            yaw: 0.5,
            pitch: -0.15,
            fov: 45.0,
        };
        camera.position -= camera.forward() * 5000.0;
        session.world.camera.eye = camera.position.to_array().map(|v| v as i32).into();
        session.world.camera.look = center.to_array().map(|v| v as i32).into();
        session.world.camera.view = camera.source_view();
        session.world.actors[session.world.player].actor.position = position;
        session.frame.scenery = session
            .world
            .update_environment(&session.assets, &mut Silent)
            .unwrap();
        for tick in 2..=120 {
            session.world.tick = tick;
            let director = &mut session.gaisen.as_mut().unwrap().director;
            director.source.effects.begin_tick();
            director.source.scene.finish_tick();
            director.advance_ambient(&mut session.world).unwrap();
        }
        let snapshot = postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
        let world = postcard::to_allocvec(&session.world).unwrap();
        for preset in [0, crate::settings::presets::MODERN_HIGH] {
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.set_preset(preset);
            let mut images = Vec::new();
            for present in [false, true] {
                session.gaisen.as_mut().unwrap().director =
                    postcard::from_bytes(&snapshot).unwrap();
                if !present {
                    let source = &mut session.gaisen.as_mut().unwrap().director.source;
                    source.effects = Default::default();
                    source.misc.drawings.clear();
                    source.misc.objects.clear();
                }
                let frozen =
                    postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
                let file = output.join(format!(
                    "ambient-stage-{stage}-{name}-preset-{preset}-present-{present}.png"
                ));
                capture_frame(&session.assets.sprites.vram, &graphics, &file, |renderer| {
                    let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
                    drawing.camera = (preset != 0).then_some(camera);
                    if present && name == "snow" {
                        let original = drawing.ambient(&session, renderer, camera.source_view())?;
                        assert!(
                            original
                                .iter()
                                .any(|s| s.vertices[0].flags & render_mesh::ORIGINAL_WEATHER != 0)
                        );
                        let mut changed = graphics.clone();
                        changed.weather.mode = crate::settings::WeatherMode::Snow;
                        renderer.configure(&changed);
                        assert!(
                            drawing
                                .ambient(&session, renderer, camera.source_view())?
                                .iter()
                                .all(|s| s.vertices[0].flags & render_mesh::ORIGINAL_WEATHER == 0)
                        );
                        renderer.configure(&graphics);
                    }
                    for _ in 0..3 {
                        drawing.update(&session, renderer, None)?;
                    }
                    Ok((drawing.scene, camera.source_view(), drawing.texts))
                })
                .unwrap();
                images.push(
                    crate::visuals::pack::decode_png(&std::fs::read(file).unwrap())
                        .unwrap()
                        .rgba,
                );
                assert_eq!(
                    frozen,
                    postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap()
                );
                assert_eq!(world, postcard::to_allocvec(&session.world).unwrap());
            }
            let changed = images[0]
                .chunks_exact(4)
                .zip(images[1].chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            assert!(
                changed > 20,
                "stage {stage} {name} preset {preset}: only {changed} affected pixels"
            );
            eprintln!("stage {stage} {name} preset {preset}: {changed} affected pixels");
        }
        if name == "fire" {
            session.frame.transition_fade = Some(255);
            let mut images = Vec::new();
            for present in [false, true] {
                session.gaisen.as_mut().unwrap().director =
                    postcard::from_bytes(&snapshot).unwrap();
                if !present {
                    let source = &mut session.gaisen.as_mut().unwrap().director.source;
                    source.effects = Default::default();
                    source.misc.drawings.clear();
                }
                let file = output.join(format!("ambient-fade-present-{present}.png"));
                capture(
                    &session,
                    &crate::settings::Settings {
                        graphics: Graphics {
                            asset_pack: false,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    &file,
                )
                .unwrap();
                images.push(
                    crate::visuals::pack::decode_png(&std::fs::read(file).unwrap())
                        .unwrap()
                        .rgba,
                );
            }
            assert_eq!(
                images[0], images[1],
                "ambient particles drew in front of the native fade"
            );
        }
    }
}
