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
                    let fade = (1.0 - age / 4.0).max(0.0).powi(2);
                    if fade > 0.0 {
                        let center = Vec3::from(impact.position.map(|v| v as f32).to_array());
                        for ray in 0..7 {
                            let seed = slot as u32 * 71 + ray * 13;
                            let angle =
                                ray as f32 * std::f32::consts::TAU / 7.0 + noise(seed) * 0.35;
                            let along =
                                self.camera.x_axis * angle.cos() + self.camera.y_axis * angle.sin();
                            let across = along.cross(self.camera.z_axis);
                            let length = (130.0 + noise(seed + 1) * 220.0) * (1.0 + age * 0.12);
                            self.patch_with_opacity(
                                center + along * length * 0.45,
                                [
                                    across * (3.0 + noise(seed + 2) * 4.0),
                                    along * length * 0.55,
                                ],
                                [2.2, 1.6, 0.75],
                                true,
                                true,
                                fade,
                            );
                        }
                        self.patch_with_opacity(
                            center,
                            [self.camera.x_axis * 25.0, self.camera.y_axis * 25.0],
                            [2.5, 2.4, 2.0],
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
