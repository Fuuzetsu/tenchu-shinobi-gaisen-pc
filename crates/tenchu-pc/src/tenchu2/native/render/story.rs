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
    terrain: super::scenery::Terrain,
    motion: crate::visuals::motion::Motion,
    pub(super) scene: SceneDrawing,
    pub texts: Vec<fonts::Text>,
    pub labels: Vec<crate::debug::Label>,
    view: Matrix,
    pub camera: Option<crate::debug::camera::Camera>,
    pub alpha: f32,
    feedback: crate::renderer::feedback::Request,
}

impl Drawing {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn capture_state(&self) -> super::state::State {
        super::state::State {
            scene: self.scene.capture_state(),
            motion: self.motion.clone(),
            wounds: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn restore_state(
        &mut self,
        state: super::state::State,
        story: &Story,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let inputs = self
            .models
            .motion_inputs(story, &renderer.graphics().motion)?;
        self.motion = state.motion;
        self.motion.rebind_inputs(&inputs);
        self.models.invalidate_textures();
        renderer.clear_feedback();
        self.scene.restore_state(
            state.scene,
            &story.level,
            &self.models.terrain,
            &story.math,
            renderer,
        );
        Ok(())
    }

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
        let (geometry, texts) = models.geometry(story, renderer, Default::default())?;
        let mut scene = SceneDrawing::new(geometry, renderer, &story.level);
        if SceneDrawing::needs_environment(graphics) {
            scene.environment(&story.level, &models.terrain, &story.math, renderer);
        }
        let mut drawing = Self {
            models,
            terrain: Default::default(),
            motion: Default::default(),
            scene,
            texts,
            labels: Vec::new(),
            view: story.state.view,
            camera: None,
            alpha: 1.0,
            feedback: crate::renderer::feedback::Request {
                scene: (story.sequence as u32) << 16 | u32::from(story.event),
                tick: story.state.effects.tick,
                additive: story.state.presentation.warp.map(|warp| warp.variant),
            },
        };
        drawing.tick(story, graphics)?;
        Ok(drawing)
    }

    pub fn update(
        &mut self,
        story: &Story,
        renderer: &mut Renderer,
        debug: Option<&crate::debug::Debug>,
    ) -> Result<()> {
        if SceneDrawing::needs_environment(renderer.graphics()) {
            self.scene
                .environment(&story.level, &self.models.terrain, &story.math, renderer);
        }
        self.tick(story, renderer.graphics())?;
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
        let cached_terrain = !renderer.graphics().fog;
        if cached_terrain {
            self.terrain.prepare(
                super::scenery::Static {
                    level: &story.level,
                    meshes: &self.models.terrain,
                    math: &story.math,
                    phases: None,
                },
                story.state.view,
                renderer,
                &mut self.scene.terrain,
            );
        } else {
            self.terrain.invalidate();
        }
        let (mut geometry, texts) = self.models.geometry(
            story,
            renderer,
            super::Presentation {
                camera: self.camera,
                pose: pose.as_ref(),
                motion: Some(&motion),
                terrain: cached_terrain.then_some(self.terrain.surfaces()),
                actors: std::mem::take(&mut self.scene.actor_vertices),
                effect_vertices: std::mem::take(&mut self.scene.effect_vertices),
                ..Default::default()
            },
        )?;
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
                scenery: None,
                sprites: &story.sprites,
            },
            renderer,
        );
        if cached_terrain {
            self.scene.update_dynamic(geometry, renderer);
        } else {
            self.scene.update(geometry, renderer);
        }
        self.texts = texts;
        self.view = story.state.view;
        Ok(())
    }

    pub fn tick(&mut self, story: &Story, graphics: &Graphics) -> Result<()> {
        self.feedback.tick = story.state.effects.tick;
        self.feedback.additive = story.state.presentation.warp.map(|warp| warp.variant);
        if let Some(environment) = &mut self.scene.environment {
            environment.advance(
                &super::environment::Frame {
                    level: &story.level,
                    meshes: &self.models.terrain,
                    math: &story.math,
                    tick: story.state.effects.tick,
                    camera: super::camera_matrix(story.state.view, self.camera, [4, 3]),
                    alpha: 1.0,
                    contact: Some(&story.contact),
                    world: None,
                    scenery: None,
                    sprites: &story.sprites,
                },
                graphics,
            );
        }
        let options = &graphics.motion;
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
        let previous = renderer.feedback.replace(self.feedback);
        let result = if let Some(camera) = self.camera {
            let [w, h] = renderer.size();
            self.scene
                .draw_camera(camera.matrix(w as f32 / h as f32), target, renderer)
        } else {
            self.scene.draw(self.view, target, renderer)
        };
        renderer.feedback = previous;
        result
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
        "Native JP event {event}, sequence {sequence}, frame {}: {} actor tracks. Authored effects are active; this still capture omits framebuffer history and audio.",
        story.state.frame,
        story.state.actors.len()
    );
    Ok(())
}
