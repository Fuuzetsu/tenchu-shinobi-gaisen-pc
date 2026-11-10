pub mod foliage;
mod gaisen;
mod geometry;
pub mod rope;
mod steps;

use super::surfaces::{Hit, Stamp, Surfaces};
use crate::{
    render_mesh::{self, Vertex},
    settings::Graphics,
};
use glam::Vec3;
use std::{collections::BTreeMap, sync::Arc};

const LIMIT: usize = 2048;
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Kind {
    Petal,
    Leaf,
    Snow,
    Drop,
    Spark,
}
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Ripple {
    position: Vec3,
    age: f32,
    strength: f32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct WetMark {
    vertices: Arc<Vec<Vertex>>,
    age: f32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Flash {
    position: Vec3,
    age: f32,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    tick: Option<u64>,
    seed: u32,
    particles: Vec<Particle>,
    rings: Vec<Ripple>,
    wet: Vec<WetMark>,
    flashes: Vec<Flash>,
    splashes: BTreeMap<usize, Vec3>,
    wires: BTreeMap<usize, u8>,
    clashes: BTreeMap<usize, u8>,
    steps: steps::Tracker,
    pub canopies: Vec<foliage::Canopy>,
}
fn point(p: tenchu_core::math::Position) -> Vec3 {
    p.map(|n| n as f32).to_array().into()
}
impl Contact {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn last_tick(&self) -> Option<u64> {
        self.tick
    }

    pub(crate) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            tick,
            seed,
            particles,
            rings,
            wet,
            flashes,
            splashes,
            wires,
            clashes,
            steps,
            canopies,
        } = self;
        out.value(&(tick, seed, particles, rings))?;
        out.value(&wet.len())?;
        for WetMark { vertices, age } in wet {
            out.shared_arc(vertices, true)?;
            out.value(age)?;
        }
        out.value(&(flashes, splashes, wires, clashes, steps, canopies))
    }

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
    pub fn begin_tick(
        &mut self,
        tick: u64,
        surfaces: &Surfaces,
        graphics: &Graphics,
        eye: Vec3,
    ) -> bool {
        if self.tick == Some(tick) {
            return false;
        }
        if self.tick.is_some_and(|t| t + 1 != tick) {
            self.clear();
        }
        self.tick = Some(tick);
        self.advance(surfaces, graphics, eye);
        true
    }

    pub fn spark(&mut self, position: Vec3, normal: Vec3, density: u16, color: [f32; 3]) {
        let first = self.particles.len();
        self.burst(
            Kind::Spark,
            position,
            110.0,
            24 * usize::from(density) / 100,
            color,
        );
        let rotation = glam::Quat::from_rotation_arc(Vec3::NEG_Y, normal);
        for particle in &mut self.particles[first..] {
            particle.velocity = rotation * particle.velocity;
        }
        if density > 0 {
            self.flash(position);
        }
    }

    pub fn splash(&mut self, position: Vec3, strength: f32) {
        self.burst(
            Kind::Drop,
            position - Vec3::Y * 8.0,
            65.0 * strength,
            12,
            [0.6, 0.76, 0.86],
        );
        self.ripple(position, strength);
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

    pub fn ripple(&mut self, position: Vec3, strength: f32) {
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
            self.wet.push(WetMark {
                vertices: Arc::new(vertices),
                age: 0.0,
            });
        }
    }
    fn flash(&mut self, position: Vec3) {
        if self.flashes.len() < 8
            && self.flashes.iter().all(|flash| {
                flash.age > 0.0 || flash.position.distance_squared(position) > 600.0_f32.powi(2)
            })
        {
            self.flashes.push(Flash { position, age: 0.0 });
        }
    }

    pub fn lights(&self) -> Vec<crate::lighting::Source> {
        self.flashes
            .iter()
            .map(|f| {
                let strength = (1.0 - f.age / 0.35).powi(2);
                crate::lighting::Source {
                    name: "Metal impact".into(),
                    light: crate::lighting::Light {
                        flame: false,
                        emission_radius: 0.0,
                        position: f.position,
                        color: [strength * 1.5, strength * 0.8, strength * 0.25],
                        radius: 3500.0,
                    },
                }
            })
            .collect()
    }

    pub fn spark_lights(&self, alpha: f32) -> impl Iterator<Item = crate::lighting::Light> + '_ {
        let alpha = alpha.clamp(0.0, 1.0);
        self.particles
            .iter()
            .filter(|p| p.kind == Kind::Spark)
            .map(move |p| {
                let strength = ((p.life - p.age) / (p.life * 0.5)).clamp(0.0, 1.0) * 0.1;
                crate::lighting::Light {
                    flame: false,
                    emission_radius: 0.0,
                    position: p.previous.lerp(p.position, alpha),
                    color: p.color.map(|c| c * strength),
                    radius: 2400.0,
                }
            })
    }
}

#[cfg(test)]
mod tests;
