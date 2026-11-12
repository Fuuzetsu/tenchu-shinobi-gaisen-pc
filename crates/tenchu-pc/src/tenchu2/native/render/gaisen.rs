use super::*;
use tenchu_core::game::Game;

pub(super) struct Drawing {
    models: crate::visuals::models::Models,
    pub(super) terrain: crate::renderer::Mesh,
    vram_revision: Option<u64>,
    sprites: sprites::Sprites,
    ambient: sprites::Sprites,
}

impl Drawing {
    pub fn new(
        game: &Game,
        assets: &Assets,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let pack = if graphics.asset_pack {
            Pack::from_env(&game.mission)?
        } else {
            Pack::default()
        };
        let mut models = crate::visuals::models::Models::build(&game.mission, &pack, graphics)?;
        renderer.set_asset_pack(&pack);
        renderer.update_vram(&game.mission.vram);
        models.bind(|material| renderer.bind_material(material));
        let terrain = renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let sprites = sprites::Sprites::new(
            &assets.sprites.vram,
            assets
                .sprites
                .effects
                .iter()
                .chain(&assets.sprites.items)
                .copied(),
            renderer,
        );
        let ambient = sprites::Sprites::new(
            &game.mission.vram,
            game.mission
                .images
                .iter()
                .copied()
                .filter(|image| image.size.into_iter().all(|v| v > 0)),
            renderer,
        );
        Ok(Self {
            models,
            terrain,
            vram_revision: None,
            sprites,
            ambient,
        })
    }
}

impl super::Drawing {
    pub(super) fn ambient(
        &mut self,
        session: &super::super::Session,
        renderer: &mut Renderer,
        view: Matrix,
    ) -> Result<Vec<Surface>> {
        let Some(scene) = &session.gaisen else {
            return Ok(Vec::new());
        };
        let drawing = self
            .gaisen
            .as_mut()
            .ok_or("Missing original stage sprites")?;
        let graphics = renderer.graphics();
        let view = self.camera.map_or(view, |c| c.source_view());
        let mut surfaces = render_effects::ambient(
            &scene.director.source,
            view,
            render_effects::Style {
                blood: graphics.blood,
                ..Default::default()
            },
        );
        if graphics.weather.mode != crate::settings::WeatherMode::Original {
            surfaces.retain(|s| s.vertices[0].flags & render_mesh::ORIGINAL_WEATHER == 0);
        }
        drawing.ambient.retain(&mut surfaces, renderer)?;
        Ok(surfaces)
    }

