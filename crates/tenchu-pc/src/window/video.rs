use super::{Application, performance::CpuFrame};
use tenchu_assets::Result;
use web_time::Instant;

impl Application {
    pub(super) fn export_video_frame(&mut self) -> Result<bool> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        self.fps_elapsed += elapsed.as_secs_f32();
        self.fps_frames += 1;
        if self.fps_elapsed >= 0.5 {
            self.fps = self.fps_frames as f32 / self.fps_elapsed;
            self.fps_elapsed = 0.0;
            self.fps_frames = 0;
        }
        let mut timing = CpuFrame {
            interval: elapsed.as_secs_f32() * 1000.0,
            ..Default::default()
        };
        let phase = Instant::now();
        if self.video.as_ref().unwrap().frames != 0 {
            self.advance_demo_frame();
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        self.debug.update(&self.campaign.game);
        timing.simulation = phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        self.update_view();
        timing.prepare = phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        if self.demo.as_ref().unwrap().frame == 0 {
            self.update_overlay();
        }
        self.update_overlay();
        timing.overlay = phase.elapsed().as_secs_f32() * 1000.0;
        let view = self.view.as_mut().unwrap();
        let video = self.video.as_mut().unwrap();
        view.renderer.profiling = self.settings.show_performance;
        let phase = Instant::now();
        view.render(
            &video.texture.create_view(&Default::default()),
            &self.campaign.game,
            &self.debug,
        )?;
        timing.draw.submit = phase.elapsed().as_secs_f32() * 1000.0;
        let phase = Instant::now();
        video.write(&view.renderer, self.demo.as_ref().unwrap())?;
        timing.draw.wait = phase.elapsed().as_secs_f32() * 1000.0;
        if self.settings.show_performance {
            self.performance.record(
                timing,
                view.renderer.timings.as_ref().and_then(|t| t.latest),
            );
        }
        let complete = video.frames == video.total;
        if complete {
            self.video.take().unwrap().finish()?;
        }
        Ok(complete)
    }
}
