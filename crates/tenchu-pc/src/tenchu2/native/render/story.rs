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
    scene: SceneDrawing,
    pub texts: Vec<fonts::Text>,
    view: Matrix,
    pub camera: Option<crate::debug::camera::Camera>,
}

impl Drawing {
    pub fn new(story: &Story, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        let mut models = models::Models::new(story, graphics, renderer)?;
        let (geometry, texts) = models.geometry(story, renderer, None)?;
        Ok(Self {
            models,
            scene: SceneDrawing::new(geometry, renderer),
            texts,
            view: story.state.view,
            camera: None,
        })
    }

    pub fn update(&mut self, story: &Story, renderer: &mut Renderer) -> Result<()> {
        let (geometry, texts) = self.models.geometry(story, renderer, self.camera)?;
        self.scene.update(geometry, renderer);
        self.texts = texts;
        self.view = story.state.view;
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
        let drawing = Drawing::new(&story, graphics, renderer)?;
        Ok((drawing.scene, story.state.view, drawing.texts))
    })?;
    eprintln!(
        "Native JP event {event}, sequence {sequence}, frame {}: {} actor tracks. Timed 3D effects and distortion remain pending; this still capture has no audio.",
        story.state.frame,
        story.state.actors.len()
    );
    Ok(())
}
