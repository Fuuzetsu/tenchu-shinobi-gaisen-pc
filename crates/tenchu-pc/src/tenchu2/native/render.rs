use super::interpolation;
use crate::visuals::interpolation::Frame as PoseFrame;
use crate::{
    render_effects::{self, Surface},
    render_mesh::{self, Vertex},
    renderer::Renderer,
    settings::Graphics,
    visuals::{
        catalog::{self, Role},
        models::Meshes,
        pack::Pack,
    },
};
use std::collections::BTreeMap;
use tenchu_assets::{
    Result,
    model::Part,
    tenchu2::level::{Level, Shading},
    texture::{Image, Vram},
};
use tenchu_core::{
    math::{Matrix, Position},
    tenchu2::{
        effects::surface::{Blend, Image as SurfaceImage},
        world::{Assets, Frame, World},
    },
};

mod background;
mod blood;
mod custom;
mod environment;
mod hud;
mod lamps;
pub(super) mod lighting;
mod motion;
mod scenery;
mod story;
#[cfg(test)]
mod tests;
mod weapons;
mod wounds;
pub(super) use story::Drawing as StoryDrawing;
pub(super) use story::capture as capture_story;

pub struct Drawing {
    models: Models,
    motion: crate::visuals::motion::Motion,
    wounds: wounds::Wounds,
    scene: SceneDrawing,
    view: Matrix,
    pub camera: Option<crate::debug::camera::Camera>,
    pub alpha: f32,
    pub show_hud: bool,
    pub sharp_text: bool,
    pub texts: Vec<crate::fonts::Text>,
    pub labels: Vec<crate::debug::Label>,
}

struct SceneDrawing {
    backgrounds: Vec<crate::renderer::EffectMesh>,
    terrain: crate::renderer::Mesh,
    props: crate::renderer::Mesh,
    actors: crate::renderer::Mesh,
    effects: crate::renderer::EffectMesh,
    environment: Option<environment::Environment>,
    lighting: lighting::Lighting,
    sky: crate::visuals::sky::Clock,
}

impl Drawing {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn light_count(&self, settings: &crate::settings::Lighting) -> usize {
        self.scene.lighting.count(settings)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn navigation_surfaces(
        &mut self,
        session: &super::Session,
        renderer: &mut Renderer,
    ) -> &crate::visuals::surfaces::Surfaces {
        let environment = self.scene.environment(
            &session.assets.level,
            &self.models.scenery,
            &session.assets.resources.math,
            renderer,
        );
        &environment.surfaces
    }

    pub fn clear_blood(&mut self) {
        self.wounds.clear();
        self.scene.clear_blood();
    }

    pub fn clear_weather(&mut self, tracks_only: bool) {
        self.scene.clear_weather(tracks_only);
    }
    pub fn weather_audio(
        &mut self,
        settings: &crate::settings::Weather,
    ) -> (tenchu_audio::Ambience, Option<tenchu_audio::ThunderCue>) {
        self.scene.weather_audio(settings)
    }

    pub fn blood_audio(
        &mut self,
        session: &super::Session,
    ) -> Result<Vec<tenchu_assets::audio::voice::Sourced<tenchu_core::tenchu2::audio::Sound>>> {
        self.wounds.sounds(session)
    }
    pub fn preserve_environment(&mut self, previous: &mut Self) {
        self.scene.preserve_environment(&mut previous.scene);
        self.motion = std::mem::take(&mut previous.motion);
        self.motion.invalidate();
        if let Some(environment) = &mut self.scene.environment {
            environment.blood.invalidate_models();
        }
    }

    pub fn new(
        session: &super::Session,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let models = Models::new(&session.assets, &session.world, graphics, renderer)?;
        #[cfg(not(target_arch = "wasm32"))]
        let models = {
            let mut models = models;
            if let Some(custom) = &session.models.custom {
                models.custom(&custom.id, &custom.pack, session, graphics, renderer)?;
            }
            models
        };
        renderer.visual_time = session.world.tick as f32 / 30.0;
        let geometry = models.geometry(
            &session.world,
            &session.assets,
            &session.frame,
            graphics,
            Presentation {
                show_hud: true,
                ..Default::default()
            },
        )?;
        let mut drawing = Self {
            models,
            motion: Default::default(),
            wounds: Default::default(),
            scene: SceneDrawing::new(geometry, renderer, &session.assets.level),
            view: session.world.camera.view,
            show_hud: true,
            sharp_text: false,
            texts: Vec::new(),
            camera: None,
            alpha: 1.0,
            labels: Vec::new(),
        };
        drawing.prepare_wounds(session, &graphics.blood, renderer);
        drawing.tick(session, graphics)?;
        Ok(drawing)
    }

    fn prepare_wounds(
        &mut self,
        session: &super::Session,
        settings: &crate::settings::Blood,
        renderer: &mut Renderer,
    ) {
        if settings.wounds() || settings.gush(false) || settings.weapons && settings.amount != 0 {
            self.scene.environment(
                &session.assets.level,
                &self.models.scenery,
                &session.assets.resources.math,
                renderer,
            );
        }
    }

