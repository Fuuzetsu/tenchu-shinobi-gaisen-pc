pub mod foliage;
mod geometry;
pub mod rope;
mod steps;

use super::surfaces::{Hit, Stamp, Surfaces};
use crate::{
    render_mesh::{self, Vertex},
    settings::Graphics,
};
use glam::Vec3;
use std::collections::BTreeMap;
use tenchu_core::{effects::Effect, game::Game};

const LIMIT: usize = 2048;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Petal,
    Leaf,
    Snow,
    Drop,
    Spark,
}
struct Particle {
    kind: Kind,
    position: Vec3,
    previous: Vec3,
    velocity: Vec3,
    color: [f32; 3],
    size: f32,
    age: f32,
    life: f32,
    phase: f32,
    ground: Option<Vec3>,
    origin: Vec3,
}
struct Ripple {
    position: Vec3,
    age: f32,
    strength: f32,
}
struct WetMark {
    vertices: Vec<Vertex>,
    age: f32,
}
struct Flash {
    position: Vec3,
    age: f32,
}

#[derive(Default)]
pub struct Contact {
    tick: Option<u64>,
    seed: u32,
    particles: Vec<Particle>,
    rings: Vec<Ripple>,
    wet: Vec<WetMark>,
    flashes: Vec<Flash>,
    splashes: BTreeMap<usize, Vec3>,
    wires: BTreeMap<usize, u8>,
    steps: steps::Tracker,
    pub canopies: Vec<foliage::Canopy>,
}
fn point(p: tenchu_core::math::Position) -> Vec3 {
    p.map(|n| n as f32).to_array().into()
}
impl Contact {
    pub fn clear(&mut self) {
        let canopies = std::mem::take(&mut self.canopies);
        *self = Self {
            canopies,
            ..Default::default()
        };
    }
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / 16777216.0
    }
    fn particle(&mut self, kind: Kind, position: Vec3, velocity: Vec3, color: [f32; 3], size: f32) {
        if self.particles.len() >= LIMIT {
            return;
        }
        let phase = self.random() * std::f32::consts::TAU;
        let life = match kind {
            Kind::Petal | Kind::Leaf => 35.0 + self.random() * 25.0,
            Kind::Snow => 1.3,
            Kind::Drop => 1.5,
            Kind::Spark => 0.35 + self.random() * 0.3,
        };
        self.particles.push(Particle {
            kind,
            position,
            previous: position,
            velocity,
            color,
            size,
            age: 0.0,
            life,
            phase,
            ground: None,
            origin: position,
        });
    }
    fn burst(&mut self, kind: Kind, position: Vec3, strength: f32, count: usize, color: [f32; 3]) {
        for _ in 0..count.min(120) {
            let angle = self.random() * std::f32::consts::TAU;
            let spread = (0.2 + self.random() * 0.8) * strength;
            let rise = (0.3 + self.random() * 0.7) * strength;
            let size = match kind {
                Kind::Petal => 15.0,
                Kind::Leaf => 30.0,
                Kind::Snow => 12.0,
                Kind::Drop => 5.0,
                Kind::Spark => 3.0,
            } * (0.6 + self.random());
            self.particle(
                kind,
                position,
                Vec3::new(angle.cos() * spread, -rise, angle.sin() * spread),
                color,
                size,
            );
        }
    }
    pub fn clear_weather(&mut self) {
        self.particles
            .retain(|p| !matches!(p.kind, Kind::Snow | Kind::Drop));
        self.rings.clear();
        self.wet.clear();
    }
    pub fn tick(
        &mut self,
        game: &Game,
        surfaces: &Surfaces,
        graphics: &Graphics,
        weather: super::weather::SurfaceWeather,
        eye: Vec3,
    ) {
        if self.tick == Some(game.tick) {
            return;
        }
        if self.tick.is_some_and(|t| t + 1 != game.tick) {
            let canopies = std::mem::take(&mut self.canopies);
            *self = Self {
                canopies,
                ..Default::default()
            };
        }
        self.tick = Some(game.tick);
        self.advance(surfaces, graphics, eye);
        let options = &graphics.effects;
        let mut splash_slots = BTreeMap::new();
        let mut wire_slots = BTreeMap::new();
        for (slot, effect) in game
            .effects
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (i, e)))
        {
            match effect {
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
                        let first = self.particles.len();
                        self.burst(
                            Kind::Spark,
                            position,
                            110.0,
                            24 * usize::from(options.spark_density) / 100,
                            [1.0, 0.64, 0.2],
                        );
                        let rotation = glam::Quat::from_rotation_arc(Vec3::NEG_Y, normal);
                        for particle in &mut self.particles[first..] {
                            particle.velocity = rotation * particle.velocity;
                        }
                        if self.flashes.len() < 8 && options.spark_density > 0 {
                            self.flashes.push(Flash { position, age: 0.0 });
                        }
                    }
                }
                _ => {}
            }
        }
        self.splashes = splash_slots;
        self.wires = wire_slots;
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
    pub fn prewarm(&mut self, surfaces: &Surfaces, graphics: &Graphics, eye: Vec3, seconds: u16) {
        for _ in 0..u32::from(seconds) * 30 {
            self.advance(surfaces, graphics, eye);
        }
    }

    fn advance(&mut self, surfaces: &Surfaces, graphics: &Graphics, eye: Vec3) {
        let options = &graphics.effects;
        let wind_strength = if graphics.weather.mode.precipitates() {
            graphics.weather.wind
        } else {
            graphics.motion.wind
        };
        let heading = f32::from(graphics.weather.heading).to_radians();
        let wind = Vec3::new(heading.sin(), 0.0, heading.cos()) * f32::from(wind_strength) * 0.5;
        let mut wet_hits = Vec::new();
        for p in &mut self.particles {
            p.previous = p.position;
            p.age += 1.0 / 30.0;
            if p.ground.is_some() {
                continue;
            }
            let foliage = matches!(p.kind, Kind::Petal | Kind::Leaf);
            if foliage {
                p.velocity = p.velocity.lerp(wind + Vec3::Y * 12.0, 0.035);
                p.velocity += Vec3::new(
                    (p.age * 3.0 + p.phase).sin(),
                    0.0,
                    (p.age * 2.3 + p.phase).cos(),
                ) * 0.8;
            } else {
                p.velocity.y += if p.kind == Kind::Snow { 2.0 } else { 7.0 };
                if p.kind != Kind::Spark {
                    p.velocity += wind * 0.015;
                }
            }
            let next = p.position + p.velocity;
            if let Some(hit) = surfaces.cast_excluding(p.position, next, render_mesh::FOLIAGE) {
                p.position = hit.position + hit.normal * 2.0;
                if foliage && hit.normal.y < -0.3 {
                    p.ground = Some(hit.normal);
                    p.velocity = Vec3::ZERO;
                } else {
                    if p.kind == Kind::Drop
                        && hit.position.y <= p.origin.y + 80.0
                        && hit.position.distance_squared(p.origin) < 1800.0_f32.powi(2)
                    {
                        wet_hits.push(hit);
                    }
                    p.age = p.life;
                }
            } else {
                p.position = next;
            }
        }
        self.particles.retain(|p| {
            p.age < p.life
                && p.position.distance_squared(eye) < 100000.0_f32.powi(2)
                && match p.kind {
                    Kind::Petal | Kind::Leaf => options.leaves || options.scatter,
                    Kind::Drop => options.splashes || options.scatter,
                    Kind::Snow => options.scatter,
                    Kind::Spark => options.sparks,
                }
        });
        self.rings.iter_mut().for_each(|r| r.age += 1.0 / 30.0);
        self.rings
            .retain(|r| r.age < 1.2 && (options.splashes || options.scatter));
        self.wet.iter_mut().for_each(|m| m.age += 1.0 / 30.0);
        self.wet
            .retain(|m| m.age < 20.0 && (options.splashes || options.scatter));
        self.flashes.iter_mut().for_each(|f| f.age += 1.0 / 30.0);
        self.flashes.retain(|f| f.age < 0.35 && options.sparks);
        for hit in wet_hits.into_iter().take(4) {
            self.wet_mark(hit, surfaces);
        }
        if options.leaves && options.leaf_density > 0 {
            for index in 0..self.canopies.len() {
                let tree = self.canopies[index];
                if ((tree.min + tree.max) * 0.5).distance_squared(eye) > 60000.0_f32.powi(2)
                    || self.random() > f32::from(options.leaf_density) / 100.0 * 0.055
                {
                    continue;
                }
                let position = tree.min
                    + (tree.max - tree.min)
                        * Vec3::new(self.random(), 0.1 + self.random() * 0.6, self.random());
                self.particle(
                    if tree.blossom {
                        Kind::Petal
                    } else {
                        Kind::Leaf
                    },
                    position,
                    wind + Vec3::Y * 10.0,
                    tree.color,
                    if tree.blossom { 18.0 } else { 35.0 },
                );
            }
        }
    }

    fn ripple(&mut self, position: Vec3, strength: f32) {
        if self.rings.len() < 96 {
            self.rings.push(Ripple {
                position,
                age: 0.0,
                strength,
            });
        }
    }
    fn wet_mark(&mut self, hit: Hit, surfaces: &Surfaces) {
        if self.wet.len() >= 96 {
            return;
        }
        let mut vertices = surfaces.project(&Stamp {
            center: hit.position,
            normal: hit.normal,
            radius: 90.0,
            depth: 30.0,
            seed: self.seed,
            shape: Some((Vec3::Z, 0.8)),
        });
        if vertices.len() > 1024 {
            return;
        }
        for v in &mut vertices {
            v.flags = (v.flags & !render_mesh::STAIN) | render_mesh::WET_CONTACT;
            v.normal = hit.normal.to_array();
        }
        if !vertices.is_empty() {
            self.wet.push(WetMark { vertices, age: 0.0 });
        }
    }
    pub fn lights(&self) -> Vec<crate::lighting::Source> {
        self.flashes
            .iter()
            .map(|f| {
                let strength = (1.0 - f.age / 0.35).powi(2);
                crate::lighting::Source {
                    name: "Hook impact".into(),
                    light: crate::lighting::Light {
                        position: f.position,
                        color: [strength * 1.5, strength * 0.8, strength * 0.25],
                        radius: 3500.0,
                    },
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
