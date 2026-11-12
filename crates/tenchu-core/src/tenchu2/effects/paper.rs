use super::surface::{Blend, Image, Quad};
use crate::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Motion {
    Outward,
    Rising,
    Burst,
    Scatter,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Particle {
    pub offset: Vector3<i16>,
    pub radius: i16,
    pub rotation: Vector3<i16>,
    pub velocity: Vector3<i16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paper {
    pub position: Position,
    pub particles: [Particle; 8],
    pub age: i32,
    pub lifetime: i32,
    pub motion: Motion,
}

impl Paper {
    pub fn new(position: Position, motion: Motion, math: &Math, random: &mut Random) -> Self {
        let particles = std::array::from_fn(|_| {
            let mut p = Particle::default();
            match motion {
                Motion::Outward | Motion::Rising => {
                    if motion == Motion::Outward {
                        p.offset.x = (random.next15() & 2047) as i16 - 1024;
                        p.offset.z = (random.next15() & 2047) as i16 - 1024;
                    }
                    p.offset.y = 512 - (random.next15() & 2047) as i16;
                    p.rotation = std::array::from_fn(|_| (random.next15() & 4095) as i16).into();
                    p.radius = (i32::from(p.offset.y)
                        .wrapping_sub(position.y)
                        .wrapping_sub(2048)
                        >> 3) as i16;
                }
                Motion::Burst | Motion::Scatter => {
                    p.velocity.y = -48 - (random.next15() & 63) as i16;
                    p.rotation.y = (random.next15() & 4095) as i16;
                    if motion == Motion::Burst {
                        // The original overwrites this angle with a second RNG call.
                        p.rotation.y = (random.next15() & 4095) as i16;
                        p.velocity.x = ((random.next15() & 63) as i16 - p.velocity.y) >> 3;
                    } else {
                        let distance = ((random.next15() & 31) as i16 - p.velocity.y) >> 3;
                        let [x, _, z] = math.velocity(p.rotation.y, distance, 0);
                        p.velocity.x = x;
                        p.velocity.z = z;
                    }
                    p.rotation.x = (random.next15() & 4095) as i16;
                    p.rotation.z = (random.next15() & 4095) as i16;
                }
            }
            p
        });
        let (age, lifetime) = match motion {
            Motion::Outward => (0, 30),
            Motion::Rising => (0, 120),
            Motion::Burst => (-30, 36),
            Motion::Scatter => (-30, 80),
        };
        Self {
            position,
            particles,
            age,
            lifetime,
            motion,
        }
    }

    pub fn update(&mut self, math: &Math, drawings: &mut Vec<Quad>) -> bool {
        self.move_particles(math);
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        if self.age >= self.lifetime {
            return false;
        }
        if previous >= 0 {
            draw(
                math,
                self.position,
                &self.particles,
                Image::Item(64),
                [64, 96, 64],
                drawings,
            );
        }
        true
    }

    fn move_particles(&mut self, math: &Math) {
        if self.age < 0 && matches!(self.motion, Motion::Burst | Motion::Scatter) {
            return;
        }
        for p in &mut self.particles {
            p.rotation.x = p.rotation.x.wrapping_add(341);
            p.rotation.z = p.rotation.z.wrapping_add(113);
            match self.motion {
                Motion::Outward => {
                    p.offset.y = p.offset.y.wrapping_add(40);
                    p.rotation.y = p
                        .rotation
                        .y
                        .wrapping_add(240i16.wrapping_sub((self.age as i16).wrapping_mul(4)));
                }
                Motion::Rising => {
                    p.rotation.y = p
                        .rotation
                        .y
                        .wrapping_add((self.age as i16).wrapping_add(10).wrapping_mul(11));
                    if p.velocity.y > 0 {
                        p.offset.y = p.offset.y.wrapping_add(p.velocity.y);
                        p.velocity.y = p.velocity.y.wrapping_sub(10);
                    }
                }
                Motion::Burst => {
                    p.rotation.y = p
                        .rotation
                        .y
                        .wrapping_add(((self.age >> 2) as i16).wrapping_mul(11));
                    p.radius = p.radius.wrapping_add(p.velocity.x);
                    p.offset.y = p.offset.y.wrapping_add(p.velocity.y);
                    p.velocity.y = p.velocity.y.wrapping_add(5);
                }
                Motion::Scatter => {
                    p.offset.x = p.offset.x.wrapping_add(p.velocity.x);
                    p.offset.y = p.offset.y.wrapping_add(p.velocity.y);
                    p.offset.z = p.offset.z.wrapping_add(p.velocity.z);
                    p.velocity.y = p.velocity.y.wrapping_add(5);
                }
            }
            if self.motion != Motion::Scatter {
                let [x, _, z] = math.velocity(p.rotation.y, p.radius, 0);
                p.offset.x = x;
                p.offset.z = z;
            }
        }
        if self.age >= self.lifetime.wrapping_sub(1) {
            match self.motion {
                Motion::Rising => {
                    self.motion = Motion::Outward;
                    self.age = 0;
                    self.lifetime = 60;
                }
                Motion::Burst => {
                    self.motion = Motion::Rising;
                    self.age = 0;
                    self.lifetime = 15;
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Leaves {
    pub position: Position,
    pub particles: [Particle; 8],
    pub age: i32,
    pub lifetime: i32,
}

impl Leaves {
    pub fn new(position: Position, wind: Position, random: &mut Random) -> Self {
        let particles = std::array::from_fn(|_| {
            let offset = Vector3::new(
                (random.next15() & 16383) as i16 - 8192,
                -((random.next15() & 8191) as i16),
                (random.next15() & 16383) as i16 - 8192,
            );
            let yaw = (random.next15() & 4095) as i16;
            let x = ((wind.x >> 3) + (random.next15() & 127) - 64) as i16;
            let z = ((wind.z >> 3) + (random.next15() & 127) - 64) as i16;
            let y = ((wind.y >> 3) + (random.next15() & 63)) as i16;
            Particle {
                offset,
                radius: 0,
                rotation: Vector3::new(
                    (random.next15() & 4095) as i16,
                    yaw,
                    (random.next15() & 4095) as i16,
                ),
                velocity: Vector3::new(x, y, z),
            }
        });
        Self {
            position,
            particles,
            age: -8,
            lifetime: 120,
        }
    }

    pub fn update(&mut self, math: &Math, drawings: &mut Vec<Quad>) -> bool {
        let previous = self.age;
        if previous >= 0 {
            for p in &mut self.particles {
                p.rotation.x = p
                    .rotation
                    .x
                    .wrapping_add((i32::from(p.velocity.x).abs() * 4) as i16);
                p.rotation.z = p.rotation.z.wrapping_add(113);
                for axis in 0..3 {
                    p.offset[axis] = p.offset[axis].wrapping_add(p.velocity[axis] >> 2);
                }
                p.velocity.x = p.velocity.x.wrapping_add(p.velocity.x >> 7);
                p.velocity.z = p.velocity.z.wrapping_add(p.velocity.z >> 7);
            }
        }
        self.age = self.age.wrapping_add(1);
        if self.age >= self.lifetime {
            return false;
        }
        if previous >= 0 {
            draw(
                math,
                self.position,
                &self.particles,
                Image::Effect(35),
                [128; 3],
                drawings,
            );
        }
        true
    }
}

fn draw(
    math: &Math,
    position: Position,
    particles: &[Particle; 8],
    image: Image,
    color: [u8; 3],
    drawings: &mut Vec<Quad>,
) {
    for particle in particles {
        let transform = Matrix {
            rotation: math.rotate_yxz(particle.rotation.to_array()),
            ..Default::default()
        };
        drawings.push(Quad {
            blood: None,
            image,
            position: position.wrapping_add(particle.offset.map(i32::from)),
            parent: None,
            corners: [[-80, -80, 0], [80, -80, 0], [-80, 80, 0], [80, 80, 0]]
                .map(|p| transform.transform(p).map(|v| v as i16)),
            color,
            blend: Blend::Add,
        });
    }
}