    pub(super) fn update_gaisen(
        &mut self,
        session: &super::super::Session,
        renderer: &mut Renderer,
        debug: Option<&crate::debug::Debug>,
    ) -> Result<()> {
        let source = &session
            .gaisen
            .as_ref()
            .ok_or("Missing original cinematic")?
            .director
            .source;
        let drawing = self
            .gaisen
            .as_mut()
            .ok_or("Missing original cinematic models")?;
        if !self.cinematic {
            std::mem::swap(&mut self.scene.terrain, &mut drawing.terrain);
            drawing.vram_revision = None;
        }
        if drawing.vram_revision != Some(source.mission.vram.revision) {
            renderer.update_vram(&source.mission.vram);
            drawing.vram_revision = Some(source.mission.vram.revision);
        }
        self.cinematic = true;
        let view = self
            .camera
            .map_or(source.rendered_view, |c| c.source_view());
        let graphics = renderer.graphics();
        let mut effects = render_effects::build_presented(
            source,
            false,
            self.camera.map(|c| c.source_view()),
            graphics.blob_shadows,
            Some(&drawing.models),
            None,
            render_effects::Style {
                sharp_text: self.sharp_text,
                subtitle_language: self.subtitle_language,
                blood: graphics.blood,
                hinged: false,
                ..Default::default()
            },
        );
        if graphics.weather.mode != crate::settings::WeatherMode::Original {
            effects.remove_flag(render_mesh::ORIGINAL_WEATHER);
        }
        // Native mechanisms retain their exact current transforms across the scene.
        // Original miscellaneous effects still run, but do not draw another hatch.
        let mut native = Builder::new(view, &session.assets.level, graphics);
        for (index, object) in session.world.environment.objects.iter().enumerate() {
            if let Some(model) =
                tenchu_core::tenchu2::environment::scenery::Model::object(index, object)
            {
                native.scenery(
                    &self.models.scenery,
                    &session.assets.level,
                    &session.assets.resources.math,
                    &model,
                )?;
            }
        }
        self.models.loose_items(
            &session.world,
            &session.assets,
            &mut session.world.scene.clone(),
            &mut native,
        )?;
        native.particles(
            super::particles::Particles {
                scene: &session.world.scene,
                view: session.world.camera.view,
                sprites: &session.assets.sprites,
                frame: &session.frame.effects,
            },
            graphics,
            1.0,
            true,
        )?;
        drawing.sprites.retain(&mut native.surfaces, renderer)?;
        let mut native_effects = render_effects::surface_geometry_into(
            native.surfaces,
            std::mem::take(&mut self.scene.effect_vertices),
        );
        native_effects.append(effects);
        effects = native_effects;
        let mut actors = std::mem::take(&mut self.scene.actor_vertices);
        render_mesh::update_indexed_characters(
            &mut actors,
            source,
            Some(&drawing.models),
            self.camera.is_some(),
            None,
        );
        let base = actors.vertices.len();
        let start = actors.indices.len() as u32;
        actors.extend_triangles(
            native
                .actors
                .indices
                .iter()
                .map(|&i| native.actors.vertices[i as usize]),
        );
        if actors.indices.len() as u32 != start {
            actors.groups.push(render_mesh::IndexedGroup {
                vertices: base..actors.vertices.len(),
                indices: start..actors.indices.len() as u32,
            });
        }
        let mut geometry = Geometry {
            lamps: Vec::new(),
            backgrounds: Vec::new(),
            terrain: Vec::new(),
            props: native.props,
            actors,
            effects,
        };
        let camera = camera_matrix(source.rendered_view, self.camera, renderer.size());
        self.scene.lighting.update_emitters(
            &session.frame.effects.emitters,
            &renderer.graphics().lighting,
        );
        self.scene
            .lighting
            .update_gaisen(source, &renderer.graphics().lighting);
        self.scene.lighting.focus = self
            .camera
            .is_none()
            .then(|| glam::Vec3::from(source.player().position.to_array().map(|v| v as f32)));
        self.labels = self
            .scene
            .inspect_lights(debug, camera, renderer, &mut geometry);
        self.scene.prepare_environment(
            &mut geometry,
            environment::Frame {
                level: &session.assets.level,
                meshes: &self.models.scenery,
                math: &source.math,
                tick: u64::from(session.world.tick),
                camera,
                alpha: self.alpha,
                contact: None,
                world: None,
                scenery: None,
                sprites: &session.assets.sprites,
            },
            renderer,
        );
        self.scene.update_dynamic(geometry, renderer);
        self.texts =
            crate::fonts::game_text(source, false, self.sharp_text, self.subtitle_language);
        self.view = source.rendered_view;
        renderer.visual_time = source.tick as f32 / 30.0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenchu2::native::{Session, gaisen::Source};
    use tenchu_assets::script::Language;
    use tenchu_core::tenchu2::world::Options;

    pub(super) fn session(character: i16) -> Session {
        session_layout(character, 1)
    }

    fn session_layout(character: i16, layout: usize) -> Session {
        session_stage(0, character, layout)
    }

    pub(super) fn session_stage(stage: usize, character: i16, layout: usize) -> Session {
        let source = Source::open(
            std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
            stage,
            character,
            layout,
            Language::English,
        )
        .unwrap();
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut session = source
            .session(
                &disc,
                Options {
                    hero: 2,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
                4000,
            )
            .unwrap();
        session.step(0).unwrap();
        if !session.cinematic() {
            let scene = session.gaisen.as_ref().unwrap();
            let (event, target) = scene.director.source.events.active().next().unwrap();
            let event = &scene.director.source.mission.scripts.events[event];
            assert_eq!(event.mode, 1);
            let index = scene.director.native_actor(target).unwrap();
            let position = event
                .bounds
                .map(|[low, high]| (i32::from(low) + i32::from(high)) / 2 * 1000)
                .into();
            session.world.actors[index].actor.position = position;
        }
        for _ in 0..2000 {
            session.step(0).unwrap();
            let game = &session.gaisen.as_ref().unwrap().director.source;
            if game.cinematic.elapsed > 30 && crate::fonts::caption_sprites(game) > 0 {
                return session;
            }
        }
        panic!("No authored caption");
    }

    #[test]
    #[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; loose native pickups remain visible during original cinematics"]
    fn imported_host_cinematic_keeps_native_pickups() {
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        for character in 0..2 {
            let mut session = session_layout(character, 3);
            let initial = session.world.clone();
            for &(item, position) in &session.assets.gaisen_pickups.clone() {
                let mut camera = crate::debug::camera::Camera {
                    position: glam::Vec3::from_array(position.to_array().map(|v| v as f32))
                        - glam::Vec3::Y * 60.0,
                    yaw: 0.5,
                    pitch: -0.7,
                    fov: 45.0,
                };
                camera.position -= camera.forward() * 1800.0;
                let mut images = Vec::new();
                for present in [true, false] {
                    session.world = initial.clone();
                    if !present {
                        session.world.items.entries.clear();
                    }
                    let world = postcard::to_allocvec(&session.world).unwrap();
                    let source =
                        postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
                    let path = output.join(format!(
                        "cinematic-source-{character}-pickup-{item}-present-{present}.png"
                    ));
                    capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                        let mut drawing =
                            super::super::Drawing::new(&session, &graphics, renderer)?;
                        drawing.camera = Some(camera);
                        drawing.update(&session, renderer, None)?;
                        Ok((drawing.scene, camera.source_view(), Vec::new()))
                    })
                    .unwrap();
                    images.push(
                        crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                            .unwrap()
                            .rgba,
                    );
                    assert_eq!(world, postcard::to_allocvec(&session.world).unwrap());
                    assert_eq!(
                        source,
                        postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap()
                    );
                }
                let changed = images[0]
                    .chunks_exact(4)
                    .zip(images[1].chunks_exact(4))
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(
                    changed > 20,
                    "source {character}, item {item}: invisible native pickup ({changed} pixels)"
                );
            }
        }
    }

