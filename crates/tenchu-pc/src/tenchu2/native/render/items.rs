use super::*;

impl Models {
    pub(super) fn loose_items(
        &self,
        world: &World,
        assets: &Assets,
        scene: &mut tenchu_core::scene::Scene,
        output: &mut Builder,
    ) -> Result<()> {
        let math = &assets.resources.math;
        for entry in &world.items.entries {
            use tenchu_core::tenchu2::items::{State, arrow::Phase};
            let (model, transform, rotation) = match &entry.state {
                State::Pickup(p) => (p.item, p.transform, [0; 3]),
                State::Projectile(p) => (p.model(), p.transform, p.rotation),
                State::Grenade(g) if g.visible => (6, g.transform, [0; 3]),
                State::Mine(m) => (7, m.transform, [0; 3]),
                State::Smoke(s) => (10, s.transform, [0; 3]),
                State::SleepBomb(bomb) => (18, bomb.transform, [0; 3]),
                State::Rice(r) => (11, r.transform, [0; 3]),
                State::Spear(spear) => (24, spear.transform, [0; 3]),
                State::Resurrection(r) => (20, r.transform, [0; 3]),
                State::Marker(marker) => (
                    32 + marker.color,
                    Matrix {
                        translation: marker.position,
                        ..Default::default()
                    },
                    [0; 3],
                ),
                State::Caltrop(c) => (
                    5,
                    Matrix {
                        translation: c.flight.position,
                        ..Default::default()
                    },
                    [0; 3],
                ),
                State::Arrow(arrow) => {
                    if arrow.flame {
                        let image =
                            assets.sprites.effects[12 + (world.tick.wrapping_sub(1) & 3) as usize];
                        let projected = output.project(arrow.flight.position);
                        if projected[2] > 0.0 {
                            let scale = 2400.0 / projected[2];
                            output.screen_sprite(
                                projected,
                                image.size.map(f32::from),
                                [scale; 2],
                                0,
                                [1.0; 3],
                                Some(image),
                                2,
                            );
                        }
                    }
                    if matches!(arrow.phase, Phase::Exploding(_)) && !arrow.flame {
                        continue;
                    }
                    let transform = if let Phase::Embedded {
                        parent: Some(parent),
                        ..
                    } = arrow.phase
                    {
                        scene.world(parent)?.compose(&arrow.transform)
                    } else {
                        arrow.transform
                    };
                    (arrow.model(), transform, arrow.rotation)
                }
                _ => continue,
            };
            if let Some(parts) = assets.items.model(model) {
                let transform = transform.compose(&Matrix {
                    rotation: math.rotate_yxz(rotation),
                    ..Default::default()
                });
                output.parts(
                    &self.items,
                    parts,
                    transform,
                    math,
                    matches!(entry.state, State::Pickup(_)),
                )?;
            }
        }
        Ok(())
    }
}