    pub fn tick(&mut self, session: &super::Session, graphics: &Graphics) -> Result<()> {
        self.tick_motion(session, &graphics.motion)?;
        if let Some(environment) = &mut self.scene.environment {
            self.wounds
                .tick(session, &self.models, environment, &graphics.blood)?;
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        session: &super::Session,
        renderer: &mut Renderer,
        debug: Option<&crate::debug::Debug>,
    ) -> Result<()> {
        let blood = renderer.graphics().blood;
        self.prepare_wounds(session, &blood, renderer);
        self.tick(session, renderer.graphics())?;
        let motion = self.motion.sample(if renderer.graphics().interpolation {
            self.alpha
        } else {
            1.0
        });
        let pose = session
            .interpolation
            .as_ref()
            .filter(|_| renderer.graphics().interpolation)
            .and_then(|history| history.sample(u64::from(session.world.tick), self.alpha));
        let view = pose
            .as_ref()
            .map_or(session.world.camera.view, |frame| frame.view);
        let mut geometry = self.models.geometry(
            &session.world,
            &session.assets,
            &session.frame,
            renderer.graphics(),
            Presentation {
                show_hud: self.show_hud,
                sharp_text: self.sharp_text,
                alpha: if renderer.graphics().interpolation {
                    self.alpha
                } else {
                    1.0
                },
                camera: self.camera,
                pose: pose.as_ref(),
                motion: Some(&motion),
            },
        )?;
        self.texts = if self.show_hud && self.sharp_text {
            hud::text(
                &session.world.controls.hud.frame,
                session.frame.transition_fade,
            )
        } else {
            Vec::new()
        };
        self.scene.lighting.update_emitters(
            &session.frame.effects.emitters,
            &renderer.graphics().lighting,
        );
        self.scene.lighting.focus = self.camera.is_none().then(|| {
            glam::Vec3::from(
                session.world.actors[session.world.player]
                    .actor
                    .position
                    .to_array()
                    .map(|v| v as f32),
            )
        });
        let camera = camera_matrix(view, self.camera, renderer.size());
        if let Some(environment) = &self.scene.environment
            && (environment.blood.attached_parts().next().is_some()
                || environment.blood.has_drops()
                || environment.blood.weapon_parts().next().is_some())
        {
            geometry.effects.append(wounds::geometry(
                &environment.blood,
                session,
                &self.models,
                pose.as_ref(),
                &motion,
                crate::visuals::blood::View {
                    camera: Some(self.camera.map_or(view, |camera| camera.source_view())),
                    projection: Some(camera),
                    alpha: if renderer.graphics().interpolation {
                        self.alpha
                    } else {
                        1.0
                    },
                    ..Default::default()
                },
            )?);
        }
        self.labels = self
            .scene
            .inspect_lights(debug, camera, renderer, &mut geometry);
        self.scene.prepare_environment(
            &mut geometry,
            environment::Frame {
                level: &session.assets.level,
                meshes: &self.models.scenery,
                math: &session.assets.resources.math,
                tick: u64::from(session.world.tick),
                camera,
                alpha: self.alpha,
                contact: Some(&session.contact),
                world: Some((&session.world, &session.assets)),
            },
            renderer,
        );
        if let Some(debug) = debug {
            if debug.collision_only && !debug.photo_mode {
                renderer.set_decals(std::iter::empty());
                geometry.terrain.clear();
                geometry.actors.clear();
                geometry.effects = render_effects::Geometry {
                    vertices: Vec::new(),
                    batches: Vec::new(),
                };
                for background in &mut geometry.backgrounds {
                    background.vertices.clear();
                    background.batches.clear();
                }
            }
            let size = renderer.size();
            let overlay =
                super::inspection::overlay(session, debug, camera, size[0] as f32 / size[1] as f32);
            geometry.effects.append(overlay.geometry);
            self.labels.extend(overlay.labels);
        }
        self.scene.update(geometry, renderer);
        renderer.visual_time = session.world.tick as f32 / 30.0;
        self.view = view;
        Ok(())
    }

    pub fn draw(&self, target: &wgpu::TextureView, renderer: &mut Renderer) -> Result<()> {
        if let Some(camera) = self.camera {
            let [w, h] = renderer.size();
            self.scene
                .draw_camera(camera.matrix(w as f32 / h as f32), target, renderer)
        } else {
            self.scene.draw(self.view, target, renderer)
        }
    }
}

fn camera_matrix(
    view: Matrix,
    camera: Option<crate::debug::camera::Camera>,
    size: [u32; 2],
) -> glam::Mat4 {
    camera
        .unwrap_or_else(|| crate::debug::camera::Camera::from_view(view, 300.0))
        .matrix(size[0] as f32 / size[1] as f32)
}

impl SceneDrawing {
    fn inspect_lights(
        &mut self,
        debug: Option<&crate::debug::Debug>,
        camera: glam::Mat4,
        renderer: &Renderer,
        geometry: &mut Geometry,
    ) -> Vec<crate::debug::Label> {
        self.lighting.lamps = std::mem::take(&mut geometry.lamps);
        let origin = camera.inverse() * glam::Vec4::new(0.0, 0.0, 1.0, 0.0);
        let eye = origin.truncate() / origin.w;
        self.lighting.debug_light = debug.and_then(|debug| debug.light_at(eye));
        let settings = &renderer.graphics().lighting;
        if !settings.markers || debug.is_some_and(|debug| debug.photo_mode || debug.collision_only)
        {
            return Vec::new();
        }
        let [width, height] = renderer.size();
        let overlay = crate::debug::light_markers(
            camera,
            width as f32 / height as f32,
            &self.lighting.named_sources(settings, eye),
            if settings.shadows {
                usize::from(settings.shadow_count)
            } else {
                0
            },
        );
        geometry.effects.append(overlay.geometry);
        overlay.labels
    }

    fn environment(
        &mut self,
        level: &Level,
        meshes: &Meshes,
        math: &tenchu_core::math::Math,
        renderer: &mut Renderer,
    ) -> &mut environment::Environment {
        let environment = self
            .environment
            .get_or_insert_with(|| environment::Environment::new(level, meshes, math, renderer));
        if environment.stale {
            environment.rebuild(level, meshes, math, renderer);
        }
        environment
    }

    fn preserve_environment(&mut self, previous: &mut Self) {
        self.environment = previous.environment.take();
        self.sky = previous.sky.clone();
        if let Some(environment) = &mut self.environment {
            environment.stale = true;
        }
    }

