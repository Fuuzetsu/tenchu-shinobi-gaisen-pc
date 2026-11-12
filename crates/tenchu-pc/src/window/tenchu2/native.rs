use super::super::*;
use crate::tenchu2::native::{
    campaign::{Action as NativeAction, Content, Playback},
    preferences,
    presentation::ContentDrawing,
    transport::{Input as PlaybackInput, Transport},
};

mod benchmark;
mod rewind;
mod state;

pub(in crate::window) struct Native {
    pub playback: Playback<DiscSource>,
    drawing: Option<ContentDrawing>,
    generation: Option<u64>,
    graphics: settings::Graphics,
    pub menu: preferences::Menu,
    transport: Transport,
    pub rewind: crate::tenchu2::native::rewind::Rewind,
    pub error: Option<String>,
    donor: Option<Game>,
}

impl Application {
    fn native_toggle_randomizer(&mut self) -> Result<()> {
        let native = self.native_tenchu2.as_mut().unwrap();
        native
            .menu
            .toggle_randomizer(&mut native.playback, Some(&self.disc))?;
        self.controls.close_editor();
        self.sync_pause();
        Ok(())
    }

    pub(in crate::window) fn native_diagnostic_report(&self) -> Result<Vec<u8>> {
        let native = self.native_tenchu2.as_ref().unwrap();
        let mut report = crate::tenchu2::native::diagnostics::report(
            &native.playback,
            &native.transport,
            &native.menu,
            &self.settings,
            &self.controls,
        );
        report["runtime"]["paused"] = serde_json::json!(self.paused);
        report["runtime"]["suspended"] = serde_json::json!(self.is_suspended());
        report["runtime"]["focused"] = serde_json::json!(self.focused);
        report["runtime"]["error"] = serde_json::json!(native.error);
        report["runtime"]["audio_active"] = serde_json::json!(self.audio.is_some());
        report["runtime"]["performance"] = self.performance.diagnostic_snapshot();
        if let Some(view) = &self.view {
            let mut renderer = crate::diagnostics::renderer(
                &view.adapter,
                &view.window,
                &view.config,
                &view.renderer,
            );
            renderer["lights"] = serde_json::json!(
                native
                    .drawing
                    .as_ref()
                    .map(|d| d.light_count(&native.graphics.lighting))
            );
            renderer["fps"] = serde_json::json!(self.fps);
            report["renderer"] = renderer;
            report["effective_graphics"] = serde_json::json!(view.renderer.graphics());
        }
        crate::diagnostics::finish(report, &self.settings, self.settings_store.path.parent())
    }

    pub(in crate::window) fn start_native_tenchu2(&mut self, selection: bool) -> Result<()> {
        let difficulty = match self.campaign.options.difficulty {
            tenchu_core::ai::Difficulty::Easy => 1,
            tenchu_core::ai::Difficulty::Normal => 2,
            tenchu_core::ai::Difficulty::Hard => 3,
        };
        let playback = if self.tenchu2.imported() {
            let path = self
                .tenchu2
                .japanese_disc()
                .ok_or("Connect a Japanese Tenchu 2 disc first")?;
            let disc = crate::source::open_disc(path.as_ref())?;
            let options = self.campaign.options;
            let stage = self
                .campaign
                .stages
                .iter()
                .find(|s| s.order == options.mission as i32)
                .ok_or("Unknown Gaisen mission")?
                .id;
            if !(1..=3).contains(&options.layout) {
                return Err("Select a Gaisen layout from 1 to 3".into());
            }
            let source = crate::tenchu2::native::gaisen::Source::open(
                &self.disc,
                stage,
                i16::from(self.tenchu2.gaisen_hero == 1),
                options.layout,
                options
                    .language
                    .unwrap_or(self.campaign.game.mission.scripts.language),
            )?;
            Playback::gaisen(
                disc,
                source,
                self.tenchu2.gaisen_hero,
                difficulty,
                self.campaign.seed,
                None,
                self.tenchu2.gaisen_water_depth,
            )?
        } else {
            let sequence = self
                .tenchu2
                .sequence
                .ok_or("Select a Japanese mission layout")?;
            let disc = crate::source::open_disc(self.tenchu2.path.as_ref())?;
            let save = Some(crate::tenchu2::native::save::SaveFile::new()?);
            if selection {
                Playback::menu(disc, difficulty, self.campaign.seed, save)?
            } else {
                Playback::load(disc, sequence, difficulty, self.campaign.seed, None, save)?
            }
        };
        self.finish_recording();
        self.performance = Default::default();
        self.release_pointer();
        if let Some(frontend) = &mut self.frontend {
            frontend.leave();
        }
        if let Some(audio) = &mut self.audio {
            audio.begin_tenchu2();
            playback.content.start_audio(audio.output());
        }
        self.native_tenchu2 = Some(Native {
            playback,
            drawing: None,
            generation: None,
            graphics: preferences::graphics(&self.settings),
            menu: preferences::Menu::new(&self.settings),
            transport: Transport::default(),
            rewind: Default::default(),
            error: None,
            donor: None,
        });
        self.tenchu2.open = false;
        self.menu.open = false;
        self.randomizer.open = false;
        self.paused = false;
        self.faulted = false;
        self.fault = None;
        self.sync_pause();
        Ok(())
    }

