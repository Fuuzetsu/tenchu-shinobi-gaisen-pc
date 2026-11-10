mod state;
use super::*;
use crate::visuals::{lights::Emitters, models::Models, pack::Pack};
pub(super) use state::State;

#[derive(Clone, Copy, PartialEq)]
struct ModelOptions {
    artwork: bool,
    smooth: bool,
    detail: u16,
    water: bool,
    water_detail: u16,
    characters: u16,
    heroes: bool,
    foliage: u16,
    variation: bool,
    grapple: bool,
    leaves: bool,
    secondary: bool,
    wind: bool,
    baked_snow: bool,
}

fn model_options(graphics: &settings::Graphics) -> ModelOptions {
    ModelOptions {
        artwork: graphics.asset_pack,
        smooth: graphics.smooth_models,
        detail: graphics.model_detail,
        water: graphics.water.enabled,
        water_detail: graphics.water.detail,
        characters: graphics.models.characters,
        heroes: graphics.models.heroes,
        foliage: graphics.models.foliage,
        variation: graphics.models.variation,
        grapple: graphics.models.grapple,
        leaves: graphics.effects.leaves || graphics.effects.scatter,
        secondary: graphics.motion.actors(),
        wind: graphics.motion.wind != 0,
        baked_snow: graphics.weather.baked_snow,
    }
}

pub(super) struct Frame<'a> {
    pub campaign: &'a Campaign,
    pub paused: bool,
    pub error: Option<&'a str>,
    pub settings: &'a Settings,
    pub menu: &'a Menu,
    pub fps: f32,
    pub offline: bool,
    pub debug: &'a crate::debug::Debug,
    pub alpha: f32,
}

pub(super) struct View {
    pub(super) adapter: wgpu::AdapterInfo,
    pub dirty: bool,
    pub(super) stage: Mesh,
    pub(super) actor: Mesh,
    shadow_stage: Option<Mesh>,
    shadow_models: Option<Mesh>,
    shared_actor_shadows: bool,
    actor_vertices: render_mesh::Indexed,
    vertices: Vec<render_mesh::Vertex>,
    lighting_updated: Instant,
    pub(super) lights: Vec<crate::lighting::Light>,
    pub(super) effects: EffectMesh,
    effect_vertices: Vec<render_mesh::Vertex>,
    pub(super) labels: Vec<crate::debug::Label>,
    pub(super) text: Vec<crate::fonts::Text>,
    pub(super) overlay: super::overlay::Overlay,
    pub(super) renderer: Renderer,
    pub(super) surface: Option<wgpu::Surface<'static>>,
    #[cfg(target_os = "android")]
    pub(super) instance: wgpu::Instance,
    pub(super) window: Arc<Window>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) size: [u32; 2],
    pub(super) vram_revision: u64,
    pack: Pack,
    pack_error: Option<String>,
    pub(super) models: Models,
    pub(super) bodies: Arc<Pack>,
    pub(super) model_library: Option<crate::visuals::appearance::library::Library>,
    pub(super) model_picker: crate::visuals::appearance::picker::Picker,
    pub(super) player_models: Vec<(String, String)>,
    player_selection: String,
    pub(super) randomizer_player: Option<String>,
    model_options: ModelOptions,
    emitters: Emitters,
    surfaces: Option<crate::visuals::surfaces::Surfaces>,
    pub(super) blood: crate::visuals::blood::Blood,
    sky: crate::visuals::sky::Clock,
    weather: crate::visuals::weather::Weather,
    pub(super) footprints: crate::visuals::footprints::Footprints,
    spray: crate::visuals::water::Spray,
    contact: crate::visuals::contact::Contact,
    motion: crate::visuals::motion::Motion,
    motion_options: settings::Motion,
    deaths: crate::visuals::death::Deaths,
    pub(super) interpolation: crate::visuals::interpolation::History,
    interpolation_enabled: bool,
    frame: Option<crate::visuals::interpolation::Frame>,
}

impl View {
    pub(super) fn death_frame(&self) -> Option<&crate::visuals::interpolation::Frame> {
        self.deaths.frame.as_ref()
    }

    pub(super) fn thunder(&self) -> Option<tenchu_audio::ThunderCue> {
        self.weather.thunder()
    }

    pub(super) fn ambience(&self, settings: &settings::Weather) -> tenchu_audio::Ambience {
        self.surfaces
            .as_ref()
            .map_or_else(Default::default, |surfaces| {
                self.weather.ambience(surfaces, settings)
            })
    }