    fn prepare_environment(
        &mut self,
        geometry: &mut Geometry,
        frame: environment::Frame<'_>,
        renderer: &mut Renderer,
    ) {
        let graphics = renderer.graphics().clone();
        let sky_light = graphics.sky.enabled
            || (graphics.lighting.enabled && graphics.lighting.directional.intensity > 0);
        if (graphics.weather.mode != crate::settings::WeatherMode::Original
            || graphics.water.enabled
            || graphics.effects.sparks
            || graphics.effects.splashes
            || graphics.blood.retained_ground()
            || graphics.blood.custom_death(false)
            || graphics.blood.custom_death(true)
            || ((self.lighting.active(&graphics.lighting) || sky_light)
                && graphics.lighting.shadows))
            && self.environment.is_none()
        {
            self.environment(frame.level, frame.meshes, frame.math, renderer);
        }
        renderer.weather = Default::default();
        self.sky.tick(frame.tick, &graphics.sky);
        renderer.daylight = self.sky.sample(&graphics.sky, 1.0);
        self.lighting.transients.clear();
        if let Some(environment) = &mut self.environment {
            if environment.stale {
                environment.rebuild(frame.level, frame.meshes, frame.math, renderer);
            }
            let mut effects = environment.update(&frame, &graphics, renderer);
            if graphics.effects.sparks && graphics.effects.spark_density > 0 {
                self.lighting.transients = crate::lighting::sparks::clusters(
                    environment.contact.spark_lights(frame.alpha),
                );
                self.lighting
                    .transients
                    .extend(environment.contact.lights().into_iter().take(2));
            }
            effects.append(std::mem::replace(
                &mut geometry.effects,
                render_effects::Geometry {
                    vertices: Vec::new(),
                    batches: Vec::new(),
                },
            ));
            geometry.effects = effects;
        } else {
            renderer.set_decals(std::iter::empty());
        }
    }

    fn clear_blood(&mut self) {
        if let Some(environment) = &mut self.environment {
            environment.blood.clear();
        }
    }

    fn clear_weather(&mut self, tracks_only: bool) {
        if let Some(environment) = &mut self.environment {
            if tracks_only {
                environment.clear_footprints();
            } else {
                environment.clear();
            }
        }
    }

    fn weather_audio(
        &mut self,
        settings: &crate::settings::Weather,
    ) -> (tenchu_audio::Ambience, Option<tenchu_audio::ThunderCue>) {
        self.environment
            .as_mut()
            .map_or_else(|| (Default::default(), None), |e| e.audio(settings))
    }

    fn new(geometry: Geometry, renderer: &mut Renderer, level: &Level) -> Self {
        Self {
            backgrounds: geometry
                .backgrounds
                .into_iter()
                .map(|g| renderer.upload_effects(g))
                .collect(),
            terrain: renderer.upload(&geometry.terrain),
            props: renderer.upload(&geometry.props),
            actors: renderer.upload(&geometry.actors),
            effects: renderer.upload_effects(geometry.effects),
            environment: None,
            lighting: {
                let mut lighting = lighting::Lighting::new(level);
                lighting.lamps = geometry.lamps;
                lighting
            },
            sky: Default::default(),
        }
    }

    fn update(&mut self, mut geometry: Geometry, renderer: &mut Renderer) {
        for (mesh, geometry) in self.backgrounds.iter_mut().zip(geometry.backgrounds) {
            renderer.update_effects(mesh, geometry);
        }
        renderer.update_mesh(&mut self.terrain, &mut geometry.terrain);
        renderer.update_mesh(&mut self.props, &mut geometry.props);
        renderer.update_mesh(&mut self.actors, &mut geometry.actors);
        renderer.update_effects(&mut self.effects, geometry.effects);
    }

    pub fn draw(
        &self,
        view: Matrix,
        target: &wgpu::TextureView,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let [w, h] = renderer.size();
        let camera = glam::Mat4::perspective_rh(
            2.0 * (120.0f32 / 300.0).atan(),
            w as f32 / h as f32,
            50.0,
            250_000.0,
        ) * crate::renderer::view_matrix(view);
        self.draw_camera(camera, target, renderer)
    }

    fn draw_camera(
        &self,
        camera: glam::Mat4,
        target: &wgpu::TextureView,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let eye = camera.inverse().project_point3(glam::Vec3::ZERO);
        let lights = self.lighting.sources(&renderer.graphics().lighting, eye);
        renderer.draw_lit(
            target,
            camera,
            crate::renderer::SceneMeshes {
                terrain: &[&self.terrain, &self.props],
                actors: &[&self.actors],
                backgrounds: &self.backgrounds,
            },
            Some(&self.effects),
            &lights,
            &self
                .environment
                .as_ref()
                .map_or_else(Vec::new, |e| vec![&e.caster, &self.props, &self.actors]),
        )
    }
}

pub fn capture(
    session: &super::Session,
    settings: &crate::settings::Settings,
    path: &std::path::Path,
) -> Result<()> {
    let graphics = &settings.graphics;
    capture_frame(&session.assets.sprites.vram, graphics, path, |renderer| {
        let mut drawing = Drawing::new(session, graphics, renderer)?;
        drawing.show_hud = settings.show_hud;
        drawing.sharp_text = settings.sharp_text;
        drawing.update(session, renderer, None)?;
        Ok((drawing.scene, session.world.camera.view, drawing.texts))
    })?;
    eprintln!(
        "Native JP tick {}: position {:?}; {} actors; radar {:?}; pending {:?}",
        session.world.tick,
        session.world.actors[session.world.player].actor.position,
        session.world.actors.len(),
        session.world.ai.global.radar,
        session.pending
    );
    Ok(())
}

fn capture_frame(
    vram: &Vram,
    graphics: &Graphics,
    path: &std::path::Path,
    build: impl FnOnce(&mut Renderer) -> Result<(SceneDrawing, Matrix, Vec<crate::fonts::Text>)>,
) -> Result<()> {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .ok_or("No graphics adapter")?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = pollster::block_on(Renderer::new(&adapter, format, [1280, 720], vram))?;
    renderer.configure(graphics);
    let (drawing, view, texts) = build(&mut renderer)?;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native JP capture"),
        size: wgpu::Extent3d {
            width: 1280,
            height: 720,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    drawing.draw(
        view,
        &texture.create_view(&Default::default()),
        &mut renderer,
    )?;
    if !texts.is_empty() {
        crate::fonts::capture(&renderer, &texture, &texts);
    }
    crate::capture::read_texture(&renderer, &texture, path)
}

pub struct Models {
    player: Option<custom::Player>,
    scenery: Meshes,
    lamps: crate::visuals::lights::Emitters,
    bodies: BTreeMap<String, Meshes>,
    weapons: Meshes,
    imported_weapons: BTreeMap<String, Meshes>,
    items: Meshes,
    weapon_names: BTreeMap<usize, String>,
}

pub struct Geometry {
    lamps: Vec<crate::lighting::Source>,
    pub backgrounds: Vec<render_effects::Geometry>,
    pub terrain: Vec<Vertex>,
    pub props: Vec<Vertex>,
    pub actors: Vec<Vertex>,
    pub effects: render_effects::Geometry,
}

pub struct Presentation<'a> {
    pub alpha: f32,
    pub show_hud: bool,
    pub sharp_text: bool,
    pub camera: Option<crate::debug::camera::Camera>,
    pub pose: Option<&'a PoseFrame>,
    pub motion: Option<&'a crate::visuals::motion::Frame>,
}

impl Default for Presentation<'_> {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            show_hud: false,
            sharp_text: false,
            camera: None,
            pose: None,
            motion: None,
        }
    }
}