    #[test]
    #[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; loose meshes and every arrow flame phase retain native pixels under cinematic VRAM"]
    fn imported_host_cinematic_keeps_native_loose_item_textures() {
        use tenchu_core::tenchu2::items::{
            State,
            arrow::{Arrow, Phase},
            flight::Flight,
            grenade::Grenade,
        };
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let mut session = session(0);
        let world = &mut session.world;
        while !world.items.entries.is_empty() {
            world.items.remove(0, &mut world.collision);
        }
        let mut arrow = Arrow::launch(
            [0, 0, 4000].into(),
            [0, 0, 4096].into(),
            i32::MIN,
            &session.assets.resources.math,
            &mut world.random,
        );
        arrow.flame = true;
        world
            .items
            .insert(
                State::Arrow(arrow.clone()),
                &mut world.scene,
                &mut world.collision,
            )
            .unwrap();
        let parent = world.scene.add(
            None,
            Matrix {
                translation: [1000, 0, 4000].into(),
                ..Default::default()
            },
        );
        arrow.flame = false;
        arrow.fire_model = false;
        arrow.transform.translation = [0, 0, 0].into();
        arrow.phase = Phase::Embedded {
            age: 10,
            parent: Some(parent),
        };
        world
            .items
            .insert(State::Arrow(arrow), &mut world.scene, &mut world.collision)
            .unwrap();
        world
            .items
            .insert(
                State::Grenade(Grenade::launch(
                    world.player,
                    Flight {
                        position: [-1000, 0, 4000].into(),
                        ..Default::default()
                    },
                )),
                &mut world.scene,
                &mut world.collision,
            )
            .unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        let source = &session.gaisen.as_ref().unwrap().director.source;
        let mut phases = Vec::new();
        for phase in 0..4 {
            session.world.tick = phase + 1;
            let before = postcard::to_allocvec(&session.world).unwrap();
            let mut images = Vec::new();
            for cinematic in [false, true] {
                let path = output.join(format!(
                    "loose-items-phase-{phase}-cinematic-{cinematic}.png"
                ));
                capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                    let models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                    let mut cinematic_models =
                        Drawing::new(source, &session.assets, &graphics, renderer)?;
                    let mut output =
                        Builder::new(Matrix::default(), &session.assets.level, &graphics);
                    if cinematic {
                        models.loose_items(
                            &session.world,
                            &session.assets,
                            &mut session.world.scene.clone(),
                            &mut output,
                        )?;
                        cinematic_models
                            .sprites
                            .retain(&mut output.surfaces, renderer)?;
                    } else {
                        renderer.update_vram(&session.assets.sprites.vram);
                        models.loose_items(
                            &session.world,
                            &session.assets,
                            &mut session.world.scene.clone(),
                            &mut output,
                        )?;
                    }
                    assert!(!output.actors.indices.is_empty());
                    assert_eq!(output.surfaces.len(), 1);
                    let geometry = Geometry {
                        lamps: Vec::new(),
                        backgrounds: Vec::new(),
                        terrain: Vec::new(),
                        props: output.props,
                        actors: output.actors,
                        effects: render_effects::surface_geometry(output.surfaces),
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
                assert_eq!(before, postcard::to_allocvec(&session.world).unwrap());
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
            // Detached RGBA8 artwork rounds five-bit channels before the GPU;
            // shared VRAM decodes them directly to float. Coverage stays exact.
            assert!(
                difference <= 1,
                "phase {phase}: {changed} changed pixels, largest channel difference {difference}"
            );
            assert!(
                images[0]
                    .chunks_exact(4)
                    .zip(images[1].chunks_exact(4))
                    .all(|(a, b)| { (a[..3] == [0; 3]) == (b[..3] == [0; 3]) }),
                "phase {phase}: flame coverage changed"
            );
            phases.push(images.remove(0));
        }
        assert!(
            phases.windows(2).all(|p| p[0] != p[1]),
            "arrow flame was invisible"
        );
    }

    #[test]
    #[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; original captions/scenery and cached return to native gameplay"]
    fn imported_host_draws_cinematics_and_restores_native_materials() {
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        for (stage, character) in [(0, 0), (0, 1), (9, 1)] {
            let mut session = session_stage(stage, character, 1);
            let mut settings = crate::settings::Settings::default();
            settings.graphics.asset_pack = false;
            let world = postcard::to_allocvec(&session.world).unwrap();
            let source = postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
            for sharp in [false, true] {
                settings.sharp_text = sharp;
                for subtitle in [None, Some(Language::Japanese)] {
                    settings.subtitle_language = subtitle;
                    super::super::capture(
                        &session,
                        &settings,
                        &output.join(format!(
                            "source-{stage}-{character}-caption-{sharp}-{subtitle:?}.png"
                        )),
                    )
                    .unwrap();
                    assert_eq!(world, postcard::to_allocvec(&session.world).unwrap());
                    assert_eq!(
                        source,
                        postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap()
                    );
                }
            }
            settings.graphics.fog = false;
            let cached = output.join(format!("source-{stage}-{character}-native-cached.png"));
            let fresh = output.join(format!("source-{stage}-{character}-native-fresh.png"));
            let vram = session.assets.sprites.vram.clone();
            capture_frame(&vram, &settings.graphics, &cached, |renderer| {
                let mut drawing =
                    super::super::Drawing::new(&session, &settings.graphics, renderer)?;
                drawing.terrain.prepare(
                    scenery::Static {
                        level: &session.assets.level,
                        meshes: &drawing.models.scenery,
                        math: &session.assets.resources.math,
                        phases: Some(&session.world.environment.static_phases),
                    },
                    session.world.camera.view,
                    renderer,
                    &mut drawing.scene.terrain,
                );
                drawing.sharp_text = true;
                drawing.subtitle_language = settings.subtitle_language;
                drawing.update(&session, renderer, None)?;
                assert!(!drawing.texts.is_empty());
                for _ in 0..6000 {
                    session.step(0)?;
                    if !session.cinematic() {
                        break;
                    }
                }
                assert!(!session.cinematic());
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, drawing.view, drawing.texts))
            })
            .unwrap();
            super::super::capture(&session, &settings, &fresh).unwrap();
            assert_eq!(
                std::fs::read(cached).unwrap(),
                std::fs::read(fresh).unwrap(),
                "Native frame changed after cinematic material handoff"
            );
        }
    }

