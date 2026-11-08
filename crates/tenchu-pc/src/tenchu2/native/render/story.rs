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
}

impl Drawing {
    pub fn new(story: &Story, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        let mut models = models::Models::new(story, graphics, renderer)?;
        let (geometry, texts) = models.geometry(story, renderer)?;
        Ok(Self {
            models,
            scene: SceneDrawing::new(geometry, renderer),
            texts,
        })
    }

    pub fn update(&mut self, story: &Story, renderer: &mut Renderer) -> Result<()> {
        let (geometry, texts) = self.models.geometry(story, renderer)?;
        self.scene.update(geometry, renderer);
        self.texts = texts;
        Ok(())
    }

    pub fn draw(
        &self,
        view: Matrix,
        target: &wgpu::TextureView,
        renderer: &mut Renderer,
    ) -> Result<()> {
        self.scene.draw(view, target, renderer)
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
