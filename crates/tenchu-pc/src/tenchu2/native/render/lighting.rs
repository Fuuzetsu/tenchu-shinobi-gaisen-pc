use crate::lighting::{self, Light, Source, Sources};
use glam::Vec3;
use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};

#[cfg(test)]
mod tests;

pub(super) struct Lighting {
    fires: Vec<Vec3>,
    pub lamps: Vec<Source>,
    pub focus: Option<Vec3>,
    pub debug_light: Option<Vec3>,
    pub transients: Vec<Source>,
    flashes: Vec<Source>,
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
            lamps: Vec::new(),
            focus: None,
            debug_light: None,
            transients: Vec::new(),
            flashes: Vec::new(),
        }
    }

    pub fn update_emitters(
        &mut self,
        emitters: &[tenchu_core::tenchu2::effects::Emitter],
        settings: &crate::settings::Lighting,
    ) {
        use tenchu_core::tenchu2::effects::Emission;
        self.flashes.clear();
        if !settings.enabled || !settings.fires || settings.count == 0 {
            return;
        }
        let mut clusters = lighting::transients::Clusters::new(f32::from(settings.radius) * 1000.0);
        for emitter in emitters {
            let fade = (emitter.remaining as f32 / 6.0).clamp(0.0, 1.0);
            let scale = (f32::from(emitter.size[0]) / 1024.0).clamp(0.0, 3.0);
            let brightness = f32::from(emitter.color[0]) / 128.0 * fade;
            let (name, strength, reach, emission_radius, flame) = match emitter.kind {
                Emission::BurningActor => ("Burning character", scale, 0.5, scale * 500.0, true),
                Emission::Explosion => ("Explosion", scale, 0.9, 0.0, false),
                Emission::Embers => ("Burning embers", scale * 0.12, 0.45, 0.0, true),
            };
            clusters.add(
                name,
                Vec3::from(emitter.position.to_array().map(|v| v as f32)),
                strength * brightness,
                reach,
                emission_radius.min(1000.0),
                flame,
            );
        }
        self.flashes = clusters.finish();
    }

    pub fn active(&self, settings: &crate::settings::Lighting) -> bool {
        settings.enabled
            && settings.count > 0
            && (self.debug_light.is_some()
                || !self.transients.is_empty()
                || settings.lamps && !self.lamps.is_empty()
                || settings.fires && (!self.fires.is_empty() || !self.flashes.is_empty()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn count(&self, settings: &crate::settings::Lighting) -> usize {
        if self.active(settings) {
            (if settings.fires {
                self.fires.len() + self.flashes.len().min(2)
            } else {
                0
            } + if settings.lamps { self.lamps.len() } else { 0 }
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
                lamps: self.lamps.clone(),
                fires,
                flashes: self.flashes.clone(),
            },
        ));
        sources.truncate(lighting::MAX_SCENE_LIGHTS);
        sources
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::tenchu2::native) fn nearest(
    level: &Level,
    math: &tenchu_core::math::Math,
    eye: Vec3,
) -> Option<Source> {
    let fires = Lighting::new(level)
        .fires
        .into_iter()
        .map(|p| fire(p, 12_000.0));
    super::lamps::defaults(level, math)
        .into_iter()
        .chain(fires)
        .min_by(|a, b| {
            eye.distance_squared(a.light.position)
                .total_cmp(&eye.distance_squared(b.light.position))
        })
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
