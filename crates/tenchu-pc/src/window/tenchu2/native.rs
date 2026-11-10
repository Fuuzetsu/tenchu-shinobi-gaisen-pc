use super::super::*;
use crate::tenchu2::native::{
    campaign::{Action as NativeAction, Content, Playback},
    preferences,
    presentation::ContentDrawing,
    transport::{Input as PlaybackInput, Transport},
};

pub(in crate::window) struct Native {
    pub playback: Playback<DiscSource>,
    drawing: Option<ContentDrawing>,
    generation: Option<u64>,
    graphics: settings::Graphics,
    pub menu: preferences::Menu,
    transport: Transport,
    pub error: Option<String>,
    donor: Option<Game>,
}

impl Application {
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
        let sequence = self
            .tenchu2
            .sequence
            .ok_or("Select a Japanese mission layout")?;
        let disc = crate::source::open_disc(self.tenchu2.path.as_ref())?;
        let difficulty = match self.campaign.options.difficulty {
            tenchu_core::ai::Difficulty::Easy => 1,
            tenchu_core::ai::Difficulty::Normal => 2,
            tenchu_core::ai::Difficulty::Hard => 3,
        };
        let save = Some(crate::tenchu2::native::save::SaveFile::new()?);
        let playback = if selection {
            Playback::menu(disc, difficulty, self.campaign.seed, save)?
        } else {
            Playback::load(disc, sequence, difficulty, self.campaign.seed, None, save)?
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
        let native = self.native_tenchu2.as_mut().unwrap();
        let buttons = native.menu.tools.input(
            &mut native.playback.content,
            &mut self.controls,
            &self.settings.navigation,
            self.focused && !native.menu.panel.open,
        );
        let update = native.transport.frame_step(
            &mut native.playback,
            buttons,
            self.audio.as_ref().map(|audio| audio.output()),
        )?;
        if update.changed {
            self.last_frame = Instant::now();
        }
        Ok(update.quit)
    }