    pub(super) fn navigation_surfaces(
        &mut self,
        game: &Game,
    ) -> &crate::visuals::surfaces::Surfaces {
        self.surfaces.get_or_insert_with(|| {
            crate::visuals::surfaces::Surfaces::new(&render_mesh::stage(
                &game.mission,
                &game.math,
                Some(&self.models),
            ))
        })
    }
    pub(super) fn diagnostic_snapshot(&self) -> serde_json::Value {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let window_backend = match self.window.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "X11",
            Ok(RawWindowHandle::Wayland(_)) => "Wayland",
            Ok(RawWindowHandle::Win32(_)) => "Win32",
            Ok(RawWindowHandle::AppKit(_)) => "AppKit",
            Ok(RawWindowHandle::AndroidNdk(_)) => "Android",
            Ok(
                RawWindowHandle::Web(_)
                | RawWindowHandle::WebCanvas(_)
                | RawWindowHandle::WebOffscreenCanvas(_),
            ) => "Web",
            _ => "Unknown",
        };
        serde_json::json!({
            "adapter": {"name": self.adapter.name, "vendor_id": self.adapter.vendor,
                "device_id": self.adapter.device, "device_type": format!("{:?}", self.adapter.device_type),
                "backend": format!("{:?}", self.adapter.backend),
                "driver": self.adapter.driver, "driver_info": self.adapter.driver_info},
            "window_pixels": self.size, "render_pixels": self.renderer.size(),
            "window_backend": window_backend,
            "scale_factor": self.window.scale_factor(),
            "present_mode": format!("{:?}", self.config.present_mode),
            "surface_format": format!("{:?}", self.config.format),
            "supported_msaa": self.renderer.sample_counts,
            "gpu_timestamps_supported": self.renderer.timings.is_some(),
            "gpu_profiling_active": self.renderer.profiling,
            "lights": self.lights.len(), "model_interpolation": self.interpolation_enabled,
            "player_model": self.player_selection, "randomizer_player": self.randomizer_player,
            "artwork": {"error": self.pack_error, "textures": self.pack.textures.len(),
                "models": self.pack.models.len(), "model_patches": self.pack.model_patches.len(),
                "water_rules": self.pack.water.len(), "vegetation_rules": self.pack.vegetation.len(),
                "body_textures": self.bodies.textures.len(), "custom_bodies": self.bodies.player_models.len()},
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn recording_settings(&self) -> serde_json::Value {
        serde_json::json!({
            "graphics": self.renderer.graphics(),
            "model_interpolation": self.interpolation_enabled,
        })
    }

    pub(super) fn prewarm_weather(
        &mut self,
        game: &Game,
        settings: &Settings,
        debug: &crate::debug::Debug,
        seconds: u16,
    ) {
        if seconds == 0 {
            return;
        }
        self.cosmetic_tick(game, settings, debug);
        if let Some(surfaces) = &self.surfaces {
            if settings.graphics.effects.leaves {
                self.contact
                    .prewarm(surfaces, &settings.graphics, debug.eye(game), seconds);
            }
            self.weather.prewarm(
                game.tick,
                debug.eye(game),
                surfaces,
                &settings.graphics.weather,
                seconds,
            );
        }
    }

    pub(super) fn clear_weather(&mut self) {
        self.weather = Default::default();
        self.footprints.clear();
        self.contact.clear_weather();
        self.dirty = true;
    }

    pub(super) fn cosmetic_tick(
        &mut self,
        game: &Game,
        settings: &Settings,
        debug: &crate::debug::Debug,
    ) {
        if let Err(error) = self.deaths.tick(game, settings.alternate_death_chance) {
            eprintln!("Player death animation: {error}");
            self.deaths.frame = None;
        }
        let death = self.deaths.frame.as_ref().filter(|_| !settings.retro);
        if let Some(frame) = death {
            self.interpolation.present(game.tick, frame);
        }
        self.sky.tick(game.tick, &settings.graphics.sky);
        self.motion
            .tick(game, &self.models, &settings.graphics.motion, death);
        let blood = settings.graphics.blood.active();
        let weather = settings.graphics.weather.mode != settings::WeatherMode::Original;
        let footprints = weather || settings.graphics.weather.baked_snow;
        let contact = settings.graphics.effects.active();
        if blood || weather || footprints || contact {
            let surfaces = self.surfaces.get_or_insert_with(|| {
                crate::visuals::surfaces::Surfaces::new(&render_mesh::stage(
                    &game.mission,
                    &game.math,
                    Some(&self.models),
                ))
            });
            if blood {
                self.blood.tick(
                    game,
                    &self.models,
                    surfaces,
                    &settings.graphics.blood,
                    death,
                );
            }
            if weather {
                let center = debug.eye(game);
                self.weather
                    .tick(game.tick, center, surfaces, &settings.graphics.weather);
            }
            if footprints {
                self.footprints.tick(
                    game,
                    &self.models,
                    surfaces,
                    &settings.graphics.weather,
                    self.weather.sample(&settings.graphics.weather, 1.0).snow,
                );
            }
            if contact {
                self.contact.tick(
                    game,
                    surfaces,
                    &settings.graphics,
                    self.weather.sample(&settings.graphics.weather, 1.0),
                    debug.eye(game),
                );
            }
        }
        if !blood {
            self.blood = Default::default();
        }
        if !weather {
            self.weather = Default::default();
        }
        if !footprints {
            self.footprints.clear();
        }
    }

    pub(super) fn nearest_lamp(
        &self,
        game: &Game,
        eye: glam::Vec3,
    ) -> Option<crate::lighting::Source> {
        self.emitters.sources(game).into_iter().min_by(|a, b| {
            eye.distance_squared(a.light.position)
                .total_cmp(&eye.distance_squared(b.light.position))
        })
    }

    fn window_attributes() -> winit::window::WindowAttributes {
        let attributes = Window::default_attributes()
            .with_title("Tenchu — gameplay preview")
            .with_inner_size(LogicalSize::new(1280, 720));
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("game")
                .unwrap()
                .dyn_into()
                .unwrap();
            attributes
                .with_canvas(Some(canvas))
                .with_prevent_default(true)
        }
        #[cfg(not(target_arch = "wasm32"))]
        attributes
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn new(
        events: &ActiveEventLoop,
        assets: &GameAssets<DiscSource>,
        game: &Game,
        settings: &Settings,
    ) -> Result<Self> {
        let window = Arc::new(
            events
                .create_window(Self::window_attributes())
                .map_err(|e| e.to_string())?,
        );
        pollster::block_on(Self::from_window(window, assets, game, settings))
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(deprecated)]
    pub(super) fn browser_window(events: &EventLoop<()>) -> Result<Arc<Window>> {
        events
            .create_window(Self::window_attributes())
            .map(Arc::new)
            .map_err(|e| e.to_string())
    }

    pub(super) async fn from_window(
        window: Arc<Window>,
        assets: &GameAssets<DiscSource>,
        game: &Game,
        settings: &Settings,
    ) -> Result<Self> {
        let configured = settings;
        let rendered = settings.rendering();
        let settings = &*rendered;
        let descriptor = wgpu::InstanceDescriptor::default();
        #[cfg(target_os = "android")]
        let descriptor = wgpu::InstanceDescriptor {
            backends: crate::android::graphics_backend(),
            ..descriptor
        };
        #[cfg(target_os = "android")]
        crate::android::startup_log(&format!("Graphics backends: {:?}", descriptor.backends));
        #[cfg(target_arch = "wasm32")]
        let descriptor = wgpu::InstanceDescriptor {
            backends: match crate::web::graphics_preference().as_str() {
                "webgl2" => wgpu::Backends::GL,
                "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
                _ => wgpu::Backends::all(),
            },
            ..descriptor
        };
        let instance = wgpu::util::new_instance_with_webgpu_detection(&descriptor).await;
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or("No graphics adapter available. Enable hardware acceleration in your browser or graphics driver.")?;
        #[cfg(target_os = "android")]
        crate::android::startup_log(&format!(
            "Adapter: {:?}\nLimits: {:?}\nFeatures: {:?}",
            adapter.get_info(),
            adapter.limits(),
            adapter.features()
        ));
        #[cfg(target_arch = "wasm32")]
        crate::web::graphics_backend(
            if adapter.get_info().backend == wgpu::Backend::Gl {
                "WebGL2"
            } else {
                "WebGPU"
            },
            &adapter.get_info().name,
        );
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Querying presentation surface");
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("No surface configuration")?;
        if let Some(format) = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(|f| !f.is_srgb())
        {
            config.format = format;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        #[cfg(target_os = "android")]
        crate::android::graphics_stage(&format!("Presentation configuration: {config:?}"));
        let mut renderer = Renderer::new(
            &adapter,
            config.format,
            [config.width, config.height],
            &game.mission.vram,
        )
        .await?;
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Renderer created; loading artwork and models");
        let mut pack_error = None;
        let mut pack = Pack::from_env(&game.mission).unwrap_or_else(|error| {
            eprintln!("Asset pack: {error}; using original assets");
            pack_error = Some(error);
            Pack::default()
        });
        let mut models =
            Models::build(&game.mission, &pack, &settings.graphics).unwrap_or_else(|error| {
                eprintln!("Asset pack: {error}; using original assets");
                pack_error = Some(error);
                pack = Pack::default();
                Models::build(&game.mission, &pack, &settings.graphics).unwrap()
            });
        let bodies = Pack::bodies(&settings.model_directory).unwrap_or_else(|error| {
            eprintln!("Player models: {error}; using disc bodies");
            Arc::default()
        });
        let player_models = crate::visuals::appearance::choices(game, &bodies, None);
        models = models
            .with_player(
                game,
                &settings.player_model,
                &pack,
                &bodies,
                &settings.graphics,
                None,
            )
            .unwrap_or_else(|error| {
                eprintln!("Player model: {error}; using Original");
                Models::build(&game.mission, &pack, &settings.graphics).unwrap()
            });
        renderer.set_asset_pack(&pack);
        let emitters = Emitters::build(game, &pack, settings.graphics.asset_pack);
        renderer.set_emitters(&emitters);
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Uploading model textures");
        renderer.bind_models(&mut models);
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Configuring presentation surface");
        surface.configure(&renderer.device, &config);
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Uploading terrain, actors and effects");
        let stage = renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let spray = if models.has_water() {
            crate::visuals::water::Spray::new(&render_mesh::shadow_stage(
                &game.mission,
                &game.math,
                Some(&models),
            ))
        } else {
            Default::default()
        };
        let mut actor = renderer.upload(&[]);
        let mut actor_vertices = render_mesh::Indexed {
            poses: renderer.pose_frame(),
            ..Default::default()
        };
        render_mesh::update_indexed_characters(
            &mut actor_vertices,
            game,
            Some(&models),
            false,
            None,
        );
        renderer.update_indexed_mesh(&mut actor, &mut actor_vertices);
        let effects = renderer.upload_effects(render_effects::build_options(
            game,
            true,
            None,
            settings.graphics.blob_shadows,
            Some(&models),
        ));
        println!(
            "Renderer: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Creating settings overlay");
        let overlay = super::overlay::Overlay::new(&window, &renderer, config.format);
        let mut contact = crate::visuals::contact::Contact::default();
        contact.canopies = models.canopies(game);
        let mut view = Self {
            adapter: adapter.get_info(),
            dirty: true,
            overlay,
            window,
            surface: Some(surface),
            #[cfg(target_os = "android")]
            instance,
            config,
            renderer,
            stage,
            actor,
            shadow_stage: None,
            shadow_models: None,
            effect_vertices: Vec::new(),
            shared_actor_shadows: false,
            actor_vertices,
            vertices: Vec::new(),
            lighting_updated: Instant::now(),
            lights: Vec::new(),
            effects,
            labels: Vec::new(),
            text: Vec::new(),
            size: [size.width, size.height],
            vram_revision: game.mission.vram.revision,
            pack,
            pack_error,
            models,
            bodies,
            model_library: None,
            model_picker: Default::default(),
            player_models,
            player_selection: settings.player_model.clone(),
            randomizer_player: None,
            model_options: model_options(&settings.graphics),
            emitters,
            surfaces: None,
            blood: Default::default(),
            sky: Default::default(),
            weather: Default::default(),
            footprints: Default::default(),
            contact,
            spray,
            motion: Default::default(),
            motion_options: settings.graphics.motion.clone(),
            deaths: crate::visuals::death::Deaths::load(assets)?,
            interpolation: Default::default(),
            interpolation_enabled: false,
            frame: None,
        };
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Applying saved graphics settings");
        view.settings(configured, game)?;
        #[cfg(target_os = "android")]
        crate::android::graphics_stage("Graphics startup complete; waiting for first frame");
        Ok(view)
    }

    fn selected_player<'a>(&'a self, settings: &'a Settings) -> &'a str {
        if let Some(selection) = &self.randomizer_player {
            return selection;
        }
        if self
            .model_library
            .as_ref()
            .is_some_and(|l| l.get(&settings.player_model).is_some())
            || self
                .player_models
                .iter()
                .any(|(id, _)| id == &settings.player_model)
        {
            &settings.player_model
        } else {
            ""
        }
    }

    pub(super) fn load_model_library(
        &mut self,
        assets: &GameAssets<DiscSource>,
        game: &Game,
        settings: &Settings,
    ) -> Result<()> {
        let library = crate::visuals::appearance::library::Library::load(assets)?;
        self.player_models =
            crate::visuals::appearance::choices(game, &self.bodies, Some(&library));
        self.model_library = Some(library);
        self.model_picker.clear();
        if !settings.player_model.is_empty() {
            self.set_models(self.build_models(game, settings)?, game);
        }
        Ok(())
    }

    pub(super) fn refresh_actor_assets(&mut self, game: &Game, settings: &Settings) -> Result<()> {
        let mut models = self.build_models(game, settings)?;
        self.renderer.bind_models(&mut models);
        self.models = models;
        self.reset_interpolation(game);
        self.dirty = true;
        Ok(())
    }

    fn build_models(&self, game: &Game, settings: &Settings) -> Result<Models> {
        let rendered = settings.rendering();
        let settings = &*rendered;
        Models::build(&game.mission, &self.pack, &settings.graphics)?.with_player(
            game,
            self.selected_player(settings),
            &self.pack,
            &self.bodies,
            &settings.graphics,
            self.model_library.as_ref(),
        )
    }

    pub(super) fn load_bodies(
        &mut self,
        directory: &str,
        game: &Game,
        settings: &Settings,
    ) -> Result<()> {
        let rendered = settings.rendering();
        let settings = &*rendered;
        let bodies = Pack::bodies(directory)?;
        let choices =
            crate::visuals::appearance::choices(game, &bodies, self.model_library.as_ref());
        if !directory.is_empty() && choices.iter().all(|(id, _)| !id.starts_with("pack:")) {
            return Err("This folder contains no compatible player_models; export assets to create a template".into());
        }
        let selection = if let Some(selection) = &self.randomizer_player {
            selection.as_str()
        } else if choices.iter().any(|(id, _)| id == &settings.player_model)
            || self
                .model_library
                .as_ref()
                .is_some_and(|l| l.get(&settings.player_model).is_some())
        {
            settings.player_model.as_str()
        } else {
            ""
        };
        let models = Models::build(&game.mission, &self.pack, &settings.graphics)?.with_player(
            game,
            selection,
            &self.pack,
            &bodies,
            &settings.graphics,
            self.model_library.as_ref(),
        )?;
        self.model_picker.clear();
        self.bodies = bodies;
        self.player_models = choices;
        self.player_selection = selection.into();
        self.set_models(models, game);
        self.reset_interpolation(game);
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn scale_pointer(&self, position: &mut winit::dpi::PhysicalPosition<f64>) {
        use winit::platform::web::WindowExtWebSys;
        if let Some(canvas) = self.window.canvas() {
            // CSS and browser zoom can scale the canvas independently of its drawing buffer.
            let rect = canvas.get_bounding_client_rect();
            let logical = position.to_logical::<f64>(self.window.scale_factor());
            position.x = logical.x * f64::from(self.size[0]) / rect.width().max(1.0);
            position.y = logical.y * f64::from(self.size[1]) / rect.height().max(1.0);
        }
    }

    pub(super) fn preparing(&self) -> bool {
        #[cfg(target_os = "android")]
        return self.renderer.preparing();
        #[cfg(not(target_os = "android"))]
        false
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.dirty = true;
        self.size = [width, height];
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.config.width = width;
        self.config.height = height;
        #[cfg(target_os = "android")]
        {
            let preparing = self.preparing();
            self.renderer.prepare_resize([width, height])?;
            if self.preparing() {
                if !preparing {
                    crate::android::preparing_graphics(true);
                }
                return Ok(());
            }
        }
        if let Some(surface) = &self.surface {
            surface.configure(&self.renderer.device, &self.config);
        }
        #[cfg(not(target_os = "android"))]
        self.renderer.resize([width, height]);
        Ok(())
    }

    pub(super) fn reload(&mut self, game: &Game, settings: &Settings) -> Result<()> {
        self.deaths.reset();
        self.model_picker.clear();
        let rendered = settings.rendering();
        let settings = &*rendered;
        self.sky = Default::default();
        self.weather = Default::default();
        self.footprints = Default::default();
        self.contact = Default::default();
        self.reset_interpolation(game);
        self.renderer.reset_vram(&game.mission.vram);
        self.vram_revision = game.mission.vram.revision;
        self.pack_error = None;
        self.pack = Pack::from_env(&game.mission).unwrap_or_else(|error| {
            eprintln!("Asset pack: {error}; using original assets");
            self.pack_error = Some(error);
            Pack::default()
        });
        self.renderer.set_asset_pack(&self.pack);
        self.player_models =
            crate::visuals::appearance::choices(game, &self.bodies, self.model_library.as_ref());
        let result = self.build_models(game, settings);
        let error = result.as_ref().err().cloned();
        self.set_models(
            result.unwrap_or_else(|_| {
                Models::build(&game.mission, &self.pack, &settings.graphics).unwrap_or_default()
            }),
            game,
        );
        self.player_selection = self.selected_player(settings).to_owned();
        self.model_options = model_options(&settings.graphics);
        self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
        self.renderer.set_emitters(&self.emitters);
        error.map_or(Ok(()), Err)
    }

    fn set_models(&mut self, mut models: Models, game: &Game) {
        self.dirty = true;
        self.renderer.bind_models(&mut models);
        self.shadow_stage = None;
        self.surfaces = None;
        self.blood = Default::default();
        self.weather.invalidate_surfaces();
        self.motion = Default::default();
        self.spray = if models.has_water() {
            crate::visuals::water::Spray::new(&render_mesh::shadow_stage(
                &game.mission,
                &game.math,
                Some(&models),
            ))
        } else {
            Default::default()
        };
        self.stage = self.renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        self.contact.canopies = models.canopies(game);
        self.models = models;
    }

    pub(super) fn artwork_status(&self) -> String {
        let status = if let Some(error) = &self.pack_error {
            format!("Asset pack failed to load: {error}")
        } else {
            format!(
                "Gaisen artwork loaded: {} textures, {} models. Imported terrain and characters load artwork separately.",
                self.pack.textures.len(),
                self.pack.models.len()
            )
        };
        #[cfg(not(target_arch = "wasm32"))]
        let status = {
            use crate::visuals::pack::{configured_root, configured_tenchu2_root};
            let mut status = status;
            for (name, path, archive) in [
                ("Shinobi Gaisen", configured_root(), "tenchu-hd.tar.zst"),
                ("Tenchu 2", configured_tenchu2_root(), "tenchu2-hd.tar.zst"),
            ] {
                status.push_str(&match path {
                    Some(path) => format!("\n{name} pack: {}", path.display()),
                    None => format!(
                        "\n{name} pack not installed. Put {archive} beside the game and restart."
                    ),
                });
            }
            status.push_str("\nReplacement artwork toggles both packs. Missing textures use the corresponding disc's originals.");
            status
        };
        status
    }

    pub(super) fn load_pack(
        &mut self,
        pack: Pack,
        game: &Game,
        settings: &Settings,
    ) -> Result<usize> {
        let rendered = settings.rendering();
        let settings = &*rendered;
        let models = Models::build(&game.mission, &pack, &settings.graphics)?.with_player(
            game,
            self.selected_player(settings),
            &pack,
            &self.bodies,
            &settings.graphics,
            self.model_library.as_ref(),
        )?;
        let count = models.replacements;
        self.renderer.set_asset_pack(&pack);
        self.set_models(models, game);
        self.pack = pack;
        self.pack_error = None;
        self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
        self.renderer.set_emitters(&self.emitters);
        Ok(count)
    }

    pub(super) fn reset_interpolation(&mut self, game: &Game) {
        self.frame = None;
        if self.interpolation_enabled {
            self.interpolation.reset(game);
            if let Some(frame) = &self.deaths.frame {
                self.interpolation.present(game.tick, frame);
            }
        } else {
            self.interpolation = Default::default();
        }
    }

    pub(super) fn settings(&mut self, settings: &Settings, game: &Game) -> Result<()> {
        let configured = settings;
        let rendered = settings.rendering();
        let settings = &*rendered;
        self.dirty = true;
        let motion = &settings.graphics.motion;
        let previous = &self.motion_options;
        if (motion.hair, motion.cloth, motion.equipment, motion.actors())
            != (
                previous.hair,
                previous.cloth,
                previous.equipment,
                previous.actors(),
            )
        {
            self.motion = Default::default();
            self.models.motion.clear();
        }
        self.motion_options = motion.clone();
        if self.interpolation_enabled != settings.graphics.interpolation {
            self.interpolation_enabled = settings.graphics.interpolation;
            self.reset_interpolation(game);
        }
        let options = model_options(&settings.graphics);
        if options != self.model_options || self.player_selection != self.selected_player(settings)
        {
            self.set_models(self.build_models(game, settings)?, game);
            self.model_options = options;
            self.player_selection = self.selected_player(settings).to_owned();
            self.reset_interpolation(game);
            self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
            self.renderer.set_emitters(&self.emitters);
        }
        if !configured.graphics.effects.active() {
            self.contact.clear();
        }
        self.models.weapon_materials =
            settings.graphics.blood.weapons || settings.graphics.weapon_reflections > 0;
        self.blood.configure(&configured.graphics.blood);
        if configured.graphics.weather.mode == settings::WeatherMode::Original {
            self.weather = Default::default();
        }
        if (configured.graphics.weather.mode == settings::WeatherMode::Original
            && !configured.graphics.weather.baked_snow)
            || !configured.graphics.weather.footprints
        {
            self.footprints.clear();
        }
        #[cfg(not(target_os = "android"))]
        self.renderer.configure(&settings.graphics);
        #[cfg(target_os = "android")]
        {
            let preparing = self.preparing();
            self.renderer
                .prepare(&settings.graphics, self.size.map(|n| n.max(1)))?;
            if !preparing && self.preparing() {
                crate::android::preparing_graphics(true);
            }
        }
        self.presentation_settings(settings);
        Ok(())
    }

    pub(super) fn presentation_settings(&mut self, settings: &Settings) {
        if self.window.fullscreen().is_some() != settings.fullscreen {
            self.window
                .set_fullscreen(settings.fullscreen.then_some(Fullscreen::Borderless(None)));
        }
        let mode = if settings.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        if self.config.present_mode != mode {
            self.config.present_mode = mode;
            if !self.preparing()
                && let Some(surface) = &self.surface
            {
                surface.configure(&self.renderer.device, &self.config);
            }
        }
    }

    pub(super) fn update(&mut self, frame: Frame<'_>) {
        self.dirty = true;
        let Frame {
            campaign,
            paused,
            error,
            settings,
            menu,
            fps,
            offline,
            debug,
            alpha,
        } = frame;
        let retro = settings.retro;
        let rendered = settings.rendering();
        let settings = &*rendered;
        let game = &campaign.game;
        if self.vram_revision != game.mission.vram.revision {
            self.renderer.update_vram(&game.mission.vram);
            self.vram_revision = game.mission.vram.revision;
        }
        self.frame = if self.interpolation_enabled {
            self.interpolation.sample(game.tick, alpha)
        } else {
            None
        }
        .or_else(|| (!retro).then(|| self.deaths.frame.clone()).flatten());
        self.renderer.daylight = self.sky.sample(&settings.graphics.sky, alpha);
        self.models.motion = if settings.graphics.motion.actors() {
            self.motion.sample(alpha)
        } else {
            Default::default()
        };
        self.renderer.weather = self.weather.sample(&settings.graphics.weather, alpha);
        self.renderer.visual_time =
            ((game.tick as f64 + f64::from(alpha) - 1.0).max(0.0) / 30.0) as f32;
        let frame = self.frame.as_ref();
        let camera = debug.camera.or(debug.follow_camera);
        let inspection = camera.is_some() || game.roster.keep_all_active;
        render_mesh::update_indexed_characters(
            &mut self.actor_vertices,
            game,
            Some(&self.models),
            inspection,
            frame,
        );
        self.renderer
            .update_indexed_mesh(&mut self.actor, &mut self.actor_vertices);
        let mut sources = crate::lighting::sources(
            game,
            &settings.graphics.lighting,
            debug.camera.map(|camera| camera.position).or_else(|| {
                debug
                    .followed_actor(game)
                    .map(|actor| glam::Vec3::from(actor.position.to_array().map(|v| v as f32)))
            }),
            debug.light_position(game),
            if settings.graphics.lighting.enabled && settings.graphics.lighting.lamps {
                self.emitters.sources_presented(game, frame)
            } else {
                Vec::new()
            },
        );
        if settings.graphics.lighting.enabled && settings.graphics.lighting.count > 0 {
            let mut sparks = if settings.graphics.effects.lightning {
                crate::lighting::lightning(game)
            } else {
                Vec::new()
            };
            if settings.graphics.effects.sparks {
                sparks.extend(crate::lighting::sparks::sources(
                    game,
                    alpha,
                    settings.graphics.effects.spark_density,
                    self.contact.spark_lights(alpha),
                ));
                sparks.extend(self.contact.lights().into_iter().take(2));
            }
            if settings.graphics.effects.gunfire {
                sparks.extend(crate::lighting::gunfire(game, alpha));
            }
            sparks.append(&mut sources);
            sparks.truncate(crate::lighting::MAX_SCENE_LIGHTS);
            sources = sparks;
        }
        self.lights = sources.iter().map(|s| s.light).collect();
        let now = Instant::now();
        self.renderer.shadow_elapsed = Some(if offline {
            1.0 / 60.0
        } else {
            now.duration_since(self.lighting_updated).as_secs_f32()
        });
        self.lighting_updated = now;
        let shadowing = (settings.graphics.weather.mode != settings::WeatherMode::Original)
            || (settings.graphics.lighting.enabled || settings.graphics.sky.enabled)
                && settings.graphics.lighting.shadows
                && ((settings.graphics.lighting.shadow_count > 0 && !self.lights.is_empty())
                    || settings.graphics.lighting.directional.intensity > 0
                    || settings.graphics.sky.enabled);
        if shadowing && self.shadow_stage.is_none() {
            self.shadow_stage = Some(self.renderer.upload_shadow(&render_mesh::shadow_stage(
                &game.mission,
                &game.math,
                Some(&self.models),
            )));
        }
        // Inspection presents every actor using the same poses as the shadow pass.
        self.shared_actor_shadows = shadowing && inspection;
        if shadowing {
            render_mesh::update_shadow_models(
                &mut self.vertices,
                game,
                Some(&self.models),
                frame,
                self.shared_actor_shadows,
            );
            let mesh = self
                .shadow_models
                .get_or_insert_with(|| self.renderer.upload(&[]));
            self.renderer.update_mesh(mesh, &mut self.vertices);
        } else {
            self.shadow_models = None;
        }
        self.text.clear();
        let mut geometry = if campaign.scene_visible()
            && (menu.open || !paused || debug.active())
            && error.is_none()
        {
            if debug.collision_only {
                render_effects::Geometry {
                    vertices: Vec::new(),
                    batches: Vec::new(),
                }
            } else {
                if settings.sharp_text && !debug.photo_mode {
                    self.text = crate::fonts::game_text(game, settings.show_hud);
                }
                render_effects::build_presented(
                    game,
                    settings.show_hud && !debug.photo_mode,
                    camera.map(|c| c.source_view()),
                    settings.graphics.blob_shadows,
                    Some(&self.models),
                    frame,
                    render_effects::Style {
                        alpha: if self.interpolation_enabled {
                            alpha
                        } else {
                            1.0
                        },
                        ..render_effects::Style::from(settings)
                    },
                )
            }
        } else {
            ui::build(
                campaign,
                paused,
                settings.circle_confirm,
                error,
                settings.sharp_text.then_some(&mut self.text),
            )
        };
        if campaign.scene_visible() && error.is_none() && !debug.collision_only {
            if settings.graphics.weather.mode != settings::WeatherMode::Original {
                geometry.remove_flag(render_mesh::ORIGINAL_WEATHER);
            }
            if settings.graphics.effects.splashes {
                geometry.remove_flag(render_mesh::ORIGINAL_SPLASH);
            }
        }
        if campaign.scene_visible()
            && error.is_none()
            && !debug.collision_only
            && (menu.open || !paused || debug.active())
            && settings.graphics.blood.active()
        {
            self.renderer
                .set_projected_decals(self.blood.projected_pages());
            let mut cosmetics = self.blood.animated_geometry(
                game,
                &self.models,
                frame,
                crate::visuals::blood::View {
                    camera: camera.map(|camera| camera.source_view()),
                    projection: Some(self.camera_matrix(game, debug)),
                    alpha: if self.interpolation_enabled {
                        alpha
                    } else {
                        1.0
                    },
                    wind: f32::from(settings.graphics.motion.wind) / 100.0,
                },
                std::mem::take(&mut self.effect_vertices),
            );
            cosmetics.append(geometry);
            geometry = cosmetics;
        } else {
            self.renderer.set_decals(std::iter::empty());
        }
        if campaign.scene_visible()
            && error.is_none()
            && !debug.collision_only
            && (settings.graphics.weather.mode != settings::WeatherMode::Original
                || settings.graphics.weather.baked_snow)
        {
            let mut footprints = self.footprints.geometry(
                &settings.graphics.weather,
                Some(self.camera_matrix(game, debug)),
                f32::from(settings.graphics.motion.wind) / 100.0,
            );
            footprints.append(geometry);
            geometry = footprints;
            let mut weather = self.weather.geometry(
                self.camera_matrix(game, debug),
                alpha,
                &settings.graphics.weather,
            );
            weather.append(geometry);
            geometry = weather;
        }
        if campaign.scene_visible()
            && error.is_none()
            && !debug.collision_only
            && settings.graphics.water.enabled
        {
            let spray = self.spray.geometry(
                self.renderer.visual_time,
                self.camera_matrix(game, debug),
                &settings.graphics.water,
            );
            geometry.append(spray);
        }
        if campaign.scene_visible() && error.is_none() && !debug.collision_only {
            let options = &settings.graphics.effects;
            if settings.graphics.models.grapple {
                geometry.append(crate::visuals::contact::rope::geometry(game));
            }
            if options.active() {
                let mut contact =
                    self.contact
                        .geometry(self.camera_matrix(game, debug), alpha, options);
                contact.append(geometry);
                geometry = contact;
            }
        }
        if campaign.scene_visible() && error.is_none() && debug.photo_mode {
            geometry.hide_ui();
            self.text.clear();
        }
        self.labels.clear();
        if campaign.scene_visible() && error.is_none() && !debug.photo_mode {
            if settings.graphics.lighting.markers {
                let overlay = crate::debug::light_markers(
                    debug,
                    game,
                    self.config.width as f32 / self.config.height as f32,
                    &sources,
                    if settings.graphics.lighting.shadows {
                        usize::from(settings.graphics.lighting.shadow_count)
                    } else {
                        0
                    },
                );
                geometry.append(overlay.geometry);
                self.labels.extend(overlay.labels);
            }
            let overlay = debug.overlay(game, self.config.width as f32 / self.config.height as f32);
            geometry.append(overlay.geometry);
            self.labels.extend(overlay.labels);
            if paused && debug.active() {
                self.labels.push(crate::debug::Label(
                    crate::debug::Anchor::TopLeft([10.0, 132.0]),
                    if cfg!(target_os = "android") {
                        "PAUSED  Pause: resume"
                    } else {
                        "PAUSED  F3: resume  F4: step"
                    }
                    .into(),
                    [1.0, 0.85, 0.35],
                ));
            }
        }
        if matches!(campaign.screen, crate::campaign::Screen::DeathHold { .. }) && !debug.photo_mode
        {
            self.labels.push(crate::debug::Label(
                crate::debug::Anchor::BottomLeft([12.0, 20.0]),
                format!(
                    "Enter / {} / Start: continue",
                    crate::controls::menu_labels(settings.circle_confirm).0
                ),
                [1.0, 0.85, 0.35],
            ));
        }
        if settings.show_fps && !debug.photo_mode {
            let label = if offline { "capture FPS" } else { "FPS" };
            self.labels.push(crate::debug::Label(
                crate::debug::Anchor::TopLeft([10.0, 10.0]),
                format!("{fps:.0} {label}  ·  tick {}", game.tick),
                [1.0, 0.85, 0.35],
            ));
        }
        self.effect_vertices = self.renderer.update_effects(&mut self.effects, geometry);
        self.window.set_title(&format!(
            "Tenchu — gameplay preview | tick {} | HP {} | motion {:04x}",
            game.tick,
            game.player().health,
            game.player().motion.action
        ));
    }

    pub(super) fn draw(
        &mut self,
        game: &Game,
        debug: &crate::debug::Debug,
    ) -> Result<Option<super::performance::DrawTimes>> {
        self.present(|view, target| view.render(target, game, debug))
    }

    pub(super) fn present(
        &mut self,
        render: impl FnOnce(&mut Self, &wgpu::TextureView) -> Result<()>,
    ) -> Result<Option<super::performance::DrawTimes>> {
        // Native presentation also supplies the event loop's vsync pacing.
        if self.size.contains(&0)
            || cfg!(target_arch = "wasm32") && !self.dirty && !self.overlay.dirty
        {
            return Ok(None);
        }
        let start = Instant::now();
        let Some(surface) = &self.surface else {
            return Ok(None);
        };
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                if let Some(surface) = &self.surface {
                    surface.configure(&self.renderer.device, &self.config);
                }
                self.dirty = true;
                return Ok(None);
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let acquire = start.elapsed();
        let start = Instant::now();
        render(self, &frame.texture.create_view(&Default::default()))?;
        let submit = start.elapsed();
        let start = Instant::now();
        // Wayland callbacks also throttle unlocked rendering; retain them for hidden windows.
        if self.config.present_mode == wgpu::PresentMode::AutoVsync || !self.window.has_focus() {
            self.window.pre_present_notify();
        }
        frame.present();
        #[cfg(target_os = "android")]
        crate::android::first_frame();
        self.dirty = false;
        self.overlay.dirty = false;
        Ok(Some(super::performance::DrawTimes {
            submit: submit.as_secs_f32() * 1000.0,
            wait: (acquire + start.elapsed()).as_secs_f32() * 1000.0,
        }))
    }

    fn camera_matrix(&self, game: &Game, debug: &crate::debug::Debug) -> glam::Mat4 {
        let aspect = self.config.width as f32 / self.config.height as f32;
        if debug.camera.or(debug.follow_camera).is_some() {
            debug.matrix(game, aspect)
        } else {
            crate::renderer::game_camera(
                self.frame.as_ref().map_or(game.rendered_view, |f| f.view),
                aspect,
            )
        }
    }

    pub(super) fn render(
        &mut self,
        target: &wgpu::TextureView,
        game: &Game,
        debug: &crate::debug::Debug,
    ) -> Result<()> {
        let terrain = [&self.stage];
        let actors = [&self.actor];
        let casters = [
            self.shadow_stage.as_ref().unwrap_or(&self.stage),
            self.shadow_models.as_ref().unwrap_or(&self.actor),
            &self.actor,
        ];
        if !self.overlay.opaque {
            self.renderer.draw_lit(
                target,
                self.camera_matrix(game, debug),
                crate::renderer::SceneMeshes::new(
                    if debug.collision_only { &[] } else { &terrain },
                    if debug.collision_only { &[] } else { &actors },
                ),
                Some(&self.effects),
                &self.lights,
                &casters[..if self.shared_actor_shadows { 3 } else { 2 }],
            )?;
        }
        self.overlay.draw(&self.renderer, target);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn capture(
        &mut self,
        game: &Game,
        debug: &crate::debug::Debug,
        path: &std::path::Path,
    ) -> Result<()> {
        let texture = self
            .renderer
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Window screenshot"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        self.render(&texture.create_view(&Default::default()), game, debug)?;
        crate::capture::read_texture(&self.renderer, &texture, path)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC and X11 GPU; a newly constructed view must apply graphics and interpolation before its first frame"]
    fn new_view_applies_the_selected_rendering_configuration() {
        use winit::platform::x11::EventLoopBuilderExtX11;
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let game = Options::default().load(&assets).unwrap();
        let mut settings = Settings::default();
        settings.set_visual_preset(settings::presets::SHOWCASE);
        let mut builder = EventLoop::builder();
        builder.with_x11().with_any_thread(true);
        let events = builder.build().unwrap();
        #[allow(deprecated)]
        let window = events
            .create_window(Window::default_attributes().with_visible(false))
            .unwrap();
        let before = crate::session::tests::state(&game);
        let view = pollster::block_on(View::from_window(
            Arc::new(window),
            &assets,
            &game,
            &settings,
        ))
        .unwrap();
        assert_eq!(view.renderer.graphics(), &settings.graphics);
        assert!(view.interpolation_enabled);
        assert_eq!(before, crate::session::tests::state(&game));
        view.renderer.device.poll(wgpu::Maintain::Wait);
        drop(view);
    }
}
