use crate::lighting::{self, Light, Source, Sources};
use glam::Vec3;
use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};

pub(super) struct Lighting {
    fires: Vec<Vec3>,
    pub focus: Option<Vec3>,
}

impl Lighting {
    pub fn new(level: &Level) -> Self {
        Self {
            fires: level
                .scene
                .statics
                .iter()
                // Native environment::update_scenery identifies these as flame emitters.
                .filter(|object| matches!(object.kind, StaticKind::Effect(0..=2 | 6)))
                .map(|object| Vec3::from(object.position.map(|v| v as f32)) - Vec3::Y * 400.0)
                .collect(),
            focus: None,
        }
    }

    pub fn active(&self, settings: &crate::settings::Lighting) -> bool {
        settings.enabled && settings.count > 0 && settings.fires && !self.fires.is_empty()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn count(&self, settings: &crate::settings::Lighting) -> usize {
        if self.active(settings) {
            self.fires.len().min(lighting::MAX_SCENE_LIGHTS)
        } else {
            0
        }
    }

    pub fn sources(&self, settings: &crate::settings::Lighting, eye: Vec3) -> Vec<Light> {
        if !self.active(settings) {
            return Vec::new();
        }
        let fires = self
            .fires
            .iter()
            .map(|&position| Source {
                name: "Fire".into(),
                light: Light {
                    flame: true,
                    emission_radius: 0.0,
                    position,
                    color: [1.0, 0.57, 0.25],
                    radius: f32::from(settings.radius) * 1000.0,
                },
            })
            .collect();
        lighting::select(
            settings,
            self.focus.unwrap_or(eye),
            None,
            Sources {
                fires,
                ..Default::default()
            },
        )
        .into_iter()
        .map(|source| source.light)
        .collect()
    }
}
