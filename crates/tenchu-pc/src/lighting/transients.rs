mod gaisen;
use super::{Light, Source};
pub use gaisen::sources;
use glam::Vec3;

pub(crate) struct Clusters {
    radius: f32,
    lights: Vec<Source>,
}

impl Clusters {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            lights: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        name: &str,
        position: Vec3,
        strength: f32,
        reach: f32,
        emission_radius: f32,
        flame: bool,
    ) {
        if strength <= 0.01 {
            return;
        }
        let light = Light {
            flame,
            emission_radius,
            position,
            color: [strength, strength * 0.43, strength * 0.12],
            radius: self.radius * reach,
        };
        if let Some(cluster) = self
            .lights
            .iter_mut()
            .find(|s| s.light.position.distance_squared(position) < 2000.0_f32.powi(2))
        {
            if strength > cluster.light.color[0] {
                cluster.light = light;
                cluster.name = name.into();
            }
        } else {
            self.lights.push(Source {
                name: name.into(),
                light,
            });
        }
    }

    pub fn finish(self) -> Vec<Source> {
        self.lights
    }
}
