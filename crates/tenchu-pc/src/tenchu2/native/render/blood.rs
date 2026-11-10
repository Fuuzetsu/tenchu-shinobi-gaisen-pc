use super::Builder;
use crate::{render_effects, settings::BloodStyle, visuals::models::rotation};
use glam::Vec3;
use tenchu_core::{math::Position, tenchu2::effects::blood::Particle};

impl Builder {
    pub(super) fn blood_particle(
        &mut self,
        position: Position,
        particle: Option<Particle>,
        style: BloodStyle,
        alpha: f32,
    ) -> bool {
        let Some(particle) = particle else {
            return false;
        };
        if style.uses_original() {
            return false;
        }
        if style == BloodStyle::Modern {
            let velocity = Vec3::from(particle.velocity.to_array().map(|v| v as f32));
            let position = Vec3::from(position.to_array().map(|v| v as f32))
                - velocity * (1.0 - alpha.clamp(0.0, 1.0));
            let (size, count) = particle.radius.map_or((11.0, 1), |radius| {
                ((f32::from(radius).abs() / 200.0).clamp(0.5, 2.0) * 19.0, 5)
            });
            render_effects::blood::Drop {
                position,
                velocity,
                count,
                size,
                seed: u32::from(particle.index) * 37 + (size.abs() * 100.0) as u32,
            }
            .draw(rotation(self.view).transpose(), |center, axes, color| {
                self.surface(
                    render_effects::soft_patch(center, axes, color, false, true, 0.9),
                    5,
                );
            });
        }
        true
    }

    pub(super) fn blood_original_hd(&mut self) {
        if let Some(surface) = self.surfaces.last_mut() {
            surface.blend =
                render_effects::blood::original_hd(&mut surface.vertices, surface.blend);
        }
    }
}