fn static_models<'a>(
    level: &'a Level,
    math: &'a tenchu_core::math::Math,
    phases: Option<&'a [i16]>,
) -> impl Iterator<Item = (&'a Part, Matrix)> {
    level
        .scene
        .statics
        .iter()
        .enumerate()
        .filter_map(move |(index, object)| {
            let tenchu_assets::tenchu2::level::objects::StaticKind::Model(model) = object.kind
            else {
                return None;
            };
            let mut rotation = object.rotation;
            if let Some(phases) = phases {
                rotation = [phases[index], object.rotation[1], 0];
            }
            Some((
                &level.objects[model],
                Matrix {
                    translation: object.position.into(),
                    rotation: math.rotate_yxz(rotation),
                },
            ))
        })
}

fn sources<'a>(name: &str, parts: &'a [Part], role: Role) -> Vec<catalog::ModelSource<'a>> {
    parts
        .iter()
        .enumerate()
        .map(|(index, part)| catalog::ModelSource {
            id: format!("tenchu2/{name}/{index}"),
            role,
            label: name.into(),
            geometry: &part.geometry,
        })
        .collect()
}

fn build(
    vram: &Vram,
    sources: Vec<catalog::ModelSource<'_>>,
    graphics: &Graphics,
) -> Result<Meshes> {
    let mut meshes = prepare(vram, sources, graphics)?;
    meshes.detach_textures();
    Ok(meshes)
}

fn prepare(
    vram: &Vram,
    sources: Vec<catalog::ModelSource<'_>>,
    graphics: &Graphics,
) -> Result<Meshes> {
    let pack = lamps::pack(vram, &sources, graphics)?;
    prepare_pack(vram, sources, graphics, &pack)
}

fn prepare_pack(
    vram: &Vram,
    sources: Vec<catalog::ModelSource<'_>>,
    graphics: &Graphics,
    pack: &Pack,
) -> Result<Meshes> {
    let mut meshes = Meshes::build(vram, pack, graphics, sources, true)?;
    if graphics.asset_pack {
        meshes.replace_textures(pack);
    }
    meshes.inherit_palette_blending(vram);
    Ok(meshes)
}

