use super::*;
use crate::visuals::{lights::Emitters, models::Models, pack::Pack};

fn model_options(graphics: &settings::Graphics) -> (bool, bool, u16) {
    (
        graphics.asset_pack,
        graphics.smooth_models,
        graphics.model_detail,
    )
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
    pub dirty: bool,
    pub(super) stage: Mesh,
    pub(super) actor: Mesh,
    shadow_stage: Option<Mesh>,
    shadow_models: Option<Mesh>,
    pub(super) lights: Vec<crate::lighting::Light>,
    pub(super) effects: EffectMesh,
    pub(super) labels: Vec<crate::debug::Label>,
    pub(super) overlay: super::overlay::Overlay,
    pub(super) renderer: Renderer,
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) window: Arc<Window>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) size: [u32; 2],
    pub(super) vram_revision: u64,
    pack: Pack,
    models: Models,
    bodies: Pack,
    pub(super) player_models: Vec<(String, String)>,
    player_selection: String,
    model_options: (bool, bool, u16),
    emitters: Emitters,
    pub(super) interpolation: crate::visuals::interpolation::History,
    interpolation_enabled: bool,
    frame: Option<crate::visuals::interpolation::Frame>,
}

impl View {
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
    pub(super) fn new(events: &ActiveEventLoop, game: &Game, settings: &Settings) -> Result<Self> {
        let window = Arc::new(
            events
                .create_window(Self::window_attributes())
                .map_err(|e| e.to_string())?,
        );
        pollster::block_on(Self::from_window(window, game, settings))
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
        game: &Game,
        settings: &Settings,
    ) -> Result<Self> {
        let descriptor = wgpu::InstanceDescriptor::default();
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
        #[cfg(target_arch = "wasm32")]
        crate::web::graphics_backend(
            if adapter.get_info().backend == wgpu::Backend::Gl {
                "WebGL2"
            } else {
                "WebGPU"
            },
            &adapter.get_info().name,
        );
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
        let mut renderer = Renderer::new(
            &adapter,
            config.format,
            [config.width, config.height],
            &game.mission.vram,
        )
        .await?;
        let mut pack = Pack::from_env().unwrap_or_else(|error| {
            eprintln!("Asset pack: {error}; using original assets");
            Pack::default()
        });
        let mut models =
            Models::build(&game.mission, &pack, &settings.graphics).unwrap_or_else(|error| {
                eprintln!("Asset pack: {error}; using original assets");
                pack = Pack::default();
                Models::build(&game.mission, &pack, &settings.graphics).unwrap()
            });
        let bodies = load_bodies(&settings.model_directory).unwrap_or_else(|error| {
            eprintln!("Player models: {error}; using disc bodies");
            Pack::default()
        });
        let player_models = crate::visuals::appearance::choices(game, &bodies);
        models = models
            .with_player(
                game,
                &settings.player_model,
                &pack,
                &bodies,
                &settings.graphics,
            )
            .unwrap_or_else(|error| {
                eprintln!("Player model: {error}; using Original");
                Models::build(&game.mission, &pack, &settings.graphics).unwrap()
            });
        renderer.set_asset_pack(&pack);
        let emitters = Emitters::build(game, &pack, settings.graphics.asset_pack);
        renderer.set_emitters(&emitters);
        renderer.bind_models(&mut models);
        surface.configure(&renderer.device, &config);
        let stage = renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let actor = renderer.upload(&render_mesh::characters(game, Some(&models)));
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
        let overlay = super::overlay::Overlay::new(&window, &renderer, config.format);
        Ok(Self {
            dirty: true,
            overlay,
            window,
            surface,
            config,
            renderer,
            stage,
            actor,
            shadow_stage: None,
            shadow_models: None,
            lights: Vec::new(),
            effects,
            labels: Vec::new(),
            size: [size.width, size.height],
            vram_revision: game.mission.vram.revision,
            pack,
            models,
            bodies,
            player_models,
            player_selection: settings.player_model.clone(),
            model_options: model_options(&settings.graphics),
            emitters,
            interpolation: Default::default(),
            interpolation_enabled: false,
            frame: None,
        })
    }