    fn native_tenchu2_redraw(&mut self) -> Result<()> {
        if self.view.as_ref().is_some_and(View::preparing) {
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
            .controller(update.menu_pressed);
        if !self.native_tenchu2.as_ref().unwrap().menu.panel.open
            && matches!(
                self.native_tenchu2.as_ref().unwrap().playback.content,
                Content::Gameplay(_)
            )
            && update.menu_pressed & tenchu_core::input::START != 0
        {
            self.paused = !self.paused;
            self.sync_pause();
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        self.sync_audio_pause();
        let native = self.native_tenchu2.as_mut().unwrap();
        let buttons = native.menu.tools.input(
            &mut native.playback.content,
            &mut self.controls,
            &self.settings.navigation,
            self.focused && !native.menu.panel.open,
        );
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
        let update = native.transport.advance(
            &mut native.playback,
            elapsed,
            input,
            self.audio.as_ref().map(|audio| audio.output()),
        )?;
        native
            .menu
            .tools
            .sync(&native.playback.content, native.playback.generation);
        native.menu.tools.advance(
            &mut self.controls,
            elapsed.as_secs_f32(),
            self.focused && !native.menu.panel.open,
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
        let mut timing = performance::CpuFrame {
            simulation: now.elapsed().as_secs_f32() * 1000.0,
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
        let wants_models = (native.menu.panel.open && native.menu.panel.page == 10)
            || !self.settings.player_model.is_empty()
            || self.settings.moveset_id().is_some();
        if wants_models && view.model_library.is_none() {
            view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
        }
        let mut models_changed = false;
        if let Content::Gameplay(session) = &mut native.playback.content
            && let Some(library) = &view.model_library
            && (native.donor.is_some()
                || !self.settings.player_model.is_empty()
                || self.settings.moveset_id().is_some())
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
                    models_changed = changed;
                    if changed {
                        session.reset_interpolation()?;
                    }
                }
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
            native
                .transport
                .interpolation(&native.playback.content, input)
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
            self.settings.show_hud,
        )?;
        timing.prepare = prepare.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        let mut leave = false;
        let mut step = false;
        let mut reload_artwork = false;
        let mut save_diagnostics = false;
        let was_open = native.menu.panel.open;
        let was_paused = self.paused;
        let previous_settings = self.settings.clone();
        view.overlay.custom(&view.window, &view.renderer, |ctx| {
            native
                .drawing
                .as_mut()
                .unwrap()
                .paint(&native.playback.content, ctx);
            if !native.menu.panel.open {
                native.playback.action = native
                    .playback
                    .content
                    .inspect(
                        ctx,
                        &native.playback.run,
                        &mut native.menu.diagnostics,
                        &mut self.paused,
                        native.error.as_deref(),
                        native.menu.tools.debug.photo_mode,
                    )
                    .or(native.playback.action);
            }
            native
                .menu
                .cutscene_controls(ctx, native.playback.content.cutscene());
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
            reload_artwork |= actions.reload_artwork;
            save_diagnostics |= actions.save_diagnostics;
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
        if let Some(audio) = &self.audio {
            let (ambience, thunder) = native
                .drawing
                .as_mut()
                .unwrap()
                .weather_audio(&graphics.weather);
            let (_, accelerated) = input.pacing(&native.playback.content);
            audio
                .output()
                .send(crate::audio::output::Command::Ambience(if accelerated {
                    Default::default()
                } else {
                    ambience
                }));
            audio
                .output()
                .send(crate::audio::output::Command::Thunder(if accelerated {
                    None
                } else {
                    thunder
                }));
        }
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
        self.performance.record_at(
            now,
            timing,
            view.renderer.timings.as_ref().and_then(|t| t.latest),
        );
        if was_open != native.menu.panel.open
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
        if step && self.native_tenchu2.as_ref().unwrap().error.is_none() {
            leave |= self.step_native_tenchu2()?;
        }
        if save_diagnostics && let Err(error) = self.save_diagnostics() {
            self.native_tenchu2.as_mut().unwrap().menu.panel.notice = error;
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
                native.menu.panel.open = !native.menu.panel.open;
                self.controls.clear();
            }
            crate::touch::Action::Pause => self.paused = !self.paused,
            crate::touch::Action::Back => {
                let native = self.native_tenchu2.as_mut().unwrap();
                if native.menu.panel.open || native.menu.diagnostics {
                    native.menu.back(&mut self.controls, &mut self.presets);
                } else if native.menu.tools.debug.click_teleport
                    || native.menu.tools.debug.teleport_armed
                {
                    native.menu.tools.debug.click_teleport = false;
                    native.menu.tools.debug.teleport_armed = false;
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
            if let Some(native) = &mut self.native_tenchu2 {
                eprintln!("JP sequence {}: {error}", native.playback.run.sequence);
                native.error = Some(error);
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
            && (native.menu.tools.debug.click_teleport || native.menu.tools.debug.teleport_armed)
            && !native.menu.panel.open
        {
            native.menu.tools.debug.click_teleport = false;
            native.menu.tools.debug.teleport_armed = false;
            return Ok(());
        }
        native.menu.tools.event(
            &event,
            self.focused && !native.menu.panel.open,
            &self.settings.navigation,
        );
        if let Some(view) = &mut self.view {
            view.overlay.event(&view.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => events.exit(),
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
                let menu_open = self.native_tenchu2.as_ref().unwrap().menu.panel.open;
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
                        KeyCode::F6 => return self.leave_native_tenchu2(),
                        KeyCode::F8 => {
                            self.settings.retro = !self.settings.retro;
                            self.settings_store.write(&self.settings)?;
                        }
                        KeyCode::F1 => {
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
                            native.playback.action = match native.playback.content {
                                Content::MissionSelection(_)
                                | Content::Prologue(_)
                                | Content::Equipment(_)
                                | Content::Results(_) => Some(NativeAction::Continue),
                                Content::Failed | Content::Complete => Some(NativeAction::Retry),
                                _ => None,
                            };
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
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC, TENCHU2_JP_DISC and isolated TENCHU_T2_CONFIG/TENCHU_T2_SAVE; main launcher must execute the JP runtime without touching Gaisen progress"]
    fn quick_launcher_runs_native_jp_and_restores_gaisen() {
        use crate::tenchu2::menu::Event;
        let config = std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate disc preferences");
        std::env::var_os("TENCHU_T2_SAVE").expect("Isolate Japanese campaign progress");
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
