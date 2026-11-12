use crate::{
    renderer::Renderer,
    settings::{menu::Menu, overlay::Edit},
};
use winit::{event::WindowEvent, window::Window};

pub(crate) struct Overlay {
    pub opaque: bool,
    pub dirty: bool,
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
    context: egui::Context,
    jobs: Vec<egui::ClippedPrimitive>,
    shapes: Vec<egui::epaint::ClippedShape>,
    free_textures: Vec<egui::TextureId>,
    screen: egui_wgpu::ScreenDescriptor,
}

pub(super) struct Extras<'a> {
    pub touch: Option<(
        &'a mut crate::touch::Overlay,
        crate::touch::Mode,
        bool,
        bool,
    )>,
    pub trial: Option<(
        &'a mut crate::trial::Editor,
        &'a tenchu_assets::game::GameAssets<crate::source::DiscSource>,
        &'a tenchu_core::game::Game,
    )>,
    pub labels: &'a [crate::debug::Label],
    pub text: &'a [crate::fonts::Text],
    pub performance: &'a mut super::performance::Performance,
    pub benchmarks: Option<&'a mut super::benchmark::Benchmarks>,
    pub presets: Option<&'a mut crate::settings::library::Manager>,
    pub controls: Option<&'a mut crate::controls::Controls>,
    pub randomizer: Option<crate::randomizer::MenuState<'a>>,
    pub tenchu2: Option<&'a mut crate::tenchu2::menu::Menu>,
    pub launcher: Option<&'a crate::campaign::Campaign>,
    pub models: Option<crate::visuals::appearance::picker::State<'a>>,
    pub demo: Option<&'a mut crate::demo::Player>,
    pub video_time: Option<f64>,
    pub frontend: Option<(
        &'a mut crate::frontend::Frontend,
        &'a crate::campaign::Campaign,
    )>,
}

pub(super) struct Updates {
    trial: Option<crate::trial::Event>,
    edits: Vec<Edit>,
    demo: Option<crate::demo::overlay::Action>,
    frontend: Option<crate::campaign::Action>,
    cancel_benchmark: bool,
    randomizer: Option<crate::randomizer::Event>,
    tenchu2: Option<crate::tenchu2::menu::Event>,
}