    fn selected_player<'a>(&self, settings: &'a Settings) -> &'a str {
        if self
            .player_models
            .iter()
            .any(|(id, _)| id == &settings.player_model)
        {
            &settings.player_model
        } else {
            ""
        }
    }

    fn build_models(&self, game: &Game, settings: &Settings) -> Result<Models> {
        Models::build(&game.mission, &self.pack, &settings.graphics)?.with_player(
            game,
            self.selected_player(settings),
            &self.pack,
            &self.bodies,
            &settings.graphics,
        )
    }

    pub(super) fn load_bodies(
        &mut self,
        directory: &str,
        game: &Game,
        settings: &Settings,
    ) -> Result<()> {
        let bodies = load_bodies(directory)?;
        let choices = crate::visuals::appearance::choices(game, &bodies);
        if !directory.is_empty() && choices.iter().all(|(id, _)| !id.starts_with("pack:")) {
            return Err("This folder contains no compatible player_models; export assets to create a template".into());
        }
        let selection = if choices.iter().any(|(id, _)| id == &settings.player_model) {
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
        )?;
        self.bodies = bodies;
        self.player_models = choices;
        self.player_selection = selection.into();
        self.set_models(models, game);
        self.reset_interpolation(game);
        Ok(())
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) {
        self.dirty = true;
        self.size = [width, height];
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.renderer.resize([width, height]);
    }

    pub(super) fn reload(&mut self, game: &Game, settings: &Settings) -> Result<()> {
        self.reset_interpolation(game);
        self.renderer.reset_vram(&game.mission.vram);
        self.vram_revision = game.mission.vram.revision;
        self.player_models = crate::visuals::appearance::choices(game, &self.bodies);
        let result = self.build_models(game, settings);
        let error = result.as_ref().err().cloned();
        self.set_models(
            result.unwrap_or_else(|_| {
                Models::build(&game.mission, &self.pack, &settings.graphics).unwrap_or_default()
            }),
            game,
        );
        self.player_selection.clone_from(&settings.player_model);
        self.model_options = model_options(&settings.graphics);
        self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
        self.renderer.set_emitters(&self.emitters);
        error.map_or(Ok(()), Err)
    }

    fn set_models(&mut self, mut models: Models, game: &Game) {
        self.dirty = true;
        self.renderer.bind_models(&mut models);
        self.shadow_stage = None;
        self.stage = self.renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        self.models = models;
    }

    pub(super) fn load_pack(
        &mut self,
        pack: Pack,
        game: &Game,
        settings: &Settings,
    ) -> Result<usize> {
        let models = Models::build(&game.mission, &pack, &settings.graphics)?.with_player(
            game,
            self.selected_player(settings),
            &pack,
            &self.bodies,
            &settings.graphics,
        )?;
        let count = models.replacements;
        self.renderer.set_asset_pack(&pack);
        self.set_models(models, game);
        self.pack = pack;
        self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
        self.renderer.set_emitters(&self.emitters);
        Ok(count)
    }

    pub(super) fn reset_interpolation(&mut self, game: &Game) {
        self.frame = None;
        if self.interpolation_enabled {
            self.interpolation.reset(game);
        } else {
            self.interpolation = Default::default();
        }
    }

    pub(super) fn settings(&mut self, settings: &Settings, game: &Game) -> Result<()> {
        self.dirty = true;
        if self.interpolation_enabled != settings.graphics.interpolation {
            self.interpolation_enabled = settings.graphics.interpolation;
            self.reset_interpolation(game);
        }
        let options = model_options(&settings.graphics);
        if options != self.model_options || self.player_selection != settings.player_model {
            self.set_models(self.build_models(game, settings)?, game);
            self.model_options = options;
            self.player_selection = settings.player_model.clone();
            self.reset_interpolation(game);
            self.emitters = Emitters::build(game, &self.pack, settings.graphics.asset_pack);
            self.renderer.set_emitters(&self.emitters);
        }
        self.renderer.configure(&settings.graphics);
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
            self.surface.configure(&self.renderer.device, &self.config);
        }
        Ok(())
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
        let game = &campaign.game;
        if self.vram_revision != game.mission.vram.revision {
            self.renderer.update_vram(&game.mission.vram);
            self.vram_revision = game.mission.vram.revision;
        }
        self.frame = if self.interpolation_enabled {
            self.interpolation.sample(game.tick, alpha)
        } else {
            None
        };
        let frame = self.frame.as_ref();
        self.actor = self.renderer.upload(&render_mesh::characters_presented(
            game,
            Some(&self.models),
            debug.camera.is_some() || game.activation.keep_all_active,
            frame,
        ));
        let sources = crate::lighting::sources(
            game,
            &settings.graphics.lighting,
            debug.eye(game),
            debug.light_position(game),
            if settings.graphics.lighting.enabled && settings.graphics.lighting.lamps {
                self.emitters.sources_presented(game, frame)
            } else {
                Vec::new()
            },
        );
        self.lights = sources.iter().map(|s| s.light).collect();
        let shadowing = settings.graphics.lighting.enabled
            && settings.graphics.lighting.shadows
            && ((settings.graphics.lighting.shadow_count > 0 && !self.lights.is_empty())
                || settings.graphics.lighting.directional.intensity > 0);
        if shadowing && self.shadow_stage.is_none() {
            self.shadow_stage = Some(self.renderer.upload(&render_mesh::shadow_stage(
                &game.mission,
                &game.math,
                Some(&self.models),
            )));
        }
        self.shadow_models = shadowing.then(|| {
            self.renderer.upload(&render_mesh::shadow_models_presented(
                game,
                Some(&self.models),
                frame,
            ))
        });
        let mut geometry =
            if campaign.playing() && (menu.open || !paused || debug.active()) && error.is_none() {
                if debug.collision_only {
                    render_effects::Geometry {
                        vertices: Vec::new(),
                        batches: Vec::new(),
                    }
                } else {
                    render_effects::build_presented(
                        game,
                        settings.show_hud && !debug.photo_mode,
                        debug.camera.map(|c| c.source_view()),
                        settings.graphics.blob_shadows,
                        Some(&self.models),
                        frame,
                    )
                }
            } else {
                ui::build(campaign, paused, error)
            };
        if campaign.playing() && error.is_none() && debug.photo_mode {
            geometry.hide_ui();
        }
        self.labels.clear();
        if campaign.playing() && error.is_none() && !debug.photo_mode {
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
                    "PAUSED  F3: resume  F4: step".into(),
                    [1.0, 0.85, 0.35],
                ));
            }
        }
        if settings.show_fps && !debug.photo_mode {
            let label = if offline { "capture FPS" } else { "FPS" };
            self.labels.push(crate::debug::Label(
                crate::debug::Anchor::TopLeft([10.0, 10.0]),
                format!("{fps:.0} {label}  ·  tick {}", game.tick),
                [1.0, 0.85, 0.35],
            ));
        }
        self.effects = self.renderer.upload_effects(geometry);
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
        // Native presentation also supplies the event loop's vsync pacing.
        if self.size.contains(&0)
            || cfg!(target_arch = "wasm32") && !self.dirty && !self.overlay.dirty
        {
            return Ok(None);
        }
        let start = Instant::now();
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                self.dirty = true;
                return Ok(None);
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let acquire = start.elapsed();
        let start = Instant::now();
        self.render(&frame.texture.create_view(&Default::default()), game, debug)?;
        let submit = start.elapsed();
        let start = Instant::now();
        self.window.pre_present_notify();
        frame.present();
        self.dirty = false;
        self.overlay.dirty = false;
        Ok(Some(super::performance::DrawTimes {
            submit: submit.as_secs_f32() * 1000.0,
            wait: (acquire + start.elapsed()).as_secs_f32() * 1000.0,
        }))
    }

    pub(super) fn render(
        &mut self,
        target: &wgpu::TextureView,
        game: &Game,
        debug: &crate::debug::Debug,
    ) -> Result<()> {
        let meshes = [&self.stage, &self.actor];
        let casters = [
            self.shadow_stage.as_ref().unwrap_or(&self.stage),
            self.shadow_models.as_ref().unwrap_or(&self.actor),
        ];
        if !self.overlay.opaque {
            self.renderer.draw_lit(
                target,
                if debug.camera.is_some() {
                    debug.matrix(game, self.config.width as f32 / self.config.height as f32)
                } else {
                    crate::renderer::game_camera(
                        self.frame.as_ref().map_or(game.rendered_view, |f| f.view),
                        self.config.width as f32 / self.config.height as f32,
                    )
                },
                if debug.collision_only { &[] } else { &meshes },
                Some(&self.effects),
                &self.lights,
                &casters,
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

fn load_bodies(directory: &str) -> Result<Pack> {
    if directory.is_empty() {
        Ok(Pack::default())
    } else {
        Pack::load(std::path::Path::new(directory))
    }
}