    fn leave_native_tenchu2(&mut self) -> Result<()> {
        self.controls.stop_rumble();
        self.native_tenchu2 = None;
        self.performance = Default::default();
        self.controls.close_editor();
        self.presets.open = false;
        self.touch.clear();
        self.campaign.screen = Screen::Launcher;
        self.campaign.quick_launcher = true;
        self.campaign.cursor = 1;
        self.reload_game()?;
        self.update_view();
        self.update_overlay();
        Ok(())
    }

    pub(in crate::window) fn step_native_tenchu2(&mut self) -> Result<bool> {
        self.controls.stop_rumble();
        let native = self.native_tenchu2.as_mut().unwrap();
        native.rewind.resume()?;
        if !native.rewind.preview {
            native
                .playback
                .configure_onikage(&self.assets, &self.settings)?;
            native.playback.configure(&self.settings)?;
        }
        let buttons = if native.rewind.preview {
            0
        } else {
            native.menu.tools.input(
                &mut native.playback.content,
                &mut self.controls,
                &self.settings,
                self.focused && !native.menu.panel.open && !native.menu.modal_open(),
            )
        };
        let ready = native.generation == Some(native.playback.generation)
            && !native.menu.actor_assets_changed;
        let graphics = self.settings.rendering().graphics.clone();
        let update = native.transport.frame_step(
            &mut native.playback,
            buttons,
            self.audio.as_ref().map(|audio| audio.output()),
            |playback, observed| {
                native
                    .rewind
                    .observed(playback, observed, &native.menu.tools);
                if let Some(drawing) = native
                    .drawing
                    .as_mut()
                    .filter(|_| ready && !observed.replaced)
                {
                    drawing.tick(&playback.content, &graphics)?;
                    if let Some(error) = native.rewind.record(
                        playback,
                        drawing,
                        &mut self.settings,
                        &native.menu.tools,
                    ) {
                        native.menu.notice(error);
                    }
                }
                Ok(())
            },
        )?;
        if update.changed {
            self.last_frame = Instant::now();
        }
        Ok(update.quit)
    }

