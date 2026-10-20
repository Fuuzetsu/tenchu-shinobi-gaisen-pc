use super::*;

pub(super) struct View {
    pub(super) stage: Mesh,
    pub(super) actor: Mesh,
    shadow_stage: Option<Mesh>,
    shadow_models: Option<Mesh>,
    lights: Vec<crate::lighting::Light>,
    pub(super) effects: EffectMesh,
    pub(super) renderer: Renderer,
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) window: Arc<Window>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) size: [u32; 2],
    pub(super) vram_revision: u64,
}

impl View {
    pub(super) fn new(events: &ActiveEventLoop, game: &Game) -> Result<Self> {
        let window = Arc::new(
            events
                .create_window(
                    Window::default_attributes()
                        .with_title("Tenchu — gameplay preview")
                        .with_inner_size(LogicalSize::new(1280, 720)),
                )
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::new(&Default::default());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .ok_or("No graphics adapter for this window")?;
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
        let renderer = pollster::block_on(Renderer::new(
            &adapter,
            config.format,
            [config.width, config.height],
            &game.mission.vram,
        ))?;
        surface.configure(&renderer.device, &config);
        let stage = renderer.upload(&render_mesh::stage(&game.mission, &game.math));
        let actor = renderer.upload(&render_mesh::characters(game));
        let effects = renderer.upload_effects(render_effects::build(game));
        println!(
            "Renderer: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );
        Ok(Self {
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
            size: [size.width, size.height],
            vram_revision: game.mission.vram.revision,
        })
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) {
        self.size = [width, height];
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.renderer.device, &self.config);
        self.renderer.resize([width, height]);
    }

    pub(super) fn reload(&mut self, game: &Game) {
        self.renderer.update_vram(&game.mission.vram);
        self.vram_revision = game.mission.vram.revision;
        self.shadow_stage = None;
        self.stage = self
            .renderer
            .upload(&render_mesh::stage(&game.mission, &game.math));
    }

    pub(super) fn settings(&mut self, settings: &Settings) {
        self.renderer.configure(&settings.graphics);
        self.window
            .set_fullscreen(settings.fullscreen.then_some(Fullscreen::Borderless(None)));
        let mode = if settings.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        if self.config.present_mode != mode {
            self.config.present_mode = mode;
            self.surface.configure(&self.renderer.device, &self.config);
        }
    }

    pub(super) fn update(
        &mut self,
        campaign: &Campaign,
        paused: bool,
        error: Option<&str>,
        settings: &Settings,
        menu: &Menu,
        fps: f32,
        debug: &crate::debug::Debug,
    ) {
        let game = &campaign.game;
        if self.vram_revision != game.mission.vram.revision {
            self.renderer.update_vram(&game.mission.vram);
            self.vram_revision = game.mission.vram.revision;
        }
        self.actor = self.renderer.upload(&if debug.camera.is_some() {
            render_mesh::inspection_characters(game)
        } else {
            render_mesh::characters(game)
        });
        self.lights = crate::lighting::sources(
            game,
            &settings.graphics.lighting,
            debug.eye(game),
            debug.light_position(game),
        );
        let shadowing = settings.graphics.lighting.enabled
            && settings.graphics.lighting.shadows
            && !self.lights.is_empty();
        if shadowing && self.shadow_stage.is_none() {
            self.shadow_stage = Some(
                self.renderer
                    .upload(&render_mesh::shadow_stage(&game.mission, &game.math)),
            );
        }
        self.shadow_models =
            shadowing.then(|| self.renderer.upload(&render_mesh::shadow_models(game)));
        let mut geometry = if menu.open {
            menu.build(
                &game.mission.font,
                settings,
                debug,
                paused,
                self.lights.len(),
            )
        } else if campaign.playing() && (!paused || debug.active()) && error.is_none() {
            if debug.collision_only {
                render_effects::Geometry {
                    vertices: Vec::new(),
                    batches: Vec::new(),
                }
            } else {
                render_effects::build_options(
                    game,
                    settings.show_hud,
                    debug.camera.map(|c| c.source_view()),
                    settings.graphics.blob_shadows,
                )
            }
        } else {
            ui::build(campaign, paused, error)
        };
        if campaign.playing() && !menu.open && error.is_none() {
            geometry
                .append(debug.overlay(game, self.config.width as f32 / self.config.height as f32));
            if paused && debug.active() {
                geometry.append(render_effects::text(
                    &game.mission.font,
                    &[(
                        -154,
                        -30,
                        "PAUSED  F3: resume  F4: step".into(),
                        [1.0, 0.85, 0.35],
                    )],
                ));
            }
        }
        if settings.show_fps && !menu.open {
            geometry.append(render_effects::text(
                &game.mission.font,
                &[(
                    -154,
                    -115,
                    format!("{fps:.0} FPS  tick {}", game.tick),
                    [1.0, 0.85, 0.35],
                )],
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

    pub(super) fn draw(&mut self, game: &Game, debug: &crate::debug::Debug) -> Result<()> {
        if self.size.contains(&0) {
            return Ok(());
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.renderer.device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        self.render(&frame.texture.create_view(&Default::default()), game, debug);
        self.window.pre_present_notify();
        frame.present();
        Ok(())
    }

    fn render(&mut self, target: &wgpu::TextureView, game: &Game, debug: &crate::debug::Debug) {
        let meshes = [&self.stage, &self.actor];
        let casters = [
            self.shadow_stage.as_ref().unwrap_or(&self.stage),
            self.shadow_models.as_ref().unwrap_or(&self.actor),
        ];
        self.renderer.draw_lit(
            target,
            debug.matrix(game, self.config.width as f32 / self.config.height as f32),
            if debug.collision_only { &[] } else { &meshes },
            Some(&self.effects),
            &self.lights,
            &casters,
        );
    }

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
        self.render(&texture.create_view(&Default::default()), game, debug);
        crate::capture::read_texture(&self.renderer, &texture, path)
    }
}