impl Overlay {
    pub fn new(window: &Window, renderer: &Renderer, format: wgpu::TextureFormat) -> Self {
        let context = egui::Context::default();
        crate::fonts::install(&context);
        context.set_theme(egui::Theme::Dark);
        context.set_visuals(egui::Visuals::dark());
        context.style_mut(|s| {
            s.visuals.override_text_color = Some(egui::Color32::from_rgb(222, 229, 238));
            s.spacing.slider_width = 100.0;
            s.spacing.interact_size.y = if cfg!(target_os = "android") {
                32.0
            } else {
                22.0
            };
            s.visuals.window_fill = egui::Color32::from_rgb(23, 27, 33);
        });
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(renderer.device.limits().max_texture_dimension_2d as usize),
        );
        let mut overlay_renderer =
            egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
        crate::frontend::gpu::register(&mut overlay_renderer, format);
        Self {
            opaque: false,
            dirty: true,
            renderer: overlay_renderer,
            state,
            context,
            jobs: Vec::new(),
            shapes: Vec::new(),
            free_textures: Vec::new(),
            screen: egui_wgpu::ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
        }
    }

    pub fn event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn touch_point(&self, point: winit::dpi::PhysicalPosition<f64>) -> egui::Pos2 {
        egui::pos2(point.x as f32, point.y as f32) / self.screen.pixels_per_point
    }

    pub fn editing(&self) -> bool {
        self.context.wants_keyboard_input()
    }

    pub fn close_popup(&mut self) -> bool {
        self.context.memory_mut(|memory| {
            let open = memory.any_popup_open();
            memory.close_popup();
            open
        })
    }

    pub fn pointer_over_ui(&self) -> bool {
        self.context.is_pointer_over_area()
    }

    pub(super) fn update(
        &mut self,
        window: &Window,
        renderer: &Renderer,
        menu: &mut Menu,
        state: crate::settings::overlay::State<'_>,
        mut extras: Extras<'_>,
    ) -> Updates {
        self.opaque = extras.frontend.is_some()
            || extras.launcher.is_some()
            || extras.trial.as_ref().is_some_and(|(e, _, _)| e.open);
        let mut input = self.state.take_egui_input(window);
        #[cfg(target_os = "android")]
        let edited = if let Some(events) = crate::android::text_events(&self.context) {
            input.events.extend(events);
            input.focused = true;
            true
        } else {
            false
        };
        #[cfg(target_arch = "wasm32")]
        {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Paste(_)));
            let pasted = crate::web::clipboard_paste();
            if self.editing() && !pasted.is_empty() {
                input
                    .events
                    .push(egui::Event::Paste(pasted.replace("\r\n", "\n")));
            }
        }
        if let Some(time) = extras.video_time {
            self.context.set_zoom_factor(3.0);
            self.context.style_mut(|s| s.animation_time = 0.0);
            input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            ));
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(1.0);
            input.time = Some(time);
            input.predicted_dt = 1.0 / 60.0;
            input.events.clear();
            input.focused = true;
        } else {
            // A native resize can be visible before its Resized event configures the surface.
            let [width, height] = renderer.size();
            let scale = window.scale_factor() as f32 * self.context.zoom_factor();
            input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width as f32, height as f32) / scale,
            ));
        }
        // Navigation is shared with the controller menu; text edits retain native key/IME events.
        if !self.editing()
            && menu.edit.is_none()
            && !extras.trial.as_ref().is_some_and(|(e, _, _)| e.open)
            && !extras.controls.as_ref().is_some_and(|c| c.editor.open)
            && !extras.randomizer.as_ref().is_some_and(|r| r.editor.open)
        {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)));
        }
        let mut trial_action = None;
        let mut edits = Vec::new();
        let mut demo_action = None;
        let mut frontend_action = None;
        let mut cancel_benchmark = false;
        let mut randomizer_action = None;
        let mut tenchu2_action = None;
        let output = self.context.run(input, |ctx| {
            if let Some((editor, assets, game)) = &mut extras.trial {
                trial_action = editor.show(ctx, assets);
                if !editor.open && game.trial.is_some() {
                    crate::trial::status(
                        ctx,
                        game,
                        editor.preview,
                        state.settings.show_trial_timer,
                    );
                }
            }
            if let Some((frontend, campaign)) = &mut extras.frontend {
                frontend_action = frontend.show(
                    ctx,
                    campaign,
                    !menu.open
                        && extras.demo.is_none()
                        && !extras.randomizer.as_ref().is_some_and(|r| r.editor.open),
                    extras.video_time.is_some(),
                    state.settings,
                );
            }
            if !self.opaque {
                crate::fonts::paint(ctx, ctx.screen_rect(), extras.text);
                crate::debug::overlay::paint_labels(ctx, extras.labels);
            }
            let mut show_performance = state.settings.show_performance;
            let profiling = show_performance;
            if show_performance && !state.debug.photo_mode {
                extras.performance.show(
                    ctx,
                    &mut show_performance,
                    menu.open,
                    renderer.timings.is_some(),
                    extras.video_time.is_some(),
                );
            }
            if let Some(demo) = &mut extras.demo {
                demo_action = crate::demo::overlay::show(
                    ctx,
                    demo,
                    state.settings,
                    state.debug,
                    state.paused,
                );
            }
            edits = crate::settings::overlay::show(
                ctx,
                menu,
                state,
                extras.presets.as_ref().map(|p| &p.library),
                extras.models.as_mut(),
            );
            if menu.open
                && let Some(presets) = &mut extras.presets
                && let Some(choice) = presets.show(ctx, state.settings)
            {
                edits.push(Edit::Preset(choice));
            }
            if let Some(benchmarks) = &mut extras.benchmarks {
                cancel_benchmark = benchmarks.show(ctx);
            }
            if menu.open
                && let Some(controls) = &mut extras.controls
            {
                controls.show_bindings(ctx);
            }
            if let Some(randomizer) = &mut extras.randomizer {
                if randomizer.launcher
                    && !menu.open
                    && !randomizer.editor.open
                    && !extras.tenchu2.as_ref().is_some_and(|m| m.open)
                {
                    egui::Area::new("randomizer-launcher".into())
                        .fixed_pos([14.0, 14.0])
                        .show(ctx, |ui| {
                            if ui
                                .add(
                                    egui::Button::new("Randomizer…  F7")
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                )
                                .clicked()
                            {
                                edits.push(Edit::Randomizer);
                            }
                            if extras.launcher.is_none()
                                && ui.button("Quick launcher…  F6").clicked()
                            {
                                tenchu2_action = Some(crate::tenchu2::menu::Event::Quick);
                            }
                        });
                }
                randomizer_action = randomizer.editor.show(
                    ctx,
                    randomizer.catalog.as_ref(),
                    randomizer.active,
                    extras.demo.is_none(),
                );
            }
            if let Some(tenchu2) = &mut extras.tenchu2 {
                if !menu.open
                    && !tenchu2.open
                    && !extras.randomizer.as_ref().is_some_and(|r| r.editor.open)
                    && let Some(campaign) = extras.launcher
                {
                    tenchu2_action = tenchu2
                        .show_launcher(ctx, campaign)
                        .or(tenchu2_action.take());
                }
                tenchu2_action = tenchu2.show(ctx).or(tenchu2_action.take());
            }
            if let Some((touch, playing, tools, camera)) = &mut extras.touch {
                touch.layout(
                    ctx.screen_rect(),
                    state.settings.touch,
                    *playing,
                    *tools,
                    *camera,
                );
                touch.paint(ctx, state.settings.touch);
            }
            if show_performance != profiling {
                edits.push(Edit::Change(crate::settings::menu::Item::Performance, 1));
            }
        });
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::clipboard_editing(self.editing());
            for command in &output.platform_output.commands {
                if let egui::OutputCommand::CopyText(text) = command {
                    crate::web::clipboard_copy(text);
                }
            }
        }
        let platform_output = output.platform_output;
        #[cfg(target_os = "android")]
        let platform_output = {
            let mut output = platform_output;
            if edited {
                self.context.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
                output.ime = None;
            }
            crate::android::platform_output(&self.context, &mut output);
            output
        };
        self.state.handle_platform_output(window, platform_output);
        self.finish_output(
            renderer,
            output.shapes,
            output.pixels_per_point,
            output.textures_delta,
        );
        Updates {
            trial: trial_action,
            edits,
            demo: demo_action,
            frontend: frontend_action,
            cancel_benchmark,
            randomizer: randomizer_action,
            tenchu2: tenchu2_action,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn custom(&mut self, window: &Window, renderer: &Renderer, ui: impl FnMut(&egui::Context)) {
        let mut input = self.state.take_egui_input(window);
        let [width, height] = renderer.size();
        let scale = window.scale_factor() as f32 * self.context.zoom_factor();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width as f32, height as f32) / scale,
        ));
        let output = self.context.run(input, ui);
        self.state
            .handle_platform_output(window, output.platform_output);
        self.finish_output(
            renderer,
            output.shapes,
            output.pixels_per_point,
            output.textures_delta,
        );
    }

    fn finish_output(
        &mut self,
        renderer: &Renderer,
        shapes: Vec<egui::epaint::ClippedShape>,
        pixels_per_point: f32,
        textures_delta: egui::TexturesDelta,
    ) {
        let changed = self.shapes != shapes
            || self.screen.size_in_pixels != renderer.size()
            || self.screen.pixels_per_point != pixels_per_point
            || !textures_delta.is_empty();
        self.dirty |= changed;
        if changed {
            self.shapes = shapes;
            self.jobs = self
                .context
                .tessellate(self.shapes.clone(), pixels_per_point);
        }
        self.screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: renderer.size(),
            pixels_per_point,
        };
        renderer.update_overlay_textures(
            &mut self.renderer,
            &mut self.free_textures,
            textures_delta,
        );
    }

    pub fn draw(&mut self, renderer: &Renderer, target: &wgpu::TextureView) {
        renderer.draw_overlay(target, &mut self.renderer, &self.jobs, &self.screen);
    }
}

