use super::{Session, campaign::Content, presentation::ContentDrawing};
use crate::{debug::Debug, demo::framing::Path, renderer::Renderer, settings::Settings};
use glam::Vec3;
use serde::Deserialize;
use tenchu_assets::{ReadAt, Result, disc::Disc};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Clip {
    pub id: String,
    pub title: String,
    pub sequence: usize,
    pub distance: f32,
    pub pitch: f32,
}

pub(super) fn catalog() -> Result<Vec<Clip>> {
    let clips: Vec<Clip> =
        serde_json::from_slice(include_bytes!("../../../data/tenchu2-benchmarks.json"))
            .map_err(|e| e.to_string())?;
    Ok(clips)
}

pub(super) struct Scene {
    pub content: Content,
    pub drawing: ContentDrawing,
    pub clip: Clip,
    path: Path,
    debug: Debug,
}

impl Scene {
    pub fn load(
        disc: &Disc<impl ReadAt>,
        clip: Clip,
        settings: &Settings,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let mut session = Session::load(disc, clip.sequence, 1)?;
        for _ in 0..30 {
            session.step(0)?;
        }
        session.interpolate(settings.graphics.interpolation)?;
        let camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
        let target = Vec3::from(
            session.world.actors[session.world.player]
                .actor
                .position
                .to_array()
                .map(|v| v as f32),
        ) - Vec3::Y * 700.0;
        let mut shot = crate::demo::Scene {
            target,
            yaw: camera.yaw,
            pitch: clip.pitch,
            distance: clip.distance,
            ..Default::default()
        };
        let graphics = super::preferences::graphics(settings);
        renderer.configure(&graphics);
        renderer.reset_vram(&session.assets.sprites.vram);
        let models =
            super::render::Models::new(&session.assets, &session.world, &graphics, renderer)?;
        let geometry = models.geometry(
            &session.world,
            &session.assets,
            &session.frame,
            &graphics,
            super::render::Presentation::default(),
        )?;
        crate::demo::framing::unobstructed(&mut shot, &geometry.terrain);
        let eye = |yaw| {
            let view = crate::debug::camera::Camera {
                position: Vec3::ZERO,
                yaw,
                pitch: shot.pitch,
                fov: 55.0,
            };
            (target - view.forward() * shot.distance).to_array()
        };
        let path = Path {
            eye: eye(shot.yaw),
            target: target.to_array(),
            end_eye: Some(eye(shot.yaw + 0.2)),
            end_target: None,
            follow_player: false,
            fov: 55.0,
        };
        path.validate()?;
        let content = Content::Gameplay(Box::new(session));
        let drawing = ContentDrawing::new(&content, &graphics, renderer)?;
        Ok(Self {
            content,
            drawing,
            clip,
            path,
            debug: Debug::default(),
        })
    }

    pub fn advance(&mut self, frame: u64, settings: &Settings) -> Result<()> {
        // Fixed presentation frames drive nominal 30 Hz native updates, irrespective of host FPS.
        if frame.is_multiple_of(2) {
            let Content::Gameplay(session) = &mut self.content else {
                unreachable!()
            };
            session.step(0)?;
            self.drawing
                .tick(&self.content, &super::preferences::graphics(settings))?;
        }
        Ok(())
    }

    pub fn prepare(
        &mut self,
        frame: u64,
        frames: u64,
        settings: &Settings,
        renderer: &mut Renderer,
    ) -> Result<()> {
        self.debug.camera = Some(self.path.sample(frame as f32 / frames.max(1) as f32));
        self.drawing
            .interpolate(if settings.graphics.interpolation {
                if frame.is_multiple_of(2) { 0.0 } else { 0.5 }
            } else {
                1.0
            });
        self.drawing
            .update(&self.content, renderer, &self.debug, settings)
    }
}
