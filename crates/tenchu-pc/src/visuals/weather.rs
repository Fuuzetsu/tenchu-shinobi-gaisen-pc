use super::surfaces::{Hit, Surfaces};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
    settings::{Weather as Settings, WeatherMode},
};
use bytemuck::Zeroable;
use glam::{Mat4, Vec3};

const RADIUS: f32 = 10000.0;
const HEIGHT: f32 = 8000.0;
const RUNOFF_LIMIT: usize = 192;
const EAVE_LIMIT: usize = 96;

mod runoff;
const SPLASH_LIMIT: usize = 192;

struct Particle {
    previous: Vec3,
    position: Vec3,
    velocity: Vec3,
    hit: Option<Hit>,
    life: u16,
    size: f32,
}

struct Splash {
    hit: Hit,
    age: u16,
}

#[derive(Default)]
pub struct Weather {
    mode: WeatherMode,
    tick: Option<u64>,
    center: Option<Vec3>,
    direction: Option<Vec3>,
    particles: Vec<Particle>,
    runoff: Vec<Particle>,
    eaves: Vec<runoff::Eave>,
    splashes: Vec<Splash>,
    wet: f32,
    puddles: f32,
    snow: f32,
    seed: u32,
}

#[derive(Clone, Copy, Default)]
pub struct SurfaceWeather {
    pub wet: f32,
    pub puddles: f32,
    pub snow: f32,
    pub time: f32,
}