impl super::Application {
    pub(super) fn update_overlay(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.native_tenchu2.is_some() {
            return;
        }
        if self.view.as_ref().is_some_and(super::View::preparing) {
            return;
        }
        let was_open = self.menu.open;
        let randomizer_was_open = self.randomizer.open;
        let library_was_open = self.demo.as_ref().is_some_and(|d| d.library);
        let frontend_visible = self.frontend_visible();
        let trial_open = self.trial.open;
        if (self.settings.model_moveset
            || self.campaign.game.moveset.kind.is_some()
            || self.settings.weapon_model != self.campaign.game.moveset.weapon_model
            || self.settings.voice_model != self.campaign.game.moveset.voice_model)
            && !self.campaign.game.cinematic.active
            && !self.campaign.game.player().scripted
            && self.campaign.playing()
            && let Err(error) = self.sync_model_moveset()
        {
            self.menu.notice = error;
        }
        #[cfg(not(target_arch = "wasm32"))]
        let video_time = self
            .video
            .as_ref()
            .map(|v| v.frames as f64 / crate::video::FPS as f64);
        #[cfg(target_arch = "wasm32")]
        let video_time = None;
        if let Some(player) = &self.demo {
            if let Some((run, _)) = &player.randomized {
                self.randomizer.stage_showcase(run, player.randomizer_tab());
            } else {
                self.randomizer.open = false;
            }
        }
        let Some(view) = &mut self.view else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        let assets_ready = true;
        #[cfg(target_arch = "wasm32")]
        let assets_ready =
            crate::web::mission_ready(self.campaign.options.language.unwrap_or_default())
                .unwrap_or(false);
        if view.model_library.is_none()
            && assets_ready
            && ((self.menu.open && self.menu.page.uses_models())
                || !self.settings.player_model.is_empty()
                || !self.settings.weapon_model.is_empty()
                || !self.settings.voice_model.is_empty()
                || self.randomizer.open)
            && let Err(error) =
                view.load_model_library(&self.assets, &self.campaign.game, &self.settings)
        {
            self.randomizer.notice = error.clone();
            self.menu.notice = error;
        }
        self.menu.player_models.clone_from(&view.player_models);
        if self.menu.moveset_models.is_empty()
            && let Some(library) = &view.model_library
        {
            self.menu
                .moveset_models
                .push((String::new(), "Selected player body".into()));
            self.menu.moveset_models.extend(
                library
                    .bodies
                    .iter()
                    .filter(|body| body.has_moveset())
                    .map(|body| (body.id.clone(), body.name.clone())),
            );
        }
        if self.menu.weapon_models.is_empty()
            && let Some(library) = &view.model_library
        {
            self.menu.weapon_models = library.equipment_choices();
            self.menu.voice_models = library.voice_choices();
        }
        self.menu.artwork_status = view.artwork_status();
        let demo_run = self.demo.as_ref().and_then(|d| d.randomized.clone());
        let updates = view.overlay.update(
            &view.window,
            &view.renderer,
            &mut self.menu,
            crate::settings::overlay::State {
                runtime: crate::settings::runtime::Runtime::Gaisen,
                settings: &self.settings,
                debug: &self.debug,
                paused: self.paused,
                lights: view.lights.len(),
                samples: &view.renderer.sample_counts,
            },
            Extras {
                touch: video_time.is_none().then_some((
                    &mut self.touch,
                    if self.debug.camera.is_some() {
                        crate::touch::Mode::FreeCamera
                    } else if self.campaign.playing() {
                        if self.paused {
                            crate::touch::Mode::Paused
                        } else {
                            crate::touch::Mode::Gameplay
                        }
                    } else {
                        crate::touch::Mode::Menus
                    },
                    was_open
                        || trial_open
                        || self.tenchu2.open
                        || self.randomizer.open
                        || self.demo.is_some()
                        || self.benchmarks.run.is_some(),
                    self.settings.navigation.camera_relative || self.debug.camera_actor.is_some(),
                )),
                trial: Some((&mut self.trial, &self.assets, &self.campaign.game)),
                tenchu2: self.demo.is_none().then_some(&mut self.tenchu2),
                launcher: (matches!(self.campaign.screen, crate::campaign::Screen::Launcher)
                    && !trial_open
                    && self.demo.is_none())
                .then_some(&self.campaign),
                labels: &view.labels,
                text: &view.text,
                performance: &mut self.performance,
                benchmarks: Some(&mut self.benchmarks),
                presets: Some(&mut self.presets),
                controls: Some(&mut self.controls),
                randomizer: (self.demo.is_none() || self.randomizer.open).then(|| {
                    crate::randomizer::MenuState {
                        catalog: self
                            .randomizer
                            .open
                            .then(|| {
                                view.model_library.as_ref().map(|library| {
                                    crate::randomizer::Catalog::new(library, &self.campaign.stages)
                                })
                            })
                            .flatten(),
                        editor: &mut self.randomizer,
                        active: demo_run
                            .as_ref()
                            .map(|(run, plan)| (run, plan))
                            .or_else(|| {
                                self.campaign.randomized.as_ref().map(|r| (&r.run, &r.plan))
                            }),
                        launcher: !trial_open
                            && self.campaign.game.trial.is_none()
                            && (frontend_visible
                                || matches!(
                                    self.campaign.screen,
                                    crate::campaign::Screen::Launcher
                                )),
                    }
                }),
                models: Some(crate::visuals::appearance::picker::State {
                    picker: &mut view.model_picker,
                    library: view.model_library.as_ref(),
                    bodies: &view.bodies,
                    game: &self.campaign.game,
                    original_model: None,
                }),
                demo: self.demo.as_mut(),
                video_time,
                frontend: if frontend_visible {
                    self.frontend.as_mut().map(|f| (f, &self.campaign))
                } else {
                    None
                },
            },
        );
        if let Some(event) = updates.trial {
            self.trial_action(event);
            return;
        }
        if let Some(event) = updates.tenchu2 {
            self.tenchu2_action(event);
            return;
        }
        if let Some(event) = updates.randomizer {
            self.randomizer_action(event);
            return;
        }
        if randomizer_was_open != self.randomizer.open {
            self.sync_pause();
            self.update_view();
        }
        if updates.cancel_benchmark {
            if let Err(error) = self.finish_benchmark(Some("Cancelled by user".into())) {
                self.pause_error(error);
            }
            return;
        }
        if let Some(action) = updates.demo {
            self.demo_action(action);
        }
        if let Some(action) = updates.frontend {
            if matches!(self.campaign.screen, crate::campaign::Screen::Results(_)) {
                self.action(action);
            } else {
                self.frontend_action(action);
            }
        }
        if library_was_open != self.demo.as_ref().is_some_and(|d| d.library) {
            self.sync_pause();
        }
        let edits = updates.edits;
        if edits.is_empty() && was_open == self.menu.open {
            return;
        }
        let mut previous = self.settings.clone();
        for edit in edits {
            self.menu.notice.clear();
            let restored = matches!(
                edit,
                Edit::Change(
                    crate::settings::menu::Item::LoadState | crate::settings::menu::Item::Rewind,
                    _
                ) | Edit::SeekRewind(_)
                    | Edit::ImportRewind
            );
            let result = match edit {
                Edit::Preference(change) => {
                    change.apply(&mut self.settings);
                    Ok(())
                }
                Edit::Change(item, steps) => self.apply_item(item, steps),
                Edit::SeekRewind(tick) => self.seek_rewind(tick),
                Edit::SelectRewind(range) => {
                    self.reverse.selection = Some(range);
                    Ok(())
                }
                Edit::ExportRewind => self.export_rewind(),
                Edit::ImportRewind => self.import_rewind(),
                Edit::ImportUnarmed => self.import_unarmed(),
                Edit::ExportUnarmed => self.export_unarmed(),
                Edit::PlayRewindForward => self.play_rewind_forward(),
                Edit::RecordedVisuals(enabled) => {
                    self.reverse.recorded_visuals = Some(enabled);
                    Ok(())
                }
                Edit::ResumeRewind => {
                    self.menu.open = false;
                    self.set_paused(false);
                    Ok(())
                }
                Edit::Preset(choice) => self.presets.library.apply(choice, &mut self.settings),
                Edit::GameplayPreset(index) => self.set_gameplay_preset(index),
                Edit::Benchmark => {
                    self.benchmarks.open = true;
                    Ok(())
                }
                Edit::PresetLibrary => {
                    self.presets.open = true;
                    Ok(())
                }
                Edit::Controls => {
                    self.controls.editor.open = true;
                    self.controls.cancel_capture();
                    self.controls.clear();
                    Ok(())
                }
                Edit::Randomizer => {
                    self.toggle_randomizer();
                    Ok(())
                }
                Edit::Diagnostics => self.save_diagnostics(),
                Edit::Reset(item) => self.reset_item(item),
                Edit::Coordinate(axis, value) => {
                    self.debug.target[axis] = value;
                    Ok(())
                }
            };
            if let Err(error) = result {
                self.menu.notice = error;
            }
            if restored {
                previous = self.settings.clone();
            }
        }
        self.apply_settings(&previous);
        if was_open && !self.menu.open {
            self.view.as_mut().unwrap().overlay.jobs.clear();
            self.controls.close_editor();
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }
}