impl Models {
    pub fn new(
        assets: &Assets,
        world: &World,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let mut scenery = sources(
            &format!("map{}/terrain", world.map),
            &assets.level.map,
            Role::Terrain,
        );
        scenery.extend(background::sources(&assets.level));
        scenery.extend(sources(
            &format!("map{}/props", world.map),
            &assets.level.objects,
            Role::Prop,
        ));
        for (id, part) in &assets.level.stumps {
            scenery.extend(sources(
                &format!("map{}/stump{id}", world.map),
                std::slice::from_ref(part),
                Role::Prop,
            ));
        }
        let (mut scenery, lamps) = lamps::prepare(&assets.level.vram, scenery, graphics)?;
        renderer.set_emitters(&lamps);
        renderer.bind_meshes(&mut scenery);
        let mut bodies = BTreeMap::new();
        for (name, model) in &assets.models {
            let mut model_sources = sources(&format!("actors/{name}"), &model.parts, Role::Actor);
            if let Some(ids) = assets.model_sources.get(name) {
                for (source, id) in model_sources.iter_mut().zip(ids) {
                    source.id.clone_from(id);
                }
            }
            let mut meshes = build(&model.vram, model_sources, graphics)?;
            meshes.skin(&model.parts);
            renderer.bind_meshes(&mut meshes);
            bodies.insert(name.clone(), meshes);
        }
        let weapon_sources = assets
            .weapons
            .iter()
            .filter(|(name, _)| !assets.weapon_imports.contains_key(*name))
            .flat_map(|(name, parts)| sources(&format!("weapons/{name}"), parts, Role::Other))
            .collect();
        let mut weapons = build(&assets.weapon_vram, weapon_sources, graphics)?;
        renderer.bind_meshes(&mut weapons);
        let mut imported_weapons = BTreeMap::new();
        for (name, source) in &assets.weapon_imports {
            let parts = &assets.weapons[name];
            let mut model_sources = sources(&format!("weapons/{name}"), parts, Role::Other);
            for model in &mut model_sources {
                model.id.clone_from(&source.source_id);
            }
            let mut meshes = build(&source.vram, model_sources, graphics)?;
            renderer.bind_meshes(&mut meshes);
            imported_weapons.insert(name.clone(), meshes);
        }
        let item_sources = assets
            .items
            .models()
            .flat_map(|(id, parts)| {
                sources(
                    &format!("items/{id}"),
                    parts,
                    if id == 0 { Role::Grapple } else { Role::Other },
                )
            })
            .collect();
        let mut items = build(&assets.items.vram, item_sources, graphics)?;
        renderer.bind_meshes(&mut items);
        let mut weapon_names = BTreeMap::new();
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            weapon_names.extend(
                interpolation::weapons(&agent.actor, instance, assets)?
                    .map(|(root, name)| (root, name.to_owned())),
            );
        }
        Ok(Self {
            player: None,
            scenery,
            lamps,
            bodies,
            weapons,
            imported_weapons,
            items,
            weapon_names,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn custom(
        &mut self,
        id: &str,
        pack: &Pack,
        session: &super::Session,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let actor = &session.world.actors[session.world.player].actor;
        let definition = session
            .assets
            .resources
            .registry
            .actor(actor.definition_kind())
            .ok_or("Missing player definition")?;
        let body = &session.assets.models[&definition.model];
        self.player = Some(custom::Player::new(
            id,
            pack,
            &body.parts,
            &body.vram,
            graphics,
            renderer,
        )?);
        Ok(())
    }

    pub fn geometry(
        &self,
        world: &World,
        assets: &Assets,
        frame: &Frame,
        graphics: &Graphics,
        presentation: Presentation<'_>,
    ) -> Result<Geometry> {
        let Presentation {
            alpha,
            show_hud,
            sharp_text,
            camera,
            pose,
            motion,
        } = presentation;
        let view = camera.map_or_else(
            || pose.map_or(world.camera.view, |frame| frame.view),
            |c| c.source_view(),
        );
        let mut output = Builder::new(view, &assets.level, graphics);
        if let Some(camera) = camera {
            output.distance = 120.0 / (camera.fov.to_radians() * 0.5).tan();
        }
        let math = &assets.resources.math;
        for part in &assets.level.map {
            output.model(
                &self.scenery,
                part,
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
                ModelStyle::Terrain,
            );
        }
        let distant_props = !graphics.fog || camera.is_some();
        if distant_props {
            for (part, matrix) in
                static_models(&assets.level, math, Some(&world.environment.static_phases))
            {
                output.model(&self.scenery, part, matrix, ModelStyle::Terrain);
            }
        }
        for model in &frame.scenery.models {
            if distant_props
                && matches!(
                    model.source,
                    tenchu_core::tenchu2::environment::scenery::Source::Static(_)
                )
            {
                continue;
            }
            output.scenery(&self.scenery, &assets.level, math, model)?;
        }
        if graphics.distant_doors
            && (!graphics.fog || graphics.fog_distance > 100 || camera.is_some())
        {
            for model in scenery::distant_doors(world, &assets.level, &frame.scenery) {
                output.scenery(&self.scenery, &assets.level, math, &model)?;
            }
        }
        // Presentation must not refresh the simulation's cached coordinate matrices.
        let mut scene = world.scene.clone();
        let player_hand = self.visit_actors(
            world,
            assets,
            &mut scene,
            pose,
            |key, meshes, part, matrix, style| {
                let deformation = motion.and_then(|frame| {
                    meshes.deformation(frame, key, part.geometry.as_ptr() as usize, matrix)
                });
                output.deformed_model(meshes, part, matrix, style, deformation.as_ref());
            },
        )?;
        let mut rope = None;
        if let Some(hook) = world.items.grapple() {
            if let Some(parts) = assets.items.model(0) {
                output.parts(&self.items, parts, hook.transform, math)?;
            }
            let hand = *world.instances[hook.owner]
                .rig
                .joints
                .get(11)
                .ok_or("Missing native grapple hand")?;
            let mut previous = PoseFrame::transform(
                pose,
                interpolation::part(
                    world.instances[hook.owner].rig.root,
                    hand,
                    world.actors[hook.owner].actor.definition_kind(),
                ),
                scene.world(hand)?,
            )
            .translation;
            if hook.owner == world.player
                && let Some(hand) = player_hand
            {
                previous = hand.translation;
            }
            if graphics.models.grapple {
                let points: Vec<_> = std::iter::once(previous)
                    .chain(hook.rope.iter().copied())
                    .map(|p| glam::Vec3::from(p.to_array().map(|v| v as f32)))
                    .collect();
                rope = Some(crate::visuals::contact::rope::path(&points));
            } else {
                for &point in &hook.rope {
                    output.line(previous, point);
                    previous = point;
                }
            }
        }
        if let Some(item) = world.items.held
            && let Some(parts) = assets.items.model(item)
        {
            let hand = *world.instances[world.player]
                .rig
                .joints
                .get(11)
                .ok_or("Missing native held-item hand")?;
            let transform = PoseFrame::transform(
                pose,
                interpolation::part(
                    world.instances[world.player].rig.root,
                    hand,
                    world.actors[world.player].actor.definition_kind(),
                ),
                scene.world(hand)?,
            );
            output.parts(&self.items, parts, player_hand.unwrap_or(transform), math)?;
        }
        for entry in &world.items.entries {
            use tenchu_core::tenchu2::items::{State, arrow::Phase};
            let (model, transform, rotation) = match &entry.state {
                State::Projectile(p) => (p.model(), p.transform, p.rotation),
                State::Grenade(g) if g.visible => (6, g.transform, [0; 3]),
                State::Mine(m) => (7, m.transform, [0; 3]),
                State::Smoke(s) => (10, s.transform, [0; 3]),
                State::SleepBomb(bomb) => (18, bomb.transform, [0; 3]),
                State::Rice(r) => (11, r.transform, [0; 3]),
                State::Spear(spear) => (24, spear.transform, [0; 3]),
                State::Resurrection(r) => (20, r.transform, [0; 3]),
                State::Marker(marker) => (
                    32 + marker.color,
                    Matrix {
                        translation: marker.position,
                        ..Default::default()
                    },
                    [0; 3],
                ),
                State::Caltrop(c) => (
                    5,
                    Matrix {
                        translation: c.flight.position,
                        ..Default::default()
                    },
                    [0; 3],
                ),
                State::Arrow(arrow) => {
                    if arrow.flame {
                        let image =
                            assets.sprites.effects[12 + (world.tick.wrapping_sub(1) & 3) as usize];
                        let projected = output.project(arrow.flight.position);
                        if projected[2] > 0.0 {
                            let scale = 2400.0 / projected[2];
                            output.screen_sprite(
                                projected,
                                image.size.map(f32::from),
                                [scale; 2],
                                0,
                                [1.0; 3],
                                Some(image),
                                2,
                            );
                        }
                    }
                    if matches!(arrow.phase, Phase::Exploding(_)) && !arrow.flame {
                        continue;
                    }
                    let transform = if let Phase::Embedded {
                        parent: Some(parent),
                        ..
                    } = arrow.phase
                    {
                        scene.world(parent)?.compose(&arrow.transform)
                    } else {
                        arrow.transform
                    };
                    (arrow.model(), transform, arrow.rotation)
                }
                _ => continue,
            };
            if let Some(parts) = assets.items.model(model) {
                let transform = transform.compose(&Matrix {
                    rotation: math.rotate_yxz(rotation),
                    ..Default::default()
                });
                output.parts(&self.items, parts, transform, math)?;
            }
        }
        for sprite in &frame.scenery.sprites {
            let size = sprite.image.size.map(f32::from);
            output.billboard(
                sprite.position,
                size,
                sprite.scale.map(|v| v as f32 / 256.0),
                sprite.rotation,
                [f32::from(sprite.color) / 128.0; 3],
                Some(sprite.image),
                sprite.blend + 1,
            );
        }
        for sprite in &frame.effects.sprites {
            if modern_contact(sprite.contact, &graphics.effects) {
                continue;
            }
            if output.blood_particle(sprite.position, sprite.blood, graphics.blood.hit, alpha) {
                continue;
            }
            let image = assets.sprites.effects[usize::from(sprite.image)];
            let blend = blend(sprite.blend, sprite.image);
            let projected = output.project(sprite.position);
            if projected[2] <= 0.0 {
                continue;
            }
            let adjustment = if camera.is_some() {
                let source_depth = (world.camera.view.apply_large(sprite.position).z
                    + world.camera.view.translation.z)
                    .max(0) as f32;
                source_depth / projected[2] * output.distance / 300.0
            } else {
                1.0
            };
            let scale = sprite.scale.map(|v| f32::from(v) / 4096.0 * adjustment);
            output.screen_sprite(
                projected,
                image.size.map(f32::from),
                scale,
                sprite.rotation,
                sprite.color.map(|v| f32::from(v) / 128.0),
                Some(image),
                blend,
            );
            if sprite.blood.is_some()
                && graphics.blood.hit == crate::settings::BloodStyle::OriginalHd
            {
                output.blood_original_hd();
            }
        }
        for point in &frame.effects.points {
            if modern_contact(point.contact, &graphics.effects) {
                continue;
            }
            if output.blood_particle(point.position, point.blood, graphics.blood.hit, alpha) {
                continue;
            }
            let projected = output.project(point.position);
            if projected[2] <= 0.0 {
                continue;
            }
            output.screen_sprite(
                projected,
                point.size.map(f32::from),
                [1.0; 2],
                0,
                point.color.map(|v| f32::from(v) / 255.0),
                None,
                if point.blend & 4 != 0 {
                    (point.blend & 3) + 1
                } else {
                    0
                },
            );
            if point.blood.is_some()
                && graphics.blood.hit == crate::settings::BloodStyle::OriginalHd
            {
                output.blood_original_hd();
            }
        }
        for ribbon in &frame.effects.ribbons {
            let depth = (ribbon.depth * 16) as f32;
            let points = ribbon.corners.map(|p| {
                if camera.is_some() {
                    let inverse = crate::renderer::view_matrix(world.camera.view).inverse();
                    let world = inverse.transform_point3(glam::Vec3::new(
                        f32::from(p[0]) * depth / 300.0,
                        -f32::from(p[1]) * depth / 300.0,
                        -depth,
                    ));
                    let projected = output.project(world.to_array().map(|v| v as i32).into());
                    [
                        projected[0],
                        projected[1],
                        render_effects::depth(projected[2].max(1.0)),
                    ]
                } else {
                    [
                        f32::from(p[0]),
                        f32::from(p[1]),
                        render_effects::depth(depth),
                    ]
                }
            });
            output.surfaces.push(Surface {
                depth,
                blend: 2,
                vertices: render_effects::quad(
                    points,
                    ribbon.colors.map(|c| c.map(|v| f32::from(v) / 255.0)),
                    None,
                    2,
                ),
            });
        }
        for quad in &frame.effects.quads {
            let death_blood =
                quad.blood == Some(tenchu_core::tenchu2::effects::surface::Blood::Death);
            if death_blood
                && (!graphics.blood.death.uses_original()
                    || graphics.blood.custom_death(false) && quad.blend != Blend::Opaque)
            {
                continue;
            }
            let ground_blood = quad.blood
                == Some(tenchu_core::tenchu2::effects::surface::Blood::Ground)
                && quad.parent.is_none();
            if ground_blood
                && (!graphics.blood.ground.uses_original()
                    || graphics.blood.retained_ground() && quad.blend != Blend::Opaque)
            {
                continue;
            }
            if graphics.effects.splashes
                && quad.parent.is_none()
                && quad.image
                    == SurfaceImage::Effect(tenchu_core::tenchu2::effects::water::RIPPLE_IMAGE)
            {
                continue;
            }
            let (image, blend) = match quad.image {
                SurfaceImage::Effect(id) => (
                    assets.sprites.effects[usize::from(id)],
                    blend(quad.blend, id),
                ),
                SurfaceImage::Item(id) => {
                    (assets.sprites.items[usize::from(id)], blend(quad.blend, 0))
                }
            };
            let mut transform = Matrix {
                translation: quad.position,
                ..Default::default()
            };
            if let Some(parent) = quad.parent {
                transform = scene.world(parent)?.compose(&transform);
            }
            let points = quad.corners.map(|v| {
                let v = if death_blood && graphics.blood.death_size != 100 {
                    v.map(|n| (i32::from(n) * i32::from(graphics.blood.death_size) / 100) as i16)
                } else {
                    v
                };
                transform
                    .transform(v.to_array())
                    .map(|v| v as f32)
                    .to_array()
            });
            let mut vertices = render_effects::quad(
                points,
                [quad.color.map(|v| f32::from(v) / 128.0); 4],
                Some(image),
                blend,
            );
            for vertex in &mut vertices {
                vertex.flags &= !render_mesh::SCREEN;
                vertex.flags |= render_mesh::UNLIT;
                if ground_blood || death_blood {
                    vertex.flags |= render_mesh::DOUBLE_SIDED;
                }
            }
            let blend = if ground_blood
                && graphics.blood.ground == crate::settings::BloodStyle::OriginalHd
                || death_blood && graphics.blood.death == crate::settings::BloodStyle::OriginalHd
            {
                render_effects::blood::original_hd(&mut vertices, blend)
            } else {
                blend
            };
            output.surface(vertices, blend);
        }
        if show_hud {
            hud::append(
                &mut output,
                &assets.sprites,
                &world.controls.hud.frame,
                sharp_text,
            );
        }
        if let Some(fade) = frame.fade {
            output.screen_sprite(
                [0.0, 0.0, 64.0],
                [320.0, 240.0],
                [1.0; 2],
                0,
                [f32::from(fade) / 255.0; 3],
                None,
                2,
            );
            for vertex in &mut output.surfaces.last_mut().unwrap().vertices {
                vertex.flags |= render_mesh::SCREEN_WIDE;
            }
        }
        if let Some(fade) = frame.transition_fade {
            output.screen_sprite(
                [0.0, 0.0, 1.0],
                [320.0, 240.0],
                [1.0; 2],
                0,
                [f32::from(fade) / 255.0; 3],
                None,
                3,
            );
            for vertex in &mut output.surfaces.last_mut().unwrap().vertices {
                vertex.flags |= render_mesh::UI | render_mesh::SCREEN_WIDE;
            }
        }
        let mut effects = render_effects::surface_geometry(output.surfaces);
        if let Some(mut rope) = rope {
            rope.append(effects);
            effects = rope;
        }
        Ok(Geometry {
            lamps: lamps::sources(&self.lamps, &assets.level, math, Some(&world.environment)),
            backgrounds: background::geometry(
                &assets.level.background,
                &assets.level.shading,
                &self.scenery,
                view,
                math,
                world.tick,
            ),
            terrain: output.terrain,
            props: output.props,
            actors: output.actors,
            effects,
        })
    }
}

fn modern_contact(
    kind: Option<tenchu_core::tenchu2::effects::ContactKind>,
    settings: &crate::settings::Effects,
) -> bool {
    use tenchu_core::tenchu2::effects::ContactKind;
    match kind {
        Some(ContactKind::Metal) => settings.sparks,
        Some(ContactKind::Water) => settings.splashes,
        None => false,
    }
}

fn blend(blend: Blend, image: u8) -> u8 {
    match blend {
        // 8003a44c configures GsSPRITE attributes separately from the TIM page.
        Blend::Texture => match image {
            1 | 5 | 11 | 18 | 27 | 31 => 0,
            24 | 25 | 28 => 3,
            30 => 1,
            _ => 2,
        },
        Blend::Opaque => 0,
        Blend::Add => 2,
        Blend::SubtractMask => 3,
    }
}

struct Builder {
    view: Matrix,
    distance: f32,
    shading: Shading,
    fog_start: Option<i32>,
    surfaces: Vec<Surface>,
    terrain: Vec<Vertex>,
    props: Vec<Vertex>,
    actors: Vec<Vertex>,
}

#[derive(Clone, Copy)]
enum ModelStyle {
    Terrain,
    Prop,
    Actor,
    Concealed,
    Imported { concealed: bool },
}

impl Builder {
    fn scenery(
        &mut self,
        meshes: &Meshes,
        level: &Level,
        math: &tenchu_core::math::Math,
        model: &tenchu_core::tenchu2::environment::scenery::Model,
    ) -> Result<()> {
        let part = if model.stump {
            level.stumps.get(&model.model)
        } else {
            level.objects.get(model.model)
        }
        .ok_or("Missing native scenery model")?;
        self.model(
            meshes,
            part,
            Matrix {
                translation: model.position,
                rotation: math.rotate_yxz(model.rotation),
            },
            match model.source {
                tenchu_core::tenchu2::environment::scenery::Source::Static(_) => {
                    ModelStyle::Terrain
                }
                tenchu_core::tenchu2::environment::scenery::Source::Object(_) => ModelStyle::Prop,
            },
        );
        Ok(())
    }

    fn new(view: Matrix, level: &Level, graphics: &Graphics) -> Self {
        Self {
            view,
            distance: 300.0,
            shading: level.shading.clone(),
            fog_start: graphics.fog.then(|| {
                ((level.draw_distance * i32::from(graphics.fog_distance) / 100) >> 4) - 256
            }),
            surfaces: Vec::new(),
            terrain: Vec::new(),
            props: Vec::new(),
            actors: Vec::new(),
        }
    }

    fn depth(&self, position: [f32; 3]) -> f32 {
        self.view.translation.z as f32
            + (0..3)
                .map(|i| f32::from(self.view.rotation[2][i]) * position[i] / 4096.0)
                .sum::<f32>()
    }

    fn project(&self, point: Position) -> [f32; 3] {
        let point = point.map(|v| v as f32).to_array();
        let v: [f32; 3] = std::array::from_fn(|axis| {
            self.view.translation[axis] as f32
                + (0..3)
                    .map(|i| f32::from(self.view.rotation[axis][i]) * point[i] / 4096.0)
                    .sum::<f32>()
        });
        [
            v[0] * self.distance / v[2],
            v[1] * self.distance / v[2],
            v[2],
        ]
    }

    fn surface(&mut self, vertices: Vec<Vertex>, blend: u8) {
        let depth = vertices
            .iter()
            .map(|v| {
                self.view.translation.z as f32
                    + (0..3)
                        .map(|i| f32::from(self.view.rotation[2][i]) * v.position[i] / 4096.0)
                        .sum::<f32>()
            })
            .sum::<f32>()
            / vertices.len() as f32;
        self.surfaces.push(Surface {
            depth,
            blend,
            vertices,
        });
    }

    fn model(&mut self, meshes: &Meshes, part: &Part, matrix: Matrix, style: ModelStyle) {
        self.deformed_model(meshes, part, matrix, style, None);
    }

    fn deformed_model(
        &mut self,
        meshes: &Meshes,
        part: &Part,
        matrix: Matrix,
        style: ModelStyle,
        deformation: Option<&crate::visuals::motion::Deformation<'_>>,
    ) {
        let terrain = matches!(style, ModelStyle::Terrain | ModelStyle::Prop);
        meshes.visit(&part.geometry, matrix, false, 0, deformation, |triangle| {
            let mut triangle = triangle.map(|v| *v);
            let depths = triangle.map(|v| self.depth(v.position).max(0.0) as i32 >> 4);
            if self
                .fog_start
                .is_some_and(|start| depths.iter().all(|&z| z >= start + 256))
            {
                return;
            }
            let depth = if terrain {
                *depths.iter().max().unwrap()
            } else {
                depths.iter().sum::<i32>() / 3
            };
            for (vertex, z) in triangle.iter_mut().zip(depths) {
                let fade = self
                    .fog_start
                    .filter(|&start| depth > start)
                    .map_or(0, |start| (z - start).max(0));
                if matches!(style, ModelStyle::Imported { .. }) {
                    vertex.color = vertex.color.map(|c| (c - fade as f32 / 128.0).max(0.0));
                } else {
                    palette_color(vertex, &self.shading, fade);
                }
            }
            if matches!(
                style,
                ModelStyle::Concealed | ModelStyle::Imported { concealed: true }
            ) {
                for vertex in &mut triangle {
                    vertex.flags |= render_mesh::TRANSLUCENT;
                    vertex.page |= 0x60;
                }
            }
            if terrain {
                for v in &mut triangle {
                    v.flags |= render_mesh::SCENERY;
                }
            }
            if triangle[0].flags & render_mesh::TRANSLUCENT == 0 {
                if matches!(style, ModelStyle::Prop) {
                    self.props.extend(triangle);
                } else if terrain {
                    self.terrain.extend(triangle);
                } else {
                    self.actors.extend(triangle);
                }
            } else {
                let blend = ((triangle[0].page >> 5) & 3) as u8 + 1;
                self.surface(triangle.to_vec(), blend);
            }
        });
    }

    fn parts(
        &mut self,
        meshes: &Meshes,
        parts: &[Part],
        matrix: Matrix,
        math: &tenchu_core::math::Math,
    ) -> Result<()> {
        let mut scene = tenchu_core::scene::Scene::default();
        let root = scene.add(None, matrix);
        let joints = tenchu_core::motion::Pose::new(parts)?.bind(&mut scene, root, math);
        for (part, joint) in parts.iter().zip(joints) {
            self.model(meshes, part, scene.world(joint)?, ModelStyle::Actor);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn billboard(
        &mut self,
        position: Position,
        size: [f32; 2],
        scale: [f32; 2],
        rotation: i32,
        color: [f32; 3],
        image: Option<Image>,
        blend: u8,
    ) {
        let projected = self.project(position);
        self.screen_sprite(
            projected,
            size,
            scale.map(|v| v * self.distance / projected[2]),
            rotation,
            color,
            image,
            blend,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn screen_sprite(
        &mut self,
        center: [f32; 3],
        size: [f32; 2],
        scale: [f32; 2],
        rotation: i32,
        color: [f32; 3],
        image: Option<Image>,
        blend: u8,
    ) {
        if center[2] <= 0.0 {
            return;
        }
        let (sin, cos) = (rotation as f32 / 4096.0).to_radians().sin_cos();
        let points = std::array::from_fn(|i| {
            let x = ((i & 1) as f32 - 0.5) * size[0] * scale[0];
            let y = (((i >> 1) & 1) as f32 - 0.5) * size[1] * scale[1];
            [
                center[0] + x * cos - y * sin,
                center[1] + x * sin + y * cos,
                render_effects::depth(center[2]),
            ]
        });
        self.surfaces.push(Surface {
            depth: center[2],
            blend,
            vertices: render_effects::quad(points, [color; 4], image, blend),
        });
    }

    fn line(&mut self, a: Position, b: Position) {
        let a = self.project(a);
        let b = self.project(b);
        if a[2] <= 0.0 || b[2] <= 0.0 {
            return;
        }
        let delta = glam::Vec2::new(b[0] - a[0], b[1] - a[1]);
        let offset = glam::Vec2::new(-delta.y, delta.x).normalize_or_zero() * 0.6;
        let points = [
            [
                a[0] - offset.x,
                a[1] - offset.y,
                render_effects::depth(a[2]),
            ],
            [
                a[0] + offset.x,
                a[1] + offset.y,
                render_effects::depth(a[2]),
            ],
            [
                b[0] - offset.x,
                b[1] - offset.y,
                render_effects::depth(b[2]),
            ],
            [
                b[0] + offset.x,
                b[1] + offset.y,
                render_effects::depth(b[2]),
            ],
        ];
        self.surfaces.push(Surface {
            depth: (a[2] + b[2]) * 0.5,
            blend: 0,
            vertices: render_effects::quad(points, [[0.3, 0.26, 0.2]; 4], None, 0),
        });
    }
}

fn palette_color(vertex: &mut Vertex, shading: &Shading, fade: i32) {
    let scale = if vertex.flags & render_mesh::TEXTURED != 0 {
        128.0
    } else {
        255.0
    };
    let index = (vertex.color[0] * scale).round() as i32;
    vertex.color = shading
        .color((index - fade).clamp(0, 255) as u8)
        .map(|v| f32::from(v) / scale);
}
