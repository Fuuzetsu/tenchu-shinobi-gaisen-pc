use super::super::story::Story;
use super::{Builder, Geometry, ModelStyle, SceneDrawing, build, capture_frame, sources};
use crate::{fonts, renderer::Renderer, settings::Graphics};
use std::path::Path;
use tenchu_assets::{ReadAt, Result, disc::Disc};
use tenchu_core::math::Matrix;

mod models;
mod overlay;
#[cfg(test)]
mod tests;

pub(crate) struct Drawing {
    models: models::Models,
    motion: crate::visuals::motion::Motion,
    pub(super) scene: SceneDrawing,
    pub texts: Vec<fonts::Text>,
    pub labels: Vec<crate::debug::Label>,
    view: Matrix,
    pub camera: Option<crate::debug::camera::Camera>,
    pub alpha: f32,
}

impl Drawing {
    pub fn clear_blood(&mut self) {
        self.scene.clear_blood();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn light_count(&self, settings: &crate::settings::Lighting) -> usize {
        self.scene.lighting.count(settings)
    }

    pub fn clear_weather(&mut self, tracks_only: bool) {
        self.scene.clear_weather(tracks_only);
    }
    pub fn weather_audio(
        &mut self,
        settings: &crate::settings::Weather,
    ) -> (tenchu_audio::Ambience, Option<tenchu_audio::ThunderCue>) {
        self.scene.weather_audio(settings)
    }
    pub fn preserve_environment(&mut self, previous: &mut Self) {
        self.scene.preserve_environment(&mut previous.scene);
        self.motion = std::mem::take(&mut previous.motion);
        self.motion.invalidate();
    }

    pub fn new(story: &Story, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        let mut models = models::Models::new(story, graphics, renderer)?;
        let (geometry, texts) = models.geometry(story, renderer, None, None, None)?;
        Ok(Self {
            models,
            motion: Default::default(),
            scene: SceneDrawing::new(geometry, renderer, &story.level),
            texts,
            labels: Vec::new(),
            view: story.state.view,
            camera: None,
            alpha: 1.0,
        })
    }

    pub fn update(
        &mut self,
        story: &Story,
        renderer: &mut Renderer,
        debug: Option<&crate::debug::Debug>,
    ) -> Result<()> {
        self.tick(story, &renderer.graphics().motion)?;
        let motion = self.motion.sample(if renderer.graphics().interpolation {
            self.alpha
        } else {
            1.0
        });
        let pose = story
            .interpolation
            .as_ref()
            .filter(|_| renderer.graphics().interpolation)
            .and_then(|history| history.sample(story.state.frame.max(0) as u64, self.alpha));
        let (mut geometry, texts) =
            self.models
                .geometry(story, renderer, self.camera, pose.as_ref(), Some(&motion))?;
        let camera = super::camera_matrix(story.state.view, self.camera, renderer.size());
        self.labels = self
            .scene
            .inspect_lights(debug, camera, renderer, &mut geometry);
        self.scene.prepare_environment(
            &mut geometry,
            super::environment::Frame {
                level: &story.level,
                meshes: &self.models.terrain,
                math: &story.math,
                tick: story.state.effects.tick,
                camera,
                alpha: self.alpha,
                contact: Some(&story.contact),
                world: None,
                sprites: &story.sprites,
            },
            renderer,
        );
        self.scene.update(geometry, renderer);
        self.texts = texts;
        self.view = story.state.view;
        Ok(())
    }

    pub fn tick(&mut self, story: &Story, options: &crate::settings::Motion) -> Result<()> {
        if self
            .motion
            .current(story.state.frame.max(0) as u64, options)
        {
            return Ok(());
        }
        let inputs = self.models.motion_inputs(story, options)?;
        self.motion
            .update(story.state.frame.max(0) as u64, &inputs, options);
        Ok(())
    }

    pub fn draw(&self, target: &wgpu::TextureView, renderer: &mut Renderer) -> Result<()> {
        if let Some(camera) = self.camera {
            let [w, h] = renderer.size();
            self.scene
                .draw_camera(camera.matrix(w as f32 / h as f32), target, renderer)
        } else {
            self.scene.draw(self.view, target, renderer)
        }
    }
}

pub(in crate::tenchu2::native) fn capture(
    disc: &Disc<impl ReadAt>,
    sequence: usize,
    event: u16,
    tick: u32,
    seed: u32,
    graphics: &Graphics,
    path: &Path,
) -> Result<()> {
    let mut story = Story::load(disc, sequence, event, seed)?;
    for _ in 0..=tick.min(story.resources.script.duration as u32 - 1) {
        story.step(true)?;
    }
    capture_frame(&story.level.vram, graphics, path, |renderer| {
        let mut drawing = Drawing::new(&story, graphics, renderer)?;
        drawing.update(&story, renderer, None)?;
        Ok((drawing.scene, story.state.view, drawing.texts))
    })?;
    eprintln!(
        "Native JP event {event}, sequence {sequence}, frame {}: {} actor tracks. Other timed 3D effects and distortion remain pending; this still capture has no audio.",
        story.state.frame,
        story.state.actors.len()
    );
    Ok(())
}
