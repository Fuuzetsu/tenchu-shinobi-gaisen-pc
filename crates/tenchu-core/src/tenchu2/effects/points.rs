use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::{ModelId, Scene},
    tenchu2::visibility::View,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Particle {
    pub position: Vector3<i16>,
    pub velocity: Vector3<i16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Points {
    #[serde(default)]
    pub sparks: bool,
    pub position: Position,
    pub parent: Option<ModelId>,
    pub particles: [Particle; 12],
    pub color: [u8; 3],
    pub color_step: Vector3<i16>,
    pub age: i16,
    pub lifetime: i16,
    pub gravity: i16,
    pub blend: u8,
    pub size: Option<[i16; 2]>,
}

#[derive(Clone, Debug)]
pub struct Drawing {
    pub sparks: bool,
    pub position: Position,
    pub color: [u8; 3],
    pub size: [i16; 2],
    pub blend: u8,
}

impl Points {
    pub fn burst(
        position: Position,
        controls: [Vector3<i16>; 4],
        gravity: i16,
        rules: &tenchu_assets::tenchu2::effects::Rules,
    ) -> Self {
        let particles = std::array::from_fn(|i| {
            let velocity = Vector3::from(std::array::from_fn(|axis| {
                (controls
                    .iter()
                    .zip(rules.point_weights[i])
                    .fold(0i32, |sum, (v, weight)| {
                        sum.wrapping_add(i32::from(v[axis]).wrapping_mul(weight))
                    })
                    >> 4) as i16
            }));
            Particle {
                position: velocity.map(|v| i32::from(v).wrapping_shl(rules.point_shifts[i]) as i16),
                velocity,
            }
        });
        Self {
            sparks: false,
            position,
            parent: None,
            particles,
            color: [128; 3],
            color_step: Vector3::default(),
            age: 0,
            lifetime: 24,
            gravity,
            blend: 0,
            size: None,
        }
    }

    pub fn update(
        &mut self,
        scene: &mut Scene,
        view: View,
        drawings: &mut Vec<Drawing>,
    ) -> Result<bool> {
        if self.age >= 0 {
            let color = std::array::from_fn(|axis| {
                (i32::from(self.color[axis])
                    + i32::from(self.color_step[axis]) * i32::from(self.age))
                .clamp(0, 255) as u8
            });
            let mut matrix = Matrix {
                translation: self.position,
                ..Matrix::default()
            };
            if let Some(parent) = self.parent {
                matrix = scene.world(parent)?.compose(&matrix);
            }
            let projection = view.matrix.compose(&matrix);
            for group in self.particles.chunks_exact_mut(3) {
                for particle in group.iter_mut() {
                    for axis in 0..3 {
                        particle.position[axis] =
                            particle.position[axis].wrapping_add(particle.velocity[axis]);
                    }
                }
                let depth =
                    crate::visibility::project(&projection, group[2].position.to_array()).1 >> 4;
                for particle in group {
                    let screen =
                        crate::visibility::project(&projection, particle.position.to_array()).0;
                    if self.size.is_some()
                        || (depth != 0
                            && i32::from(depth) <= view.distance >> 4
                            && i32::from(screen[0]).abs() <= 160
                            && i32::from(screen[1]).abs() <= 120)
                    {
                        drawings.push(Drawing {
                            sparks: self.sparks,
                            position: matrix.transform(particle.position.to_array()),
                            color,
                            size: self.size.unwrap_or([1; 2]),
                            blend: self.blend,
                        });
                    }
                    particle.velocity.y = particle.velocity.y.wrapping_add(self.gravity);
                }
            }
        }
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        Ok(previous < self.lifetime)
    }
}
