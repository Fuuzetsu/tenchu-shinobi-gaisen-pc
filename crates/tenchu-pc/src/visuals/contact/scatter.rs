use super::*;

impl Contact {
    pub fn scatter(
        &mut self,
        actors: impl Iterator<Item = StepActor> + Clone,
        surfaces: &Surfaces,
        graphics: &Graphics,
        weather: crate::visuals::weather::SurfaceWeather,
        eye: Vec3,
    ) {
        if graphics.effects.scatter {
            let contacts = self.steps.update(actors, surfaces, eye);
            for contact in contacts {
                let p = contact.hit.position;
                let strength = contact.strength;
                for leaf in self.particles.iter_mut().filter(|p| p.ground.is_some()) {
                    let delta = leaf.position - p;
                    if delta.length_squared() < 700.0_f32.powi(2) {
                        leaf.ground = None;
                        leaf.velocity =
                            delta.normalize_or_zero() * strength * 20.0 - Vec3::Y * strength * 18.0;
                    }
                }
                if let Some(tree) = self
                    .canopies
                    .iter()
                    .filter(|t| {
                        p.x > t.min.x - 400.0
                            && p.x < t.max.x + 400.0
                            && p.z > t.min.z - 400.0
                            && p.z < t.max.z + 400.0
                            && p.y >= t.min.y
                    })
                    .min_by(|a, b| {
                        ((a.min + a.max) * 0.5)
                            .distance_squared(p)
                            .total_cmp(&((b.min + b.max) * 0.5).distance_squared(p))
                    })
                    .copied()
                {
                    self.burst(
                        if tree.blossom {
                            Kind::Petal
                        } else {
                            Kind::Leaf
                        },
                        p - Vec3::Y * 12.0,
                        28.0 * strength,
                        (3.0 * strength) as usize,
                        tree.color,
                    );
                }
                let exposed = surfaces.exposed(p - Vec3::Y * 5.0, graphics.weather.direction());
                if weather.snow > 0.03 && exposed {
                    self.burst(
                        Kind::Snow,
                        p - Vec3::Y * 15.0,
                        25.0 * strength,
                        (6.0 * strength) as usize,
                        [0.82, 0.86, 0.92],
                    );
                } else if weather.puddles > 0.04 && weather.wet > 0.2 && exposed {
                    self.burst(
                        Kind::Drop,
                        p - Vec3::Y * 8.0,
                        30.0 * strength,
                        (4.0 * strength) as usize,
                        [0.57, 0.72, 0.85],
                    );
                    self.ripple(p, strength * 0.6);
                }
            }
        }
    }
}
