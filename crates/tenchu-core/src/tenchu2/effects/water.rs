use super::{
    Effect, Effects,
    points::{Particle, Points},
    sprites::{Motion, Sprites},
    surface::{Blend, Image, Plane, Quad},
};
use crate::{
    Result,
    math::{Math, Position, Vector3},
    random::Random,
    scene::ModelId,
};
use serde::{Deserialize, Serialize};

pub const RIPPLE_IMAGE: u8 = 17;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ripple {
    pub position: Position,
    pub parent: Option<ModelId>,
    pub age: i16,
    pub lifetime: i16,
    pub speed: i32,
    pub plane: Plane,
}

impl Ripple {
    pub fn update(&mut self, drawings: &mut Vec<Quad>) -> Result<bool> {
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        if self.age >= self.lifetime {
            return Ok(false);
        }
        if previous >= 0 {
            let radius = self.speed.wrapping_mul(i32::from(self.age));
            for [x, z] in [[-1, -1], [1, -1], [1, 1], [-1, 1]] {
                let x = radius.wrapping_mul(x);
                let z = radius.wrapping_mul(z);
                drawings.push(Quad {
                    image: Image::Effect(RIPPLE_IMAGE),
                    position: self.position,
                    parent: self.parent,
                    corners: [
                        self.plane.point(x, z)?,
                        self.plane.point(0, z)?,
                        self.plane.point(x, 0)?,
                        Vector3::default(),
                    ],
                    color: [(64 - i32::from(self.age) * 2) as u8; 3],
                    blend: Blend::Add,
                });
            }
        }
        Ok(true)
    }
}

impl Effects {
    pub fn ripple(&mut self, position: Position, speed: i32) -> Result<()> {
        self.insert(Effect::Ripple(Ripple {
            position,
            parent: None,
            age: 0,
            lifetime: 32,
            speed,
            plane: Plane::flat(),
        }))
    }

    pub fn splash(
        &mut self,
        position: Position,
        size: i32,
        math: &Math,
        random: &mut Random,
    ) -> Result<()> {
        let mut effect = Sprites::new(29, position.wrapping_add(Position::new(0, -60, 0)), 8, 16);
        effect.contact = Some(super::ContactKind::Water);
        effect.velocity.y = -20;
        effect.size = [[size as i16; 2], [(size >> 4) as i16; 2]];
        effect.color_step = [-8; 3].into();
        effect.motion = Motion::Embers;
        for (i, particle) in effect.particles.iter_mut().enumerate() {
            particle.offset = math.velocity(i as i16 * 512, (size >> 5) as i16, 0).into();
        }
        self.insert(Effect::Sprites(effect))?;
        for i in 0..4 {
            let particles: [Particle; 12] = std::array::from_fn(|_| {
                let mut position = Vector3::new((random.next15() & 63) as i16 + 64, 0, 0);
                if random.next15() & 1 != 0 {
                    position.x = -position.x;
                }
                position.y = (random.next15() & 511) as i16;
                position.z = (random.next15() & 63) as i16 + 64;
                if random.next15() & 1 != 0 {
                    position.z = -position.z;
                }
                Particle {
                    velocity: Vector3::new(position.x >> 2, -80, position.z >> 2),
                    position,
                }
            });
            let ripple = position.wrapping_add(Position::new(
                i32::from(particles[0].position.x) * 8,
                0,
                i32::from(particles[0].position.z) * 8,
            ));
            self.insert(Effect::Points(Points {
                contact: Some(super::ContactKind::Water),
                position: position.wrapping_sub(Position::new(0, size >> 2, 0)),
                parent: None,
                particles,
                color: [127, 127, 143],
                color_step: Vector3::default(),
                age: 0,
                lifetime: 20,
                gravity: 8,
                blend: 1,
                size: None,
            }))?;
            self.insert(Effect::Ripple(Ripple {
                position: ripple,
                parent: None,
                age: -15 - i * 4,
                lifetime: 30,
                speed: 20,
                plane: Plane::flat(),
            }))?;
        }
        Ok(())
    }
}
