use super::*;

impl Application {
    pub(super) fn benchmark_redraw(&mut self) -> Result<bool> {
        if !self.menu.benchmarks.running() {
            return Ok(false);
        }
        let Some(view) = &mut self.view else {
            return Ok(true);
        };
        let polled = self.controls.poll(self.focused);
        self.menu
            .controller(polled.menu_pressed, self.settings.circle_confirm);
        self.controls.clear();
        self.controls.stop_rumble();
        if let Some(audio) = &self.audio {
            audio.send(Command::Pause {
                paused: true,
                music_while_paused: false,
            });
        }
        let result = self.menu.benchmarks.frame(
            &self.session,
            &self.settings,
            super::super::benchmark::Host {
                window: &view.window,
                surface: &view.surface,
                config: &mut view.config,
                renderer: &mut view.renderer,
                adapter: &view.adapter,
            },
        );
        let finished = match result {
            Ok(finished) => finished,
            Err(error) => {
                let finished = self.menu.benchmarks.finish(Some(error.clone()));
                self.menu.notice(error);
                finished
            }
        };
        if !self.menu.benchmarks.running() {
            if let Some(finished) = &finished
                && finished.options.exit
            {
                self.quit = true;
                self.error = finished.cancelled.clone();
            }
            let result = super::super::benchmark::restore(
                &self.session,
                &mut view.drawing,
                &mut view.renderer,
                &mut self.settings,
                &mut self.presets,
                &self.store,
                finished,
            );
            if let Err(error) = result {
                if self.quit {
                    self.error = Some(error.clone());
                }
                self.menu.notice(error);
            }
            view.graphics = preferences::graphics(&self.settings);
            view.config.present_mode = if self.settings.vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            };
            view.surface.configure(&view.renderer.device, &view.config);
            view.performance = Default::default();
            self.transport.discard_backlog();
            self.last = Instant::now();
        }
        view.window.request_redraw();
        Ok(true)
    }
}