    #[test]
    #[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; native door poses remain visible beneath original cinematic effects"]
    fn imported_host_cinematic_does_not_cover_open_native_doors() {
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let mut session = session(0);
        let source = &session.gaisen.as_ref().unwrap().director.source;
        let spawn = source
            .mission
            .misc
            .spawns
            .iter()
            .find(|s| matches!(s.kind, tenchu_assets::misc::Kind::Door { .. }))
            .unwrap();
        let tenchu_assets::misc::Kind::Door { rotation, .. } = spawn.kind else {
            unreachable!()
        };
        let mut camera = crate::debug::camera::Camera {
            position: glam::Vec3::from_array(spawn.position.map(|v| v as f32)),
            yaw: f32::from(rotation) * std::f32::consts::TAU / 4096.0,
            pitch: 0.0,
            fov: 45.0,
        };
        camera.position -= camera.forward() * 6500.0;
        camera.position.y -= 1500.0;
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        let source_before =
            postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap();
        let mut images = Vec::new();
        for opened in [false, true] {
            if opened {
                for (index, object) in session
                    .world
                    .environment
                    .objects
                    .iter_mut()
                    .take(2)
                    .enumerate()
                {
                    object.yaw = object
                        .yaw
                        .wrapping_add(if index == 0 { 1024 } else { -1024 });
                }
            }
            let path = output.join(format!("cinematic-door-open-{opened}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
                drawing.camera = Some(camera);
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            })
            .unwrap();
            images.push(
                crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                    .unwrap()
                    .rgba,
            );
            assert_eq!(
                source_before,
                postcard::to_allocvec(&session.gaisen.as_ref().unwrap().director).unwrap()
            );
        }
        assert!(
            images[0]
                .chunks_exact(4)
                .zip(images[1].chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
                > 1000
        );
    }
}

#[cfg(test)]
mod ambient_tests;
#[cfg(test)]
mod particles_tests;
