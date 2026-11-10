use super::super::{Painter, noise};
use crate::render_effects::style::blood_color;
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game};

impl Painter<'_> {
    pub(in crate::render_effects::combat) fn blood(
        &mut self,
        game: &Game,
        alpha: f32,
        gunfire: bool,
        frame: Option<&crate::visuals::interpolation::Frame>,
    ) {
        for (slot, effect) in crate::visuals::death::blood::slots(&game.effects, frame).enumerate()
        {
            if gunfire
                && game.effects.gun_sources[..game.effects.active_slots().len()]
                    .get(slot)
                    .is_some_and(Option::is_some)
            {
                continue;
            }
            let (position, velocity, count, size) = match effect {
                Some(Effect::Blood(blood)) if blood.mode == 0 => (
                    blood.position,
                    blood.velocity,
                    5,
                    (blood.scale as f32 / 8192.0).clamp(0.5, 2.0) * 19.0,
                ),
                Some(Effect::Bleed(bleed)) if blood_color(bleed.color) => {
                    (bleed.position, bleed.velocity, 1, 11.0)
                }
                _ => continue,
            };
            let velocity = Vec3::from(velocity.map(f32::from));
            let position = Vec3::from(position.map(|n| n as f32).to_array())
                - velocity * (1.0 - alpha.clamp(0.0, 1.0));
            super::Drop {
                position,
                velocity,
                count,
                size,
                seed: slot as u32 * 37,
            }
            .draw(self.camera, |center, axes, color| {
                self.patch(center, axes, color, false, true);
            });
        }
        for (index, burst) in crate::visuals::death::blood::billboards(&game.effects, frame)
            .enumerate()
            .filter(|(_, b)| matches!(b.graphic, tenchu_core::effects::Graphic::Impact(2)))
        {
            let center = Vec3::from(burst.position.map(|v| v as f32).to_array());
            let size = (burst.scale[0] as f32 / 300.0).clamp(20.0, 90.0);
            for i in 0..5 {
                let seed = index as u32 * 31 + i * 7;
                let offset = self.camera.x_axis * (noise(seed) - 0.5)
                    + self.camera.y_axis * (noise(seed + 1) - 0.5);
                self.patch_with_opacity(
                    center + offset * size * 2.0,
                    [
                        self.camera.x_axis * size * 0.45,
                        self.camera.y_axis * size * 0.6,
                    ],
                    [0.38, 0.008, 0.016],
                    false,
                    true,
                    0.4,
                );
            }
        }
    }
}
