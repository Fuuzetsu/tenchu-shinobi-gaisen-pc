use super::*;
use tenchu_core::{effects::Effect, game::Game};

impl Contact {
    pub fn tick(
        &mut self,
        game: &Game,
        surfaces: &Surfaces,
        graphics: &Graphics,
        weather: crate::visuals::weather::SurfaceWeather,
        eye: Vec3,
    ) {
        if !self.begin_tick(game.tick, surfaces, graphics, eye) {
            return;
        }
        let options = &graphics.effects;
        let mut splash_slots = BTreeMap::new();
        let mut wire_slots = BTreeMap::new();
        let mut clash_slots = BTreeMap::new();
        for (slot, effect) in game
            .effects
            .active_slots()
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (i, e)))
        {
            match effect {
                Effect::Impact(impact)
                    if options.sparks
                        && options.spark_density > 0
                        && impact.sprite == 1
                        && impact.time == 9
                        && impact.count == 1
                        && point(impact.position).distance_squared(eye) < 35000.0_f32.powi(2) =>
                {
                    self.flash(point(impact.position));
                }
                Effect::Bleed(spark) if options.sparks && spark.color == [0, 127, 255] => {
                    clash_slots.insert(slot, spark.time);
                    if options.spark_density > 0
                        && point(spark.position).distance_squared(eye) < 35000.0_f32.powi(2)
                        && self
                            .clashes
                            .get(&slot)
                            .is_none_or(|&time| spark.time >= time)
                    {
                        self.flash(point(spark.position));
                    }
                }
                Effect::Splash(s) if options.splashes => {
                    let position = point(s.position);
                    splash_slots.insert(slot, position);
                    if self.splashes.get(&slot) != Some(&position)
                        && position.distance_squared(eye) < 35000.0_f32.powi(2)
                    {
                        let strength = (f32::from(s.size[0]) / 8192.0).clamp(0.5, 3.0);
                        self.burst(
                            Kind::Drop,
                            position - Vec3::Y * 8.0,
                            65.0 * strength,
                            12,
                            [0.6, 0.76, 0.86],
                        );
                        self.ripple(position, strength);
                    }
                }
                Effect::FlyWire(w) if options.sparks => {
                    wire_slots.insert(slot, w.mode);
                    if w.mode == 1 && self.wires.get(&slot) != Some(&1) {
                        let hit = surfaces.cast_excluding(
                            point(w.start),
                            point(w.end),
                            render_mesh::FOLIAGE,
                        );
                        let position = hit
                            .map_or_else(|| point(w.end), |hit| hit.position + hit.normal * 40.0);
                        let normal = hit.map_or_else(
                            || {
                                (point(w.start) - point(w.end))
                                    .try_normalize()
                                    .unwrap_or(Vec3::NEG_Y)
                            },
                            |hit| hit.normal,
                        );
                        self.spark(position, normal, options.spark_density, [1.0, 0.64, 0.2]);
                    }
                }
                _ => {}
            }
        }
        self.splashes = splash_slots;
        self.wires = wire_slots;
        self.clashes = clash_slots;
        if options.scatter {
            let contacts = self.steps.update(game, surfaces, eye);
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
