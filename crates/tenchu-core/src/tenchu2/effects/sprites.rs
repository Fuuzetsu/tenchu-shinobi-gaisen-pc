use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::{ModelId, Scene},
    tenchu2::visibility::View,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub image: u8,
    pub duration: i16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Particle {
    pub offset: Vector3<i16>,
    pub age: i16,
    pub frame: usize,
    pub clock: i16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Motion {
    #[default]
    Linear,
    Dust(Vec<i16>),
    Smoke,
    Dispersing,
    Embers,
    Healing,
    SleepGas,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sprites {
    pub position: Position,
    pub parent: Option<ModelId>,
    pub velocity: Vector3<i16>,
    pub color: [u8; 3],
    pub color_step: Vector3<i16>,
    pub size: [[i16; 2]; 2],
    pub frames: Vec<Frame>,
    pub particles: Vec<Particle>,
    pub lifetime: i16,
    pub spin: i16,
    pub gravity: i16,
    pub depth_offset: i32,
    pub motion: Motion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawing {
    pub image: u8,
    pub position: Position,
    pub size: [i16; 2],
    pub scale: [i16; 2],
    pub color: [u8; 3],
    pub rotation: i32,
    pub depth: i16,
    pub blend: super::surface::Blend,
}

impl Sprites {
    pub fn new(image: u8, position: Position, count: usize, lifetime: i16) -> Self {
        Self {
            position,
            parent: None,
            velocity: Vector3::default(),
            color: [128; 3],
            color_step: Vector3::default(),
            size: [[0; 2]; 2],
            frames: vec![Frame { image, duration: 0 }],
            particles: vec![Particle::default(); count],
            lifetime,
            spin: 0,
            gravity: 0,
            depth_offset: 0,
            motion: Motion::Linear,
        }
    }

    pub fn update(
        &mut self,
        scene: &mut Scene,
        view: View,
        rotations: &mut Vec<i32>,
        drawings: &mut Vec<Drawing>,
    ) -> Result<bool> {
        let parent = self.parent.map(|id| scene.world(id)).transpose()?;
        self.advance(
            |position| {
                let mut matrix = Matrix {
                    translation: position,
                    ..Matrix::default()
                };
                if let Some(parent) = parent {
                    matrix = parent.compose(&matrix);
                }
                (
                    matrix.translation,
                    i32::from(view.actor(&matrix).unwrap_or(0)),
                )
            },
            rotations,
            drawings,
        )
    }

    pub(crate) fn advance(
        &mut self,
        mut project: impl FnMut(Position) -> (Position, i32),
        rotations: &mut Vec<i32>,
        drawings: &mut Vec<Drawing>,
    ) -> Result<bool> {
        let linear = matches!(self.motion, Motion::Linear);
        let lifetime = i32::from(if linear && self.lifetime <= 0 {
            32
        } else {
            self.lifetime
        });
        for (index, particle) in self.particles.iter_mut().enumerate() {
            let previous_age = particle.age;
            if !linear || self.lifetime > 0 {
                particle.age = particle.age.wrapping_add(1);
            }
            let age = i32::from(particle.age);
            if (if linear { age < 0 } else { previous_age < 0 }) || age >= lifetime {
                continue;
            }
            if lifetime == 0 {
                return Err("Zero Tenchu 2 particle lifetime".into());
            }
            let fraction = (age << 12) / lifetime;
            let mut color = std::array::from_fn(|axis| {
                (i32::from(self.color[axis]) + i32::from(self.color_step[axis]) * age).clamp(0, 255)
                    as u8
            });
            let mut offset = particle.offset.map(i32::from);
            let mut size = std::array::from_fn(|axis| {
                interpolate(self.size[0][axis], self.size[1][axis], fraction, 12)
            });
            let mut depth_offset = self.depth_offset;
            match &self.motion {
                Motion::Linear => {}
                Motion::Dispersing => {
                    offset = offset.map(|v| (v * age) >> 2);
                    let fade = (i32::from(self.color_step.x) * age) >> 1;
                    color = self
                        .color
                        .map(|v| (i32::from(v) + fade).clamp(0, 255) as u8);
                }
                Motion::SleepGas => {
                    offset.x = (offset.x * age) >> 2;
                    offset.z = (offset.z * age) >> 2;
                    let value = i32::from(self.color[0]) + i32::from(self.color_step.x) * age;
                    color = [
                        value.clamp(0, 255) as u8,
                        (value - 8).clamp(0, 255) as u8,
                        (value - 8).clamp(0, 255) as u8,
                    ];
                }
                Motion::Dust(sizes) => {
                    offset.x += (offset.x * age) >> 3;
                    offset.z += (offset.z * age) >> 3;
                    let initial = *sizes.get(index).ok_or("Missing Tenchu 2 dust size")?;
                    size =
                        [initial.wrapping_add(((i32::from(initial) * fraction) >> 12) as i16); 2];
                    color = [color[0]; 3];
                    depth_offset = 0;
                }
                Motion::Smoke | Motion::Embers => {
                    let base = if matches!(self.motion, Motion::Embers) {
                        4
                    } else {
                        0
                    };
                    offset.x = offset.x * base + ((offset.x * age) >> 2);
                    offset.z = offset.z * base + ((offset.z * age) >> 2);
                    particle.offset.y = particle
                        .offset
                        .y
                        .wrapping_add(self.velocity.y)
                        .wrapping_add(particle.age);
                    let fade = i32::from(self.color_step.x) * age;
                    color = [(i32::from(self.color[0]) + if base == 0 { fade >> 1 } else { fade })
                        .clamp(0, 255) as u8; 3];
                    if base != 0 && age < lifetime >> 3 {
                        size = std::array::from_fn(|axis| {
                            interpolate(self.size[1][axis], self.size[0][axis], fraction, 9)
                        });
                    }
                }
                Motion::Healing => {
                    let value = if age < lifetime >> 1 {
                        i32::from(self.color_step.x) * age
                    } else {
                        32 - i32::from(self.color_step.x) * (age - (lifetime >> 1))
                    };
                    color = [
                        value as u8,
                        (value + i32::from(self.color_step.y)) as u8,
                        (value + i32::from(self.color_step.z)) as u8,
                    ];
                    depth_offset = -16;
                }
            }
            let (position, depth) = project(self.position.wrapping_add(offset));
            let depth = depth.wrapping_add(depth_offset);
            if depth <= 0 {
                continue;
            }
            let frame = self
                .frames
                .get(particle.frame)
                .ok_or("Missing Tenchu 2 particle frame")?;
            let scale = (4800 / depth.wrapping_mul(4)) as i16;
            let image = usize::from(frame.image);
            if rotations.len() <= image {
                rotations.resize(image + 1, 0);
            }
            if !matches!(self.motion, Motion::SleepGas) {
                rotations[image] = if matches!(self.motion, Motion::Dust(_) | Motion::Healing) {
                    0
                } else {
                    ((i32::from(
                        if matches!(
                            self.motion,
                            Motion::Smoke | Motion::Embers | Motion::Dispersing
                        ) {
                            self.gravity
                        } else {
                            self.spin
                        },
                    ) * age)
                        % 360)
                        * 4096
                };
            }
            drawings.push(Drawing {
                image: frame.image,
                position,
                size,
                scale: size.map(|s| s.wrapping_mul(scale)),
                color,
                rotation: rotations[image],
                depth: depth as i16,
                blend: super::surface::Blend::Texture,
            });
            if !matches!(self.motion, Motion::Dust(_) | Motion::SleepGas) {
                let clock = particle.clock;
                particle.clock = clock.wrapping_add(1);
                if clock >= frame.duration {
                    particle.clock = 0;
                    particle.frame = (particle.frame + 1) % self.frames.len();
                }
            }
        }
        if linear || matches!(self.motion, Motion::Dispersing) {
            self.position = self.position.wrapping_add(self.velocity.map(i32::from));
            if linear {
                self.velocity.y = self.velocity.y.wrapping_add(self.gravity);
            }
        }
        Ok(self.lifetime < 0 || self.particles.last().is_some_and(|p| p.age < self.lifetime))
    }
}

fn interpolate(start: i16, end: i16, fraction: i32, shift: u8) -> i16 {
    start.wrapping_add((((i32::from(end) - i32::from(start)) * fraction) >> shift) as i16)
}
