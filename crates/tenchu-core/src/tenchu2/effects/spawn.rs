use super::{
    Effect, Effects,
    points::Points,
    sprites::{Frame, Motion, Sprites},
};
use crate::{
    Result,
    math::{Math, Position, Vector3},
    random::Random,
    tenchu2::rig::Rig,
};
use tenchu_assets::tenchu2::effects::Rules;

impl Effects {
    pub fn fuse(&mut self, position: Position, rules: &Rules, random: &mut Random) -> Result<()> {
        let controls = std::array::from_fn(|i| {
            Vector3::new(
                if i & 1 == 0 { 10 } else { -10 },
                -40 - (random.next15() % 20) as i16,
                if i < 2 { 10 } else { -10 },
            )
        });
        let mut points = Points::burst(position, controls, 2, rules);
        points.color = [128, 128, 96];
        points.blend = 1;
        self.insert(Effect::Points(points))
    }

    pub fn impact(
        &mut self,
        position: Position,
        direction: Position,
        math: &Math,
        rules: &Rules,
        random: &mut Random,
    ) -> Result<()> {
        let direction = math.normalize_vector(direction).map(|v| v as i16);
        let controls = std::array::from_fn(|i| {
            let side = if i & 1 == 0 { 2 } else { -2 };
            Vector3::new(
                ((i32::from(direction.x) + side * i32::from(direction.z)) >> 8) as i16,
                (if i < 2 { -40 } else { -20 }) - (random.next15() % 20) as i16,
                ((i32::from(direction.z) + side * i32::from(direction.x)) >> 8) as i16,
            )
        });
        let mut points = Points::burst(position, controls, 6, rules);
        points.color = [128, 128, 64];
        points.blend = 1;
        self.insert(Effect::Points(points))
    }

    pub fn trail(
        &mut self,
        position: Position,
        velocity: Vector3<i16>,
        kind: u8,
        hidden: bool,
        random: &mut Random,
    ) -> Result<()> {
        if kind == 0 && hidden {
            return Ok(());
        }
        let bubbles = kind > 1;
        let mut effect = Sprites::new(
            if bubbles {
                34
            } else if kind == 0 {
                19
            } else {
                29
            },
            position,
            if bubbles { 4 } else { 8 },
            16,
        );
        effect.velocity = if bubbles {
            Vector3::new(0, -64, 0)
        } else {
            Vector3::new(
                (-i32::from(velocity.x) >> 1) as i16,
                0,
                (-i32::from(velocity.z) >> 1) as i16,
            )
        };
        effect.size = if bubbles {
            [[64; 2], [256; 2]]
        } else {
            [[256; 2], [1024; 2]]
        };
        effect.color_step = [-8; 3].into();
        for (i, particle) in effect.particles.iter_mut().enumerate() {
            let mask = if bubbles { 255 } else { 511 };
            let center = (mask + 1) / 2;
            particle.offset = Vector3::new(
                (random.next15() & mask) as i16 - center as i16,
                if bubbles { (3 - i as i16) * -128 } else { 0 },
                (random.next15() & mask) as i16 - center as i16,
            );
            particle.age = -(i as i16) * if bubbles { 2 } else { 1 };
        }
        self.insert(Effect::Sprites(effect))
    }

    pub fn smoke(
        &mut self,
        position: Position,
        kind: u8,
        hidden: bool,
        math: &Math,
        random: &mut Random,
    ) -> Result<()> {
        if kind == 0 && hidden {
            return Ok(());
        }
        let (image, count, lifetime) = match kind {
            0 => (19, 8, 16),
            1 => (22, 8, 128),
            2 => (23, 8, 12),
            3 => (25, 1, 8),
            4 => (25, 1, 32),
            _ => return Err("Unknown Tenchu 2 smoke effect".into()),
        };
        let mut effect = Sprites::new(image, position, count, lifetime);
        match kind {
            0 => {
                effect.color_step = [-8; 3].into();
                let mut sizes = Vec::new();
                for particle in &mut effect.particles {
                    particle.offset = Vector3::new(
                        (random.next15() & 511) as i16 - 256,
                        0,
                        (random.next15() & 511) as i16 - 256,
                    );
                    sizes.push((random.next15() & 255) as i16 + 128);
                }
                effect.motion = Motion::Dust(sizes);
            }
            1 | 2 => {
                effect.motion = Motion::Smoke;
                effect.velocity.y = if kind == 1 { -40 } else { -100 };
                effect.size = [[512; 2], [if kind == 1 { 6144 } else { 2048 }; 2]];
                effect.color = [64; 3];
                effect.color_step = [if kind == 1 { -1 } else { -4 }; 3].into();
                effect.gravity = if kind == 1 { 2 } else { 0 };
                for (i, particle) in effect.particles.iter_mut().enumerate() {
                    particle.age = -(i as i16) * if kind == 1 { 8 } else { 4 };
                    particle.offset = if kind == 1 {
                        math.velocity(random.next15() as i16, 127, 0).into()
                    } else {
                        Vector3::new(
                            (random.next15() & 63) as i16 - 32,
                            0,
                            (random.next15() & 63) as i16 - 32,
                        )
                    };
                }
            }
            _ => {
                effect.velocity.y = -60;
                effect.size = [[512; 2], [if kind == 3 { 1024 } else { 1536 }; 2]];
                effect.color_step = [if kind == 3 { -16 } else { -4 }; 3].into();
            }
        }
        self.insert(Effect::Sprites(effect))
    }