    fn native_tenchu2_redraw(&mut self) -> Result<()> {
        if self.native_benchmark_redraw()? {
            return Ok(());
        }
        if self.view.as_ref().is_some_and(View::preparing) {
            self.controls.stop_rumble();
            return Ok(());
        }
        let update = self.controls.poll(self.focused);
        if update.disconnected {
            self.paused = true;
            self.sync_pause();
        }
        self.native_tenchu2
            .as_mut()
            .unwrap()
            .menu
            .controller(update.menu_pressed, self.settings.circle_confirm);
        if !self.native_tenchu2.as_ref().unwrap().menu.panel.open
            && !self.native_tenchu2.as_ref().unwrap().menu.modal_open()
            && self
                .native_tenchu2
                .as_mut()
                .unwrap()
                .playback
                .controller_pause(update.menu_pressed, self.settings.circle_confirm)
        {
            self.paused = !self.paused;
            self.sync_pause();
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        if !self.paused && self.native_tenchu2.as_ref().unwrap().rewind.preview {
            self.native_tenchu2.as_mut().unwrap().rewind.resume()?;
        }
        self.advance_native_rewind(elapsed);
        self.sync_audio_pause();
        let native = self.native_tenchu2.as_mut().unwrap();
        if !native.rewind.preview {
            native
                .playback
                .configure_onikage(&self.assets, &self.settings)?;
            native.playback.configure(&self.settings)?;
        }
        let buttons = if native.rewind.preview {
            0
        } else {
            native.menu.tools.input(
                &mut native.playback.content,
                &mut self.controls,
                &self.settings,
                self.focused && !native.menu.panel.open && !native.menu.modal_open(),
            )
        };
        let input = PlaybackInput {
            buttons,
            cutscene_speed: self.settings.cutscene_speed,
            simulation_speed: self.native_tenchu2.as_ref().unwrap().menu.tools.debug.speed,
            fast_forward: self.controls.held(KeyCode::Backspace)
                || self.native_tenchu2.as_ref().unwrap().menu.fast_forward,
            paused: self.is_suspended(),
        };
        let native = self.native_tenchu2.as_mut().unwrap();
        native
            .playback
            .content
            .interpolate(self.settings.rendering().graphics.interpolation)?;
        let ready = native.generation == Some(native.playback.generation)
            && !native.menu.actor_assets_changed;
        let graphics = self.settings.rendering().graphics.clone();
        let feedback_enabled = self.focused
            && !input.paused
            && !input.pacing(&native.playback.content).1
            && self.settings.frontend.rumble;
        let update = if native.rewind.preview {
            native.transport.preview(
                &mut native.playback,
                self.audio.as_ref().map(|a| a.output()),
            );
            crate::tenchu2::native::transport::Update::default()
        } else {
            native.transport.advance(
                &mut native.playback,
                elapsed,
                input,
                self.audio.as_ref().map(|audio| audio.output()),
                |playback, observed| {
                    let content = &playback.content;
                    native
                        .rewind
                        .observed(playback, observed, &native.menu.tools);
                    self.controls.native_feedback(
                        content.feedback(),
                        feedback_enabled,
                        &self.settings.vibration,
                    );
                    if let Some(drawing) = native
                        .drawing
                        .as_mut()
                        .filter(|_| ready && !observed.replaced)
                    {
                        drawing.tick(content, &graphics)?;
                        if let Some(error) = native.rewind.record(
                            playback,
                            drawing,
                            &mut self.settings,
                            &native.menu.tools,
                        ) {
                            native.menu.notice(error);
                        }
                    }
                    Ok(())
                },
            )?
        };
        self.controls.native_feedback(
            native.playback.content.feedback(),
            feedback_enabled && !native.playback.death.frozen(),
            &self.settings.vibration,
        );
        native
            .menu
            .tools
            .sync(&native.playback.content, native.playback.generation);
        native.menu.tools.advance(
            &mut self.controls,
            elapsed.as_secs_f32(),
            self.focused && !native.menu.panel.open && !native.menu.modal_open(),
            &self.settings.navigation,
        );
        if update.quit {
            return self.leave_native_tenchu2();
        }
        if update.changed {
            self.last_frame = Instant::now();
        }
        self.fps_elapsed += elapsed.as_secs_f32();
        if self.fps_elapsed >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_elapsed;
            self.fps_elapsed = 0.0;
            self.fps_frames = 0;
        }
        let rewind_cpu = std::mem::take(&mut native.rewind.cpu).as_secs_f32() * 1000.0;
        let mut timing = performance::CpuFrame {
            simulation: (now.elapsed().as_secs_f32() * 1000.0 - rewind_cpu).max(0.0),
            rewind: rewind_cpu,
            ..Default::default()
        };
        let prepare = Instant::now();
        let suspended = self.is_suspended();
        let graphics = preferences::graphics(&self.settings);
        let Some(view) = &mut self.view else {
            return Ok(());
        };
        view.presentation_settings(&self.settings);
        view.renderer.profiling = self.settings.show_performance;
        let native = self.native_tenchu2.as_mut().unwrap();
        let randomized = native.playback.randomized().is_some();
        let wants_models = !randomized
            && ((native.menu.panel.open && native.menu.panel.page.uses_models())
                || !native
                    .playback
                    .content
                    .current_model(native.playback.run.hero, &self.settings));
        if wants_models && view.model_library.is_none() {
            view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
            if native
                .playback
                .content
                .current_model(native.playback.run.hero, &self.settings)
                && let Some(pack) = native.playback.content.embedded_bodies()
            {
                view.bodies = pack;
            }
        }
        let mut models_changed = std::mem::take(&mut native.menu.actor_assets_changed);
        if randomized {
            models_changed |= native
                .playback
                .randomizer_models(&self.settings, &view.bodies)?;
        }
        if let Content::Gameplay(session) = &mut native.playback.content
            && !randomized
            && let Some(library) = &view.model_library
            && (native.donor.is_some()
                || !self.settings.player_model.is_empty()
                || self.settings.moveset_id().is_some()
                || !self.settings.weapon_model.is_empty()
                || !self.settings.voice_model.is_empty())
        {
            if native.donor.is_none() {
                native.donor = Some(crate::tenchu2::native::models::donor(&self.assets)?);
            }
            match session.models.sync(
                &mut session.world,
                &mut session.assets,
                native.donor.as_mut().unwrap(),
                library,
                &view.bodies,
                &self.settings,
            ) {
                Ok(changed) => {
                    models_changed |= changed;
                    if changed {
                        session.reset_interpolation()?;
                    }
                }
                Err(error) => native.menu.panel.notice = error,
            }
        }
        if let Content::Story { story, .. } = &mut native.playback.content
            && !randomized
            && let Some(library) = &view.model_library
        {
            match story.models.sync(
                &story.resources,
                native.playback.run.hero,
                library,
                &view.bodies,
                &self.settings,
            ) {
                Ok(changed) => models_changed |= changed,
                Err(error) => native.menu.panel.notice = error,
            }
        }
        if native.generation != Some(native.playback.generation)
            || native.graphics != graphics
            || models_changed
        {
            #[cfg(target_os = "android")]
            {
                view.renderer
                    .prepare(&graphics, view.size.map(|n| n.max(1)))?;
                if view.preparing() {
                    crate::android::preparing_graphics(true);
                    return Ok(());
                }
            }
            #[cfg(not(target_os = "android"))]
            view.renderer.configure(&graphics);
            if native.generation != Some(native.playback.generation)
                || crate::visuals::models::Options::from(&native.graphics)
                    != crate::visuals::models::Options::from(&graphics)
                || models_changed
            {
                native
                    .drawing
                    .get_or_insert(ContentDrawing::Empty)
                    .replace(
                        &native.playback.content,
                        native.playback.vram(),
                        &graphics,
                        &mut view.renderer,
                        native.generation == Some(native.playback.generation),
                    )?;
            }
            native.generation = Some(native.playback.generation);
            native.graphics.clone_from(&graphics);
            self.last_frame = Instant::now();
            self.clock.discard_host_backlog();
        }
        let drawing = native.drawing.as_mut().unwrap();
        drawing.interpolate(if graphics.interpolation {
            native.transport.interpolation(&native.playback, input)
        } else {
            1.0
        });
        native.menu.tools.update_camera(
            &native.playback.content,
            drawing,
            &self.settings.navigation,
            elapsed.as_secs_f32(),
            &mut view.renderer,
        );
        drawing.update(
            &native.playback.content,
            &mut view.renderer,
            &native.menu.tools.debug,
            &self.settings,
        )?;
        if let Some(error) = native.rewind.record(
            &native.playback,
            drawing,
            &mut self.settings,
            &native.menu.tools,
        ) {
            native.menu.notice(error);
        }
        native.rewind.sync(&mut native.menu.tools);
        let rewind_cpu = std::mem::take(&mut native.rewind.cpu).as_secs_f32() * 1000.0;
        timing.rewind += rewind_cpu;
        timing.prepare = (prepare.elapsed().as_secs_f32() * 1000.0 - rewind_cpu).max(0.0);
        let phase = Instant::now();
        let screenshot = std::mem::take(&mut native.menu.screenshot);
        let mut leave = false;
        let mut step = false;
        let mut reload_artwork = false;
        let mut save_diagnostics = false;
        let mut export_unarmed = false;
        let mut state_action = None;
        let mut rewind_action = None;
        let mut open_randomizer = false;
        let mut randomizer_action = None;
        let was_open = native.menu.panel.open || native.menu.modal_open();
        let was_paused = self.paused;
        let previous_settings = self.settings.clone();
        view.overlay.custom(&view.window, &view.renderer, |ctx| {
            native
                .drawing
                .as_mut()
                .unwrap()
                .paint(&native.playback.content, ctx);
            if !native.menu.panel.open && !native.menu.modal_open() {
                native.playback.action = native
                    .playback
                    .content
                    .controls(ctx, self.settings.circle_confirm)
                    .or(native.playback.action);
                native.playback.content.inspect(
                    ctx,
                    &native.playback.run,
                    &mut native.menu.diagnostics,
                    &mut self.paused,
                    native.error.as_deref(),
                    native.menu.tools.debug.photo_mode,
                );
            }
            native
                .menu
                .cutscene_controls(ctx, native.playback.content.cutscene());
            if !native.menu.panel.open && !native.menu.modal_open() {
                native
                    .playback
                    .death_controls(ctx, native.menu.tools.debug.photo_mode);
            }
            if self.settings.show_performance && !native.menu.tools.debug.photo_mode {
                self.performance.show(
                    ctx,
                    &mut self.settings.show_performance,
                    native.menu.panel.open,
                    view.renderer.timings.is_some(),
                    false,
                );
            }
            let actions = native.menu.show(
                ctx,
                &mut preferences::State {
                    settings: &mut self.settings,
                    paused: &mut self.paused,
                    controls: &mut self.controls,
                    presets: &mut self.presets,
                    lights: native
                        .drawing
                        .as_ref()
                        .unwrap()
                        .light_count(&view.renderer.graphics().lighting),
                    samples: &view.renderer.sample_counts,
                    models: Some(crate::visuals::appearance::picker::State {
                        picker: &mut view.model_picker,
                        library: view.model_library.as_ref(),
                        bodies: &view.bodies,
                        game: &self.campaign.game,
                        original_model: if let Content::Gameplay(session) = &native.playback.content
                        {
                            session
                                .assets
                                .resources
                                .registry
                                .actor(session.world.actors[session.world.player].actor.kind)
                                .map(|a| format!("tenchu2-jp:{}", a.model))
                        } else {
                            None
                        },
                    }),
                },
            );
            if let Some(bodies) = actions.models {
                view.bodies = bodies;
                view.model_picker.clear();
            }
            if let Some(request) = actions.spawn {
                native
                    .menu
                    .apply_spawn(&mut native.playback, request, view.model_library.as_ref());
            }
            reload_artwork |= actions.reload_artwork;
            save_diagnostics |= actions.save_diagnostics;
            export_unarmed |= actions.export_unarmed;
            state_action = actions.state;
            rewind_action = actions.rewind;
            open_randomizer = actions.open_randomizer;
            randomizer_action = native.menu.show_randomizer(ctx, &native.playback);
            if actions.clear_blood {
                native.drawing.as_mut().unwrap().clear_blood();
            }
            if actions.clear_weather || actions.clear_footprints {
                native
                    .drawing
                    .as_mut()
                    .unwrap()
                    .clear_weather(!actions.clear_weather);
            }
            native
                .menu
                .apply_tools(&mut native.playback.content, view.size);
            leave |= actions.leave;
            step |= actions.step;
            if actions.select_mission {
                native.playback.action = Some(NativeAction::SelectMission);
                native.menu.panel.open = false;
                self.paused = false;
            }
            if self.settings.show_fps && !native.menu.tools.debug.photo_mode {
                egui::Area::new("native-fps".into())
                    .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                    .show(ctx, |ui| {
                        ui.label(format!("{:.0} FPS", self.fps));
                    });
            }
            let mode = if native.menu.panel.open
                || native.menu.modal_open()
                || native.menu.diagnostics
                || !matches!(native.playback.content, Content::Gameplay(_))
            {
                crate::touch::Mode::Menus
            } else if native.menu.tools.debug.camera.is_some() {
                crate::touch::Mode::FreeCamera
            } else if suspended {
                crate::touch::Mode::Paused
            } else {
                crate::touch::Mode::Gameplay
            };
            self.touch
                .layout(ctx.screen_rect(), self.settings.touch, mode, false, false);
            self.touch.paint(ctx, self.settings.touch);
        });
        timing.overlay = phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        if reload_artwork {
            match native.drawing.as_mut().unwrap().replace(
                &native.playback.content,
                native.playback.vram(),
                &graphics,
                &mut view.renderer,
                true,
            ) {
                Ok(()) => native.menu.panel.notice = "Artwork reloaded".into(),
                Err(error) => native.menu.panel.notice = error,
            }
        }
        timing.prepare += phase.elapsed().as_secs_f32() * 1000.0;
        let (_, accelerated) = input.pacing(&native.playback.content);
        native.drawing.as_mut().unwrap().presentation_audio(
            &native.playback.content,
            &graphics.weather,
            accelerated || native.rewind.preview,
            self.audio.as_ref().map(|audio| audio.output()),
        )?;
        view.window.set_title(&format!(
            "Tenchu 2 Japan · sequence {} · tick {}",
            native.playback.run.sequence,
            native.playback.content.tick()
        ));
        if let Some(draw) = view.present(|view, target| {
            native.drawing.as_ref().unwrap().draw(
                &native.playback.content,
                target,
                &mut view.renderer,
            )?;
            view.overlay.draw(&view.renderer, target);
            Ok(())
        })? {
            timing.draw = draw;
            self.fps_frames += 1;
        }
        if screenshot {
            let result = (|| {
                let path = crate::capture::screenshot_path(&format!(
                    "tenchu2-{}-{}",
                    native.playback.run.sequence,
                    native.playback.content.tick()
                ))?;
                crate::capture::render_to_png(
                    &mut view.renderer,
                    view.size,
                    view.config.format,
                    &path,
                    |renderer, target| {
                        native.drawing.as_ref().unwrap().draw(
                            &native.playback.content,
                            target,
                            renderer,
                        )?;
                        view.overlay.draw(renderer, target);
                        Ok(())
                    },
                )
            })();
            native.menu.screenshot_result(result);
        }
        self.performance.record_at(
            now,
            timing,
            view.renderer.timings.as_ref().and_then(|t| t.latest),
        );
        if was_open != (native.menu.panel.open || native.menu.modal_open())
            || was_paused != self.paused
            || previous_settings.pause_in_menu != self.settings.pause_in_menu
            || previous_settings.pause_on_defocus != self.settings.pause_on_defocus
            || previous_settings.music_while_paused != self.settings.music_while_paused
        {
            if !native.menu.panel.open {
                self.controls.close_editor();
                self.presets.open = false;
            }
            self.sync_pause();
        }
        if previous_settings != self.settings {
            if let Some(audio) = &mut self.audio {
                audio.volume(self.settings.volume);
            }
            self.settings_store.write(&self.settings)?;
        }
        if open_randomizer {
            self.native_toggle_randomizer()?;
        }
        if let Some(event) = randomizer_action {
            let native = self.native_tenchu2.as_mut().unwrap();
            if let Err(error) = native.menu.randomizer_action(&mut native.playback, event) {
                native.menu.panel.notice = error;
            } else {
                native.error = None;
                native.rewind.clear();
                self.paused = false;
            }
            self.sync_pause();
        }
        if step && self.native_tenchu2.as_ref().unwrap().error.is_none() {
            leave |= self.step_native_tenchu2()?;
        }
        if let Some(action) = rewind_action {
            self.native_rewind_action(action);
        }
        if let Some(save) = state_action {
            self.native_state_action(save);
        }
        if save_diagnostics && let Err(error) = self.save_diagnostics() {
            self.native_tenchu2.as_mut().unwrap().menu.panel.notice = error;
        }
        if export_unarmed {
            let native = self.native_tenchu2.as_mut().unwrap();
            native.menu.export_unarmed(&native.playback);
            native.transport.discard_backlog();
            self.last_frame = Instant::now();
        }
        if leave {
            self.leave_native_tenchu2()?;
        }
        Ok(())
    }

