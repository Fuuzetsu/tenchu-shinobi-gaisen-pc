use crate::{
    Result,
    math::Matrix,
    scene::{ModelId, Scene},
    visibility,
};

#[derive(Clone, Debug)]
pub struct Trail {
    pub parent: ModelId,
    pub vectors: [[i16; 3]; 2],
    pub points: [[[i16; 2]; 2]; 10],
    pub world_points: [[crate::math::Position; 2]; 10],
    pub count: usize,
    pub depth: u16,
}

impl Trail {
    pub fn new(parent: ModelId, vectors: [[i16; 3]; 2]) -> Self {
        Self {
            parent,
            vectors,
            points: [[[0; 2]; 2]; 10],
            world_points: [[crate::math::Position::ZERO; 2]; 10],
            count: 0,
            depth: 0,
        }
    }

    pub fn draw(&mut self, scene: &mut Scene, view: Matrix, display: bool) -> Result<u32> {
        if display {
            self.count = (self.count + 1).min(9);
            for i in (1..self.count).rev() {
                self.points[i] = self.points[i - 1];
                self.world_points[i] = self.world_points[i - 1];
            }
            let world = scene.world(self.parent)?;
            self.world_points[0] = self.vectors.map(|v| world.transform(v));
            let matrix = view.compose(&world);
            self.points[0] = self.vectors.map(|v| visibility::project(&matrix, v).0);
            self.depth = visibility::project(&matrix, [0; 3]).1 >> 2;
            if self.depth == 0 {
                return Ok(0);
            }
        } else if self.count == 0 {
            return Ok(0);
        } else {
            self.count -= 1;
        }
        Ok(self.count.saturating_sub(1) as u32 * 52)
    }
}
