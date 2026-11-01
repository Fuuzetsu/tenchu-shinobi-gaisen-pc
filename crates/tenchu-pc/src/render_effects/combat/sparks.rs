use super::{Painter, noise};
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game};

impl Painter<'_> {
    pub(super) fn sparks(&mut self, game: &Game, alpha: f32, density: u16) {
        let density = density.min(300);
        if density == 0 {
            return;
        }
        let alpha = alpha.clamp(0.0, 1.0);
        for (slot, effect) in game.effects.active_slots().iter().enumerate() {
            if game.effects.gun_sources[slot].is_some() {
                continue;
            }
            match effect {
                Some(Effect::Bleed(spark)) if super::super::style::metal_spark(spark.color) => {
                    let velocity = Vec3::from(spark.velocity.map(f32::from));
                    let position = Vec3::from(spark.position.map(|v| v as f32).to_array())
                        - velocity * (1.0 - alpha);
                    let projected =
                        velocity - self.camera.z_axis * velocity.dot(self.camera.z_axis);
                    let along = projected.try_normalize().unwrap_or(self.camera.y_axis);
                    let across = along.cross(self.camera.z_axis).normalize_or_zero();
                    let fade = ((f32::from(spark.time) + 1.0 - alpha) / 6.0).clamp(0.0, 1.0);
                    let cool = spark.color[2] != 0;
                    let count = usize::from(density) / 100
                        + usize::from(noise(slot as u32) < f32::from(density % 100) / 100.0);
                    for i in 0..count {
                        let seed = slot as u32 * 37 + i as u32 * 13;
                        let width = 4.0 + noise(seed + 1) * 4.0;
                        let center = position - velocity * i as f32 * 0.3
                            + across * (noise(seed + 2) - 0.5) * i as f32 * 12.0;
                        let length =
                            (velocity.length() * (0.65 + noise(seed + 3))).clamp(8.0, 90.0);
                        self.patch_with_opacity(
                            center,
                            [across * width, along * length],
                            if cool {
                                [0.55, 1.2, 2.0]
                            } else {
                                [2.0, 1.1, 0.3]
                            },
                            true,
                            true,
                            fade,
                        );
                    }
                }
                Some(Effect::Impact(impact)) if impact.sprite == 1 => {
                    let age = f32::from(impact.count.saturating_sub(1)) + alpha;
                    let fade = (1.0 - age / f32::from(impact.time.max(1))).max(0.0).powi(2);
                    if fade > 0.0 {
                        let size = 60.0 + age * 12.0;
                        self.patch_with_opacity(
                            Vec3::from(impact.position.map(|v| v as f32).to_array()),
                            [self.camera.x_axis * size, self.camera.y_axis * size],
                            [1.7, 1.4, 0.8],
                            true,
                            true,
                            fade,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}