    fn native_tenchu2_touch(&mut self, action: crate::touch::Action) -> Result<()> {
        match action {
            crate::touch::Action::Settings => {
                let native = self.native_tenchu2.as_mut().unwrap();
                native.menu.benchmarks.ui.open = false;
                if let Some(editor) = &mut native.menu.randomizer {
                    editor.open = false;
                }
                native.menu.panel.open = !native.menu.panel.open;
                self.controls.clear();
            }
            crate::touch::Action::Pause => {
                if self.native_tenchu2.as_ref().unwrap().rewind.preview {
                    self.native_rewind_action(crate::tenchu2::native::rewind::Action::Resume);
                } else {
                    self.paused = !self.paused;
                }
            }
            crate::touch::Action::Back => {
                let native = self.native_tenchu2.as_mut().unwrap();
                if native.menu.panel.open || native.menu.diagnostics || native.menu.modal_open() {
                    native.menu.back(&mut self.controls, &mut self.presets);
                } else if native.menu.tools.debug.click_teleport
                    || native.menu.tools.debug.click_kill
                    || native.menu.tools.debug.teleport_armed
                    || native.menu.tools.debug.pick_actor_position
                {
                    native.menu.tools.debug.click_teleport = false;
                    native.menu.tools.debug.click_kill = false;
                    native.menu.tools.debug.teleport_armed = false;
                    native.menu.tools.debug.pick_actor_position = false;
                } else if native.menu.tools.debug.camera.is_some()
                    || native.menu.tools.debug.camera_actor.is_some()
                {
                    native.menu.tools.debug.return_camera();
                    self.controls.clear();
                } else {
                    return self.leave_native_tenchu2();
                }
            }
            crate::touch::Action::QuickMenu => return self.leave_native_tenchu2(),
            crate::touch::Action::Recenter => {}
        }
        if !self.native_tenchu2.as_ref().unwrap().menu.panel.open {
            self.controls.close_editor();
            self.presets.open = false;
        }
        self.sync_pause();
        Ok(())
    }

