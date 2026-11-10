use crate::lighting::{self, Light, Source, Sources};
use glam::Vec3;
use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};

pub(super) struct Lighting {
    fires: Vec<Vec3>,
    pub focus: Option<Vec3>,
    pub debug_light: Option<Vec3>,
    pub transients: Vec<Source>,
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
            debug_light: None,
            transients: Vec::new(),
        }
    }

    pub fn active(&self, settings: &crate::settings::Lighting) -> bool {
        settings.enabled
            && settings.count > 0
            && (self.debug_light.is_some()
                || !self.transients.is_empty()
                || settings.fires && !self.fires.is_empty())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn count(&self, settings: &crate::settings::Lighting) -> usize {
        if self.active(settings) {
            (if settings.fires { self.fires.len() } else { 0 }
                + usize::from(self.debug_light.is_some())
                + self.transients.len())
            .min(lighting::MAX_SCENE_LIGHTS)
        } else {
            0
        }
    }

    pub fn sources(&self, settings: &crate::settings::Lighting, eye: Vec3) -> Vec<Light> {
        self.named_sources(settings, eye)
            .into_iter()
            .map(|source| source.light)
            .collect()
    }

    pub fn named_sources(&self, settings: &crate::settings::Lighting, eye: Vec3) -> Vec<Source> {
        if !self.active(settings) {
            return Vec::new();
        }
        let fires = self
            .fires
            .iter()
            .map(|&position| fire(position, f32::from(settings.radius) * 1000.0))
            .collect();
        let mut sources = self.transients.clone();
        sources.extend(lighting::select(
            settings,
            self.focus.unwrap_or(eye),
            self.debug_light,
            Sources {
                fires,
                ..Default::default()
            },
        ));
        sources.truncate(lighting::MAX_SCENE_LIGHTS);
        sources
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::tenchu2::native) fn nearest(level: &Level, eye: Vec3) -> Option<Source> {
    Lighting::new(level)
        .fires
        .into_iter()
        .min_by(|a, b| {
            eye.distance_squared(*a)
                .total_cmp(&eye.distance_squared(*b))
        })
        .map(|position| fire(position, 12_000.0))
}

fn fire(position: Vec3, radius: f32) -> Source {
    Source {
        name: "Fire".into(),
        light: Light {
            flame: true,
            emission_radius: 0.0,
            position,
            color: [1.0, 0.57, 0.25],
            radius,
        },
    }
}