    pub fn burn(&mut self, rig: &Rig, interval: i16, duration: i16, rules: &Rules) -> Result<()> {
        for (i, &bone) in rules.burn_bones.iter().enumerate() {
            let Some(&parent) = rig.joints.get(usize::from(bone)) else {
                continue;
            };
            let mut effect = Sprites::new(12, Position::ZERO, 1, duration);
            effect.parent = Some(parent);
            effect.frames = (12..=15)
                .map(|image| Frame { image, duration: 0 })
                .collect();
            effect.size = [[1024; 2]; 2];
            effect.depth_offset = -16;
            effect.particles[0].age = (i as i16 >> 1).wrapping_mul(interval);
            self.insert(Effect::Sprites(effect))?;
        }
        Ok(())
    }

    pub fn flash(&mut self, position: Position, weapon: bool) -> Result<()> {
        let mut effect = Sprites::new(if weapon { 18 } else { 16 }, position, 1, 4);
        effect.size = if weapon {
            [[1536; 2], [256; 2]]
        } else {
            [[2048; 2], [512; 2]]
        };
        self.insert(Effect::Sprites(effect))
    }

    pub fn explosion(
        &mut self,
        position: Position,
        kind: u8,
        math: &Math,
        random: &mut Random,
    ) -> Result<()> {
        let large = kind == 0;
        let mask = if large { 2047 } else { 1023 };
        let center = (mask + 1) / 2;
        for i in 0..if large { 4 } else { 1 } {
            let mut effect = Sprites::new(6, position, 6, 12);
            effect.size = [[3072; 2], [1024; 2]];
            effect.frames = [2, 2, 2, 3, 4]
                .into_iter()
                .enumerate()
                .map(|(i, duration)| Frame {
                    image: 6 + i as u8,
                    duration,
                })
                .collect();
            for (j, particle) in effect.particles.iter_mut().enumerate() {
                particle.age = -((j as i16 >> 1) + i * 4);
                particle.offset.y = -((random.next15() & mask) as i16);
                particle.offset.x = (random.next15() & mask) as i16 - center as i16;
                particle.offset.z = (random.next15() & mask) as i16 - center as i16;
            }
            self.insert(Effect::Sprites(effect))?;
        }
        let mut smoke = Sprites::new(25, position, if large { 8 } else { 4 }, 32);
        smoke.velocity.y = -60;
        smoke.size = [[1024; 2], [3072; 2]];
        smoke.color_step = [-4; 3].into();
        for (i, particle) in smoke.particles.iter_mut().enumerate() {
            particle.age = -4 - 2 * i as i16;
            particle.offset.y = -((random.next15() & 1023) as i16);
            particle.offset.x = (random.next15() & mask) as i16 - center as i16;
            particle.offset.z = (random.next15() & mask) as i16 - center as i16;
        }
        self.insert(Effect::Sprites(smoke))?;
        if large {
            let mut sparks =
                Sprites::new(24, position.wrapping_add(Position::new(0, -60, 0)), 8, 16);
            sparks.velocity.y = -20;
            sparks.size = [[6144; 2], [128; 2]];
            sparks.color_step = [-8; 3].into();
            sparks.motion = Motion::Embers;
            for (i, particle) in sparks.particles.iter_mut().enumerate() {
                particle.age = -4;
                particle.offset = math.velocity((i as i16) * 512, 511, 0).into();
            }
            self.insert(Effect::Sprites(sparks))?;
        }
        Ok(())
    }

    pub fn heal(&mut self, rig: &Rig, kind: u8, rules: &Rules, random: &mut Random) -> Result<()> {
        for &bone in &rules.healing_bones {
            let Some(&parent) = rig.joints.get(usize::from(bone)) else {
                continue;
            };
            let mut effect = Sprites::new(21, Position::ZERO, 2, 32);
            effect.parent = Some(parent);
            effect.size = [[2048, 512], [2048, 4096]];
            effect.color = [0; 3];
            effect.color_step = match kind {
                0 => [2, 0, 16],
                1 => [2, 16, 0],
                _ => [16, 2, 0],
            }
            .into();
            effect.particles[1].age = -16;
            effect.motion = Motion::Healing;
            self.insert(Effect::Sprites(effect))?;
        }
        let mut particles = std::array::from_fn(|_| super::points::Particle::default());
        for particle in &mut particles {
            particle.velocity.y = -20;
            particle.position.x = ((random.next15() & 127) + 256) as i16;
            if random.next15() & 1 != 0 {
                particle.position.x = -particle.position.x;
            }
            particle.position.y = (random.next15() & 511) as i16;
            particle.position.z = ((random.next15() & 127) + 256) as i16;
            if random.next15() & 1 != 0 {
                particle.position.z = -particle.position.z;
            }
        }
        self.insert(Effect::Points(Points {
            position: Position::ZERO,
            parent: rig.joints.first().copied(),
            particles,
            color: match kind {
                0 => [96, 96, 128],
                1 => [96, 128, 96],
                _ => [128, 96, 96],
            },
            color_step: Vector3::default(),
            age: 0,
            lifetime: 32,
            gravity: 0,
            blend: 1,
            size: Some([2; 2]),
        }))
    }
}
