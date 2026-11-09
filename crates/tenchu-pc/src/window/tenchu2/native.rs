use super::super::*;
use crate::tenchu2::native::{
    campaign::{Action as NativeAction, Content, Playback},
    presentation::ContentDrawing,
    transport::{Input as PlaybackInput, Transport},
};

pub(in crate::window) struct Native {
    pub playback: Playback<DiscSource>,
    drawing: Option<ContentDrawing>,
    generation: Option<u64>,
    graphics: settings::Graphics,
    pub open: bool,
    diagnostics: bool,
    transport: Transport,
    pub error: Option<String>,
}

impl Application {
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
            graphics: self.settings.graphics.clone(),
            open: false,
            diagnostics: false,
            transport: Transport::default(),
            error: None,
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
        let update = native.transport.frame_step(
            &mut native.playback,
            self.controls.buttons().swap_bytes(),
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
        if matches!(
            self.native_tenchu2.as_ref().unwrap().playback.content,
            Content::Gameplay(_)
        ) && update.menu_pressed & tenchu_core::input::START != 0
        {
            self.paused = !self.paused;
            self.sync_pause();
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        self.sync_audio_pause();
        let input = PlaybackInput {
            buttons: if self.native_tenchu2.as_ref().unwrap().open || !self.focused {
                0
            } else {
                self.controls.buttons().swap_bytes()
            },
            speed: self.settings.cutscene_speed,
            fast_forward: self.controls.held(KeyCode::Backspace),
            paused: self.is_suspended(),
        };
        let native = self.native_tenchu2.as_mut().unwrap();
        let update = native.transport.advance(
            &mut native.playback,
            elapsed,
            input,
            self.audio.as_ref().map(|audio| audio.output()),
        )?;
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
        let suspended = self.is_suspended();
        let Some(view) = &mut self.view else {
            return Ok(());
        };
        let native = self.native_tenchu2.as_mut().unwrap();
        if native.generation != Some(native.playback.generation)
            || native.graphics != self.settings.graphics
        {
            #[cfg(target_os = "android")]
            {
                view.renderer
                    .prepare(&self.settings.graphics, view.size.map(|n| n.max(1)))?;
                if view.preparing() {
                    crate::android::preparing_graphics(true);
                    return Ok(());
                }
            }
            #[cfg(not(target_os = "android"))]
            view.renderer.configure(&self.settings.graphics);
            native
                .drawing
                .get_or_insert(ContentDrawing::Empty)
                .replace(
                    &native.playback.content,
                    native.playback.vram(),
                    &self.settings.graphics,
                    &mut view.renderer,
                )?;
            native.generation = Some(native.playback.generation);
            native.graphics.clone_from(&self.settings.graphics);
            self.last_frame = Instant::now();
            self.clock.discard_host_backlog();
        } else {
            native
                .drawing
                .as_mut()
                .unwrap()
                .update(&native.playback.content, &mut view.renderer)?;
        }
        let mut leave = false;
        let was_open = native.open;
        let was_paused = self.paused;
        let previous_settings = self.settings.clone();
        view.overlay.custom(&view.window, &view.renderer, |ctx| {
            native.drawing.as_mut().unwrap().paint(&native.playback.content, ctx);
            native.playback.action = native.playback.content.inspect(
                ctx, &native.playback.run, &mut native.diagnostics, &mut self.paused,
                native.error.as_deref(),
            ).or(native.playback.action);
            let mut choose_mission = false;
            egui::Window::new("Tenchu 2 settings · F1")
                .open(&mut native.open).default_size([440.0, 400.0]).vscroll(true)
                .show(ctx, |ui| {
                    ui.label("Japanese gameplay, voices, swimming and campaign progression");
                    ui.small("Modern lighting and some effects are still being connected.");
                    ui.checkbox(&mut self.paused, "Pause");
                    ui.checkbox(&mut self.settings.pause_in_menu, "Pause while this menu is open");
                    ui.checkbox(&mut self.settings.pause_on_defocus, "Pause when unfocused");
                    ui.checkbox(&mut self.settings.show_fps, "Show FPS");
                    crate::tenchu2::native::transport::speed_control(ui, &mut self.settings.cutscene_speed);
                    ui.add(egui::Slider::new(&mut self.settings.volume, 0..=100).text("Volume"));
                    ui.separator();
                    ui.checkbox(&mut self.settings.graphics.fog, "Distance fog");
                    ui.checkbox(&mut self.settings.graphics.asset_pack, "Replacement artwork");
                    if let Some(path) = crate::visuals::pack::configured_tenchu2_root() {
                        ui.small(format!("Tenchu 2 artwork: {}", path.display()));
                    } else {
                        ui.small("No Tenchu 2 artwork pack found. Install tenchu2-hd.tar.zst beside the game, or import it in the Android launcher.");
                    }
                    ui.checkbox(&mut self.settings.graphics.bilinear, "Texture filtering");
                    ui.checkbox(&mut self.settings.graphics.mipmaps, "Texture mipmaps");
                    ui.horizontal(|ui| {
                        ui.label("Edge anti-aliasing");
                        for &samples in &view.renderer.sample_counts {
                            ui.selectable_value(&mut self.settings.graphics.samples, samples, format!("{samples}×"));
                        }
                    });
                    ui.separator();
                    ui.checkbox(&mut native.diagnostics, "Show integration diagnostics");
                    choose_mission = ui.button("Original mission selection").clicked();
                    leave |= ui.button("Return to quick launcher · F6").clicked();
                });
            if choose_mission {
                native.playback.action = Some(NativeAction::SelectMission);
                native.open = false;
                self.paused = false;
            }
            if self.settings.show_fps {
                egui::Area::new("native-fps".into()).anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
                    .show(ctx, |ui| { ui.label(format!("{:.0} FPS", self.fps)); });
            }
            let mode = if native.open || native.diagnostics || !matches!(native.playback.content, Content::Gameplay(_)) {
                crate::touch::Mode::Menus
            } else if suspended { crate::touch::Mode::Paused } else { crate::touch::Mode::Gameplay };
            self.touch.layout(ctx.screen_rect(), self.settings.touch, mode, false, false);
            self.touch.paint(ctx, self.settings.touch);
        });
        view.window.set_title(&format!(
            "Tenchu 2 Japan · sequence {} · tick {}",
            native.playback.run.sequence,
            native.playback.content.tick()
        ));
        if view
            .present(|view, target| {
                native.drawing.as_ref().unwrap().draw(
                    &native.playback.content,
                    target,
                    &mut view.renderer,
                )?;
                view.overlay.draw(&view.renderer, target);
                Ok(())
            })?
            .is_some()
        {
            self.fps_frames += 1;
        }
        if was_open != native.open || was_paused != self.paused {
            self.sync_pause();
        }
        if previous_settings != self.settings {
            if let Some(audio) = &mut self.audio {
                audio.volume(self.settings.volume);
            }
            self.settings_store.write(&self.settings)?;
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
                native.open = !native.open;
            }
            crate::touch::Action::Pause => self.paused = !self.paused,
            crate::touch::Action::Back => {
                let native = self.native_tenchu2.as_mut().unwrap();
                if native.open || native.diagnostics {
                    native.open = false;
                    native.diagnostics = false;
                } else {
                    return self.leave_native_tenchu2();
                }
            }
            crate::touch::Action::QuickMenu => return self.leave_native_tenchu2(),
            crate::touch::Action::Recenter => {}
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
                if pressed
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
                        KeyCode::F6 => return self.leave_native_tenchu2(),
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
                        KeyCode::Enter => {
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
                if !pressed || !self.view.as_ref().is_some_and(|v| v.overlay.editing()) {
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
        let Content::Gameplay(session) = &app.native_tenchu2.as_ref().unwrap().playback.content
        else {
            panic!("Not gameplay")
        };
        assert_eq!(session.world.tick, 60);
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
