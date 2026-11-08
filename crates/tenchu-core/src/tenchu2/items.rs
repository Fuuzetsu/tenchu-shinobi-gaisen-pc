pub mod aim;
pub mod flight;
pub mod grapple;
pub mod projectile;

use crate::{
    Result,
    math::{Math, Matrix, Position},
    scene::{ModelId, Scene},
    tenchu2::collision::Collision,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Items {
    pub aim: aim::Aim,
    pub held: Option<u8>,
    pub entries: Vec<Entry>,
    free_models: Vec<(ModelId, i32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub model: ModelId,
    pub state: State,
    unused_floor: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum State {
    Grapple(grapple::Grapple),
    Projectile(projectile::Projectile),
}

impl Items {
    pub fn grapple(&self) -> Option<&grapple::Grapple> {
        self.entries.iter().find_map(|e| match &e.state {
            State::Grapple(g) => Some(g),
            _ => None,
        })
    }

    pub fn grapple_mut(&mut self) -> Option<&mut grapple::Grapple> {
        self.entries.iter_mut().find_map(|e| match &mut e.state {
            State::Grapple(g) => Some(g),
            _ => None,
        })
    }

    pub fn insert(
        &mut self,
        mut state: State,
        scene: &mut Scene,
        collision: &mut Collision,
    ) -> Result<()> {
        if self.entries.len() == 30 {
            self.remove(0, collision);
        }
        let (model, floor) = self
            .free_models
            .pop()
            .unwrap_or_else(|| (scene.add(None, Matrix::default()), 0));
        if let State::Projectile(projectile) = &mut state {
            // The source allocator leaves the previous item's floor outside its cleared range.
            projectile.flight.floor = floor;
            scene.update(model, projectile.transform);
            projectile.register(model, collision)?;
        }
        self.entries.push(Entry {
            model,
            state,
            unused_floor: floor,
        });
        Ok(())
    }

    pub fn remove(&mut self, index: usize, collision: &mut Collision) {
        let entry = self.entries.remove(index);
        collision.remove(entry.model);
        let floor = match entry.state {
            State::Projectile(projectile) => projectile.flight.floor,
            State::Grapple(_) => entry.unused_floor,
        };
        self.free_models.push((entry.model, floor));
    }
}

fn facing(direction: Position, vertical: i32, math: &Math) -> [[i16; 3]; 3] {
    let up = if direction.x == 0 && direction.z == 0 {
        Position::new(0, 0, direction.y.wrapping_neg())
    } else {
        Position::new(0, vertical, 0)
    };
    let cross = |a: Position, b: Position| {
        Position::from_fn(|i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            a[j].wrapping_mul(b[k])
                .wrapping_sub(a[k].wrapping_mul(b[j]))
                >> 12
        })
    };
    let side = math.normalize_vector(cross(up, direction));
    let up = math.normalize_vector(cross(direction, side));
    std::array::from_fn(|i| {
        [
            side[i].wrapping_neg() as i16,
            up[i] as i16,
            direction[i].wrapping_neg() as i16,
        ]
    })
}