    pub(in crate::window) fn native_tenchu2_event(
        &mut self,
        events: &ActiveEventLoop,
        event: WindowEvent,
    ) {
        let result = self.handle_native_tenchu2_event(events, event);
        if let Err(error) = result {
            self.controls.stop_rumble();
            if let Some(native) = &mut self.native_tenchu2 {
                eprintln!("JP sequence {}: {error}", native.playback.run.sequence);
                native.error = Some(error);
                native.playback.content.clear_feedback();
                self.paused = true;
                self.sync_pause();
            } else {
                self.pause_error(error);
            }
        }
    }

    fn handle_native_tenchu2_event(
        &mut self,
        events: &ActiveEventLoop,
        event: WindowEvent,
    ) -> Result<()> {
        if let Some(view) = &self.view
            && self
                .native_tenchu2
                .as_mut()
                .unwrap()
                .menu
                .benchmarks
                .event(&view.window, &event)
        {
            return Ok(());
        }
        if let WindowEvent::Touch(touch) = &event
            && let Some(view) = &self.view
        {
            let (handled, action) = self.touch.event(
                touch.id,
                touch.phase,
                view.overlay.touch_point(touch.location),
            );
            self.controls.touch(self.touch.sample());
            if let Some(action) = action {
                self.native_tenchu2_touch(action)?;
            }
            if handled || self.native_tenchu2.is_none() {
                return Ok(());
            }
        }
        let native = self.native_tenchu2.as_mut().unwrap();
        if matches!(event, WindowEvent::KeyboardInput { ref event, .. } if event.state == ElementState::Pressed && event.physical_key == PhysicalKey::Code(KeyCode::Escape))
            && (native.menu.tools.debug.click_teleport
                || native.menu.tools.debug.click_kill
                || native.menu.tools.debug.teleport_armed
                || native.menu.tools.debug.pick_actor_position)
            && !native.menu.panel.open
            && !native.menu.modal_open()
        {
            native.menu.tools.debug.click_teleport = false;
            native.menu.tools.debug.click_kill = false;
            native.menu.tools.debug.teleport_armed = false;
            native.menu.tools.debug.pick_actor_position = false;
            return Ok(());
        }
        native.menu.tools.event(
            &event,
            self.focused
                && !native.menu.panel.open
                && !native.menu.modal_open()
                && !native.menu.diagnostics
                && !self
                    .view
                    .as_ref()
                    .is_some_and(|v| v.overlay.pointer_over_ui()),
            &self.settings.navigation,
        );
        if let Some(view) = &mut self.view {
            view.overlay.event(&view.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.controls.stop_rumble();
                events.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(view) = &mut self.view {
                    view.resize(size.width, size.height)?;
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.sync_pause();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if !self.focused || is_synthetic || event.repeat {
                    return Ok(());
                }
                let code = match event.physical_key {
                    PhysicalKey::Code(code) => code,
                    _ if event.logical_key
                        == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack) =>
                    {
                        KeyCode::Escape
                    }
                    _ => return Ok(()),
                };
                let pressed = event.state == ElementState::Pressed;
                if self.controls.editor.open {
                    if pressed {
                        self.controls.editor_key(code);
                    }
                    return Ok(());
                }
                let menu = &self.native_tenchu2.as_ref().unwrap().menu;
                let menu_open = menu.panel.open || menu.randomizer_open();
                if pressed
                    && !menu_open
                    && matches!(
                        self.native_tenchu2.as_ref().unwrap().playback.content,
                        Content::Gameplay(_)
                    )
                    && matches!(
                        self.controls.key_command(code, true, self.paused),
                        Some(Command::Pause)
                    )
                {
                    return self.native_tenchu2_touch(crate::touch::Action::Pause);
                }
                if pressed {
                    match code {
                        KeyCode::Backspace
                            if !menu_open
                                && !menu.modal_open()
                                && self
                                    .native_tenchu2
                                    .as_ref()
                                    .unwrap()
                                    .playback
                                    .content
                                    .cutscene() =>
                        {
                            self.controls.key(code, true);
                            return Ok(());
                        }
                        KeyCode::Escape | KeyCode::BrowserBack => {
                            return self.native_tenchu2_touch(crate::touch::Action::Back);
                        }
                        KeyCode::F2 if !menu_open => {
                            let menu = &mut self.native_tenchu2.as_mut().unwrap().menu;
                            if let Err(error) =
                                menu.tools.change(settings::menu::Item::FreeCamera, 1)
                            {
                                menu.panel.notice = error;
                            }
                            self.controls.clear();
                            return Ok(());
                        }
                        KeyCode::F10 if !menu_open => {
                            self.native_tenchu2
                                .as_mut()
                                .unwrap()
                                .menu
                                .tools
                                .change(settings::menu::Item::PhotoMode, 1)?;
                            return Ok(());
                        }
                        KeyCode::F9 => {
                            self.native_tenchu2
                                .as_mut()
                                .unwrap()
                                .menu
                                .request_screenshot();
                            return Ok(());
                        }
                        KeyCode::F11 | KeyCode::F12 => {
                            if code == KeyCode::F12
                                && (self.controls.held(KeyCode::ControlLeft)
                                    || self.controls.held(KeyCode::ControlRight))
                            {
                                self.native_rewind_action(
                                    crate::tenchu2::native::rewind::Action::Toggle,
                                );
                                return Ok(());
                            }
                            self.native_state_action(code == KeyCode::F11);
                            return Ok(());
                        }
                        KeyCode::F6 => return self.leave_native_tenchu2(),
                        KeyCode::F7 => return self.native_toggle_randomizer(),
                        KeyCode::F8 => {
                            self.settings.retro = !self.settings.retro;
                            self.settings_store.write(&self.settings)?;
                        }
                        KeyCode::F1 => {
                            self.native_tenchu2
                                .as_mut()
                                .unwrap()
                                .menu
                                .benchmarks
                                .ui
                                .open = false;
                            return self.native_tenchu2_touch(crate::touch::Action::Settings);
                        }
                        KeyCode::F3 => {
                            return self.native_tenchu2_touch(crate::touch::Action::Pause);
                        }
                        KeyCode::F4
                            if self.paused
                                && self.native_tenchu2.as_ref().unwrap().error.is_none() =>
                        {
                            if self.step_native_tenchu2()? {
                                self.leave_native_tenchu2()?;
                            }
                            return Ok(());
                        }
                        KeyCode::Enter if !menu_open => {
                            let native = self.native_tenchu2.as_mut().unwrap();
                            native.playback.confirm();
                        }
                        _ => {}
                    }
                }
                if !pressed
                    || !menu_open && !self.view.as_ref().is_some_and(|v| v.overlay.editing())
                {
                    self.controls.key(code, pressed);
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(target_os = "android")]
                if crate::android::take_back() {
                    return self.native_tenchu2_touch(crate::touch::Action::Back);
                }
                self.native_tenchu2_redraw()?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod launcher_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC, TENCHU2_JP_DISC and isolated TENCHU_T2_CONFIG/TENCHU_T2_SAVE/TENCHU_T2_RANDOMIZER; main launcher and native seeds must preserve normal progress"]
    fn quick_launcher_runs_native_jp_and_restores_gaisen() {
        use crate::tenchu2::menu::Event;
        let config = std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate disc preferences");
        std::env::var_os("TENCHU_T2_SAVE").expect("Isolate Japanese campaign progress");
        let seed_path =
            std::env::var_os("TENCHU_T2_RANDOMIZER").expect("Isolate native seed preferences");
        let disc = std::env::var_os("TENCHU2_JP_DISC").unwrap();
        let mut app = crate::window::tests::application(Options::default());
        app.campaign.screen = Screen::Launcher;
        app.campaign.options.difficulty = tenchu_core::ai::Difficulty::Hard;
        app.campaign.saved = Some(Default::default());
        let save = app.campaign.saved.as_ref().unwrap().encode();
        let player = app.campaign.game.player().position;
        app.tenchu2.path = disc.to_string_lossy().into_owned();
        app.apply_tenchu2(Event::Connect).unwrap();
        assert!(app.tenchu2.native());
        app.tenchu2.sequence = Some(9);
        app.tenchu2.map = app
            .tenchu2
            .layouts
            .iter()
            .find(|layout| layout.sequence == 9)
            .unwrap()
            .map;
        app.tenchu2_action(Event::Play);
        assert!(matches!(
            app.native_tenchu2.as_ref().unwrap().playback.content,
            Content::Equipment(_)
        ));
        for _ in 0..15 {
            app.step_native_tenchu2().unwrap();
        }
        app.native_tenchu2.as_mut().unwrap().playback.action = Some(NativeAction::Continue);
        for _ in 0..30 {
            if matches!(
                app.native_tenchu2.as_ref().unwrap().playback.content,
                Content::Gameplay(_)
            ) {
                break;
            }
            app.step_native_tenchu2().unwrap();
        }
        let native = app
            .native_tenchu2
            .as_ref()
            .expect("F6 used the Gaisen sandbox");
        assert_eq!(native.playback.run.hero, 0);
        assert_eq!(native.playback.run.difficulty, 3);
        let Content::Gameplay(session) = &native.playback.content else {
            panic!("Not gameplay")
        };
        assert!(!session.bank.samples.is_empty());
        let start = session.world.actors[session.world.player].actor.position;
        app.controls.key(KeyCode::KeyW, true);
        for _ in 0..60 {
            assert!(!app.step_native_tenchu2().unwrap());
        }
        app.controls.key(KeyCode::KeyW, false);
        let sampled = Instant::now();
        app.performance.record_at(
            sampled,
            performance::CpuFrame {
                prepare: 5.0,
                ..Default::default()
            },
            None,
        );
        app.performance.record_at(
            sampled + std::time::Duration::from_millis(20),
            performance::CpuFrame::default(),
            None,
        );
        let Content::Gameplay(session) = &app.native_tenchu2.as_ref().unwrap().playback.content
        else {
            panic!("Not gameplay")
        };
        assert_eq!(session.world.tick, 60);
        let before = serde_json::to_vec(&session.world).unwrap();
        let report = crate::diagnostics::decode(&app.diagnostic_report().unwrap()).unwrap();
        assert_eq!(report["engine"], "tenchu2-jp");
        assert_eq!(report["game"]["tick"], 60);
        assert_eq!(report["session"]["sequence"], 9);
        assert_eq!(report["runtime"]["performance"]["cpu"][0][0], 20.0);
        assert_eq!(report["runtime"]["performance"]["cpu"][0][3], 5.0);
        assert_eq!(
            report["recent_inputs"]["inputs"].as_array().unwrap().len(),
            60
        );
        assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
        assert_ne!(
            session.world.actors[session.world.player].actor.position,
            start
        );
        assert_eq!(app.campaign.game.tick, 0);
        assert_eq!(app.campaign.game.player().position, player);
        app.native_tenchu2_touch(crate::touch::Action::Settings)
            .unwrap();
        assert!(app.is_suspended());
        app.native_tenchu2_redraw().unwrap();
        let Content::Gameplay(session) = &app.native_tenchu2.as_ref().unwrap().playback.content
        else {
            panic!("Not gameplay")
        };
        assert_eq!(session.world.tick, 60);
        app.native_tenchu2_touch(crate::touch::Action::Back)
            .unwrap();
        assert!(!app.is_suspended());
        let normal = app
            .native_tenchu2
            .as_ref()
            .unwrap()
            .playback
            .capture_state()
            .unwrap();
        let preferences = app.settings.clone();
        app.native_toggle_randomizer().unwrap();
        assert!(app.native_tenchu2.as_ref().unwrap().menu.modal_open());
        assert!(app.is_suspended());
        app.native_tenchu2_redraw().unwrap();
        assert!(
            app.native_tenchu2
                .as_ref()
                .unwrap()
                .playback
                .capture_state()
                .unwrap()
                == normal
        );
        let native = app.native_tenchu2.as_mut().unwrap();
        let run = crate::randomizer::Run::new(crate::randomizer::Document::for_source(
            crate::randomizer::Source::Tenchu2,
        ));
        native
            .menu
            .randomizer_action(
                &mut native.playback,
                crate::randomizer::Event::start(run.clone()),
            )
            .unwrap();
        assert_eq!(native.playback.randomized().unwrap().0, &run);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&seed_path).unwrap()).unwrap();
        assert_eq!(
            saved["current"]["document"]["format"],
            "tenchu2-jp-randomizer"
        );
        native
            .menu
            .randomizer_action(&mut native.playback, crate::randomizer::Event::End)
            .unwrap();
        assert!(native.playback.capture_state().unwrap() == normal);
        assert_eq!(app.settings, preferences);
        std::fs::remove_file(seed_path).unwrap();
        app.native_tenchu2_touch(crate::touch::Action::QuickMenu)
            .unwrap();
        assert!(app.native_tenchu2.is_none());
        assert!(matches!(app.campaign.screen, Screen::Launcher));
        assert!(app.campaign.quick_launcher);
        assert_eq!(app.campaign.saved.as_ref().unwrap().encode(), save);
        assert!(app.campaign.save_request.is_none());
        assert_eq!(app.campaign.game.tick, 0);
        app.apply_tenchu2(Event::MissionSelection).unwrap();
        assert!(matches!(
            app.native_tenchu2.as_ref().unwrap().playback.content,
            Content::MissionSelection(_)
        ));
        app.native_tenchu2_touch(crate::touch::Action::QuickMenu)
            .unwrap();
        assert_eq!(app.campaign.saved.as_ref().unwrap().encode(), save);
        // The old cross-game sandbox remains an explicit choice, not the JP default.
        app.tenchu2.sandbox = true;
        app.apply_tenchu2(Event::Play).unwrap();
        assert!(app.native_tenchu2.is_none());
        assert!(app.campaign.sandbox.is_some());
        assert!(app.campaign.playing());
        std::fs::remove_file(config).unwrap();
    }
}
