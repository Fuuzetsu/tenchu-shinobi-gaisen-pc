use super::*;

impl Application {
    pub(super) fn native_benchmark_redraw(&mut self) -> Result<bool> {
        let native = self.native_tenchu2.as_mut().unwrap();
        if !native.menu.benchmarks.running() {
            return Ok(false);
        }
        let Some(view) = &mut self.view else {
            return Ok(true);
        };
        let Some(surface) = view.surface.as_ref() else {
            return Ok(true);
        };
        let polled = self.controls.poll(self.focused);
        native
            .menu
            .controller(polled.menu_pressed, self.settings.circle_confirm);
        self.controls.clear();
        self.controls.stop_rumble();
        if let Some(audio) = &mut self.audio {
            audio.pause(true, false);
        }
        let result = native.menu.benchmarks.frame(
            &native.playback,
            &self.settings,
            crate::tenchu2::native::benchmark::Host {
                window: &view.window,
                surface,
                config: &mut view.config,
                renderer: &mut view.renderer,
                adapter: &view.adapter,
            },
        );
        view.window.request_redraw();
        let finished = match result {
            Ok(finished) => finished,
            Err(error) => {
                let finished = native.menu.benchmarks.finish(Some(error.clone()));
                native.menu.notice(error);
                finished
            }
        };
        if !native.menu.benchmarks.running() {
            let result = crate::tenchu2::native::benchmark::restore(
                &native.playback,
                native.drawing.as_mut().unwrap(),
                &mut view.renderer,
                &mut self.settings,
                &mut self.presets,
                &self.settings_store,
                finished,
            );
            if let Err(error) = result {
                native.menu.notice(error);
            }
            native.graphics = preferences::graphics(&self.settings);
            native.transport.discard_backlog();
            view.presentation_settings(&self.settings);
            self.sync_audio_pause();
            self.last_frame = Instant::now();
            self.performance = Default::default();
        }
        Ok(true)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use winit::platform::x11::EventLoopBuilderExtX11;

    #[test]
    #[ignore = "requires TENCHU_DISC, TENCHU2_DISC, X11 and GPU; main-host benchmark cancellation preserves gameplay/cutscene state and presentation"]
    fn native_benchmark_main_host_restores_modal_and_scene() {
        let mut app = crate::window::tests::application(Default::default());
        let events = winit::event_loop::EventLoop::builder()
            .with_x11()
            .with_any_thread(true)
            .build()
            .unwrap();
        #[allow(deprecated)]
        let window = std::sync::Arc::new(
            events
                .create_window(
                    winit::window::Window::default_attributes()
                        .with_visible(false)
                        .with_inner_size(winit::dpi::PhysicalSize::new(640, 480)),
                )
                .unwrap(),
        );
        app.view = Some(
            pollster::block_on(View::from_window(
                window,
                &app.assets,
                &app.campaign.game,
                &app.settings,
            ))
            .unwrap(),
        );
        for sequence in [9, 23] {
            let disc = crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref())
                .unwrap();
            let playback = Playback::load(disc, sequence, 2, 71, Some([0; 21]), None).unwrap();
            app.native_tenchu2 = Some(Native {
                playback,
                drawing: None,
                generation: None,
                graphics: preferences::graphics(&app.settings),
                menu: preferences::Menu::new(&app.settings),
                transport: Default::default(),
                rewind: Default::default(),
                error: None,
                donor: None,
            });
            app.paused = true;
            app.native_tenchu2_redraw().unwrap();
            app.paused = false;
            let native = app.native_tenchu2.as_mut().unwrap();
            let state = native.playback.capture_state().unwrap();
            let visuals =
                postcard::to_allocvec(&native.drawing.as_ref().unwrap().capture_state().unwrap())
                    .unwrap();
            let preferences = app.settings.clone();
            native.menu.benchmarks.ui.pending = Some(crate::window::benchmark::Options {
                seconds: 2,
                offscreen: true,
                ..Default::default()
            });
            assert!(app.is_suspended());
            for _ in 0..4 {
                assert!(app.native_benchmark_redraw().unwrap());
            }
            app.native_tenchu2
                .as_mut()
                .unwrap()
                .menu
                .benchmarks
                .cancel("Cancelled by user");
            assert!(app.native_benchmark_redraw().unwrap());
            let native = app.native_tenchu2.as_mut().unwrap();
            assert!(!native.menu.benchmarks.running() && native.menu.benchmarks.open());
            assert!(native.playback.capture_state().unwrap() == state);
            assert_eq!(
                postcard::to_allocvec(&native.drawing.as_ref().unwrap().capture_state().unwrap())
                    .unwrap(),
                visuals
            );
            assert_eq!(app.settings, preferences);
            assert_eq!(
                serde_json::to_value(native.menu.benchmarks.ui.report.as_ref().unwrap()).unwrap()["cancelled"],
                "Cancelled by user"
            );
            native.menu.benchmarks.ui.open = false;
            assert!(!app.is_suspended());
        }
    }
}