impl Weather {
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / 16777216.0
    }

    pub fn reset_particles(&mut self, mode: WeatherMode) {
        self.mode = mode;
        self.particles.clear();
        self.runoff.clear();
        self.eaves.clear();
        self.splashes.clear();
    }

    pub fn invalidate_surfaces(&mut self) {
        self.reset_particles(self.mode);
    }

    pub fn sample(&self, settings: &Settings, alpha: f32) -> SurfaceWeather {
        if settings.mode == WeatherMode::Original {
            return SurfaceWeather::default();
        }
        SurfaceWeather {
            wet: self.wet * f32::from(settings.wetness) / 100.0,
            puddles: self.puddles * f32::from(settings.puddles) / 100.0,
            snow: self.snow * f32::from(settings.snow) / 100.0,
            time: ((self.tick.unwrap_or(0) % 36000) as f32 + alpha.clamp(0.0, 1.0)) / 30.0,
        }
    }

    pub fn tick(&mut self, tick: u64, center: Vec3, surfaces: &Surfaces, settings: &Settings) {
        if settings.mode == WeatherMode::Original {
            *self = Self::default();
            return;
        }
        if self.tick == Some(tick) {
            return;
        }
        if self.tick.is_some_and(|old| old + 1 != tick) {
            *self = Self::default();
        }
        self.tick = Some(tick);
        if self.mode != settings.mode
            || self
                .center
                .is_some_and(|old| old.distance_squared(center) > RADIUS * RADIUS)
        {
            self.reset_particles(settings.mode);
        }
        self.center = Some(center);
        let intensity = f32::from(settings.amount) / 100.0;
        self.wet = (self.wet
            + if self.mode == WeatherMode::Rain {
                intensity * f32::from(settings.pooling) / 60000.0
            } else {
                -1.0 / 3600.0
            })
        .clamp(0.0, 1.0);
        self.puddles = (self.puddles
            + if self.mode == WeatherMode::Rain {
                intensity * self.wet * f32::from(settings.pooling) / 180000.0
            } else {
                -1.0 / 5400.0
            })
        .clamp(0.0, 1.0);
        self.snow = (self.snow
            + if self.mode == WeatherMode::Snow {
                intensity / 900.0
            } else {
                -1.0 / 1800.0
            })
        .clamp(0.0, 1.0);
        self.splashes.iter_mut().for_each(|s| s.age += 1);
        self.splashes.retain(|s| s.age < 14);
        let direction = settings.direction();
        let wind_changed = self.direction != Some(direction);
        self.direction = Some(direction);
        let mut landings = Vec::new();
        self.particles.retain_mut(|p| {
            if wind_changed {
                p.velocity = settings.velocity(p.velocity.y);
                if !surfaces.exposed(p.position, p.velocity) {
                    return false;
                }
                p.hit = surfaces.cast(p.position, p.position + p.velocity * f32::from(p.life));
            }
            p.previous = p.position;
            p.position += p.velocity;
            p.life = p.life.saturating_sub(1);
            if let Some(hit) = p.hit
                && (hit.position - p.position).dot(p.velocity) <= 0.0
            {
                landings.push(hit);
                return false;
            }
            p.life != 0
                && (p.position - center).abs().max_element() < f32::from(settings.distance) * 1500.0
        });
        self.update_runoff(center, surfaces, settings, &mut landings);
        if self.mode == WeatherMode::Rain {
            for hit in landings {
                if self.splashes.len() < SPLASH_LIMIT {
                    self.splashes.push(Splash { hit, age: 0 });
                }
            }
        }
        if !self.mode.precipitates() {
            return;
        }
        self.particles.truncate(settings.particles());
        let missing = settings.particles().saturating_sub(self.particles.len());
        for _ in 0..missing.min(192) {
            self.spawn(center, surfaces, settings);
        }
    }

    fn spawn(&mut self, center: Vec3, surfaces: &Surfaces, settings: &Settings) {
        let distant = self.particles.len() % 3 == 2;
        let radius = if distant {
            f32::from(settings.distance) * 1000.0
        } else {
            RADIUS
        };
        let position = center
            + Vec3::new(
                (self.random() * 2.0 - 1.0) * radius,
                -self.random() * HEIGHT - 100.0,
                (self.random() * 2.0 - 1.0) * radius,
            );
        let snow = self.mode == WeatherMode::Snow;
        let velocity = settings.velocity(if snow {
            24.0 + self.random() * 24.0
        } else {
            330.0 + self.random() * 100.0
        });
        if !surfaces.exposed(position, velocity) {
            return;
        }
        let life = if snow { 450 } else { 70 };
        let hit = surfaces.cast(position, position + velocity * f32::from(life));
        let size = if snow {
            9.0 + self.random() * 11.0
        } else {
            3.5 + self.random() * 2.5
        } * if distant {
            (radius / RADIUS).sqrt()
        } else {
            1.0
        };
        self.particles.push(Particle {
            previous: position,
            position,
            velocity,
            hit,
            life,
            size,
        });
    }

    pub fn geometry(&self, camera: Mat4, alpha: f32, settings: &Settings) -> Geometry {
        let mut vertices = Vec::new();
        if settings.mode != self.mode || !self.mode.precipitates() {
            return Geometry {
                vertices,
                batches: Vec::new(),
            };
        }
        let inverse = camera.inverse();
        let near = inverse.project_point3(Vec3::ZERO);
        let forward = (inverse.project_point3(Vec3::Z) - near).normalize_or_zero();
        let right = forward.cross(Vec3::NEG_Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let alpha = alpha.clamp(0.0, 1.0);
        let mut particles: Vec<_> = self
            .particles
            .iter()
            .map(|p| (p, false))
            .chain(self.runoff.iter().map(|p| (p, true)))
            .collect();
        particles.sort_unstable_by(|(a, _), (b, _)| {
            forward
                .dot(b.position - near)
                .total_cmp(&forward.dot(a.position - near))
        });
        for (p, runoff) in particles {
            if runoff && (!settings.runoff || settings.runoff_amount == 0) {
                continue;
            }
            if !runoff && self.mode == WeatherMode::Rain && settings.streaks == 0 {
                continue;
            }
            let position = p.previous.lerp(p.position, alpha);
            let snow = self.mode == WeatherMode::Snow;
            let axis = if snow {
                up
            } else {
                p.velocity.normalize_or_zero()
            };
            let side = if snow {
                right
            } else {
                axis.cross(forward).normalize_or_zero()
            };
            let length = if snow {
                p.size
            } else {
                (p.velocity.length() * if runoff { 1.8 } else { 0.8 }).min(650.0)
            };
            let distance = position.distance(near);
            let limit = f32::from(settings.distance) * 1000.0;
            let fade = ((forward.dot(position - near) - 60.0) / 500.0).clamp(0.0, 1.0)
                * ((limit - distance) / (limit * 0.2)).clamp(0.0, 1.0);
            let strength = if snow || runoff {
                1.0
            } else {
                f32::from(settings.streaks) / 100.0
            };
            let width = if snow {
                p.size
            } else {
                p.size.max((distance * 0.0005).min(60.0)) * strength.sqrt()
            };
            quad(
                &mut vertices,
                if snow {
                    position
                } else {
                    position - axis * length * 0.5
                },
                side * width,
                axis * length * if snow { 1.0 } else { 0.5 },
                if snow {
                    [0.8, 0.84, 0.9]
                } else {
                    [0.65, 0.75, 0.86]
                },
                fade * if snow {
                    0.65
                } else if runoff {
                    0.55
                } else {
                    0.34 * strength
                },
            );
        }
        for splash in &self.splashes {
            let normal = splash.hit.normal;
            let tangent = normal.any_orthonormal_vector();
            let age = (f32::from(splash.age) + alpha) / 14.0;
            let radius = 8.0 + age * 90.0;
            for segment in 0..8 {
                let angle = segment as f32 * std::f32::consts::TAU / 8.0;
                let ray = tangent * angle.cos() + normal.cross(tangent) * angle.sin();
                let center = splash.hit.position
                    + normal * (8.0 + (age * std::f32::consts::PI).sin() * 18.0)
                    + ray * radius;
                quad(
                    &mut vertices,
                    center,
                    ray * 12.0,
                    normal.cross(ray) * 3.0,
                    [0.55, 0.65, 0.78],
                    (1.0 - age) * 0.3 * f32::from(settings.streaks) / 100.0,
                );
            }
        }
        let count = vertices.len() as u32;
        Geometry {
            vertices,
            batches: vec![Batch {
                blend: 5,
                range: 0..count,
            }],
        }
    }
}

pub(super) fn quad(
    vertices: &mut Vec<Vertex>,
    center: Vec3,
    right: Vec3,
    up: Vec3,
    color: [f32; 3],
    opacity: f32,
) {
    for (x, y) in [
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ] {
        vertices.push(Vertex {
            position: (center + right * x + up * y).to_array(),
            color,
            uv: [x, y],
            opacity,
            flags: render_mesh::ALPHA
                | render_mesh::DOUBLE_SIDED
                | render_mesh::FOG
                | render_mesh::SOFT,
            ..Vertex::zeroed()
        });
    }
}

#[cfg(test)]
mod tests;
