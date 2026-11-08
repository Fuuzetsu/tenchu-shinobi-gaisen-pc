use crate::{Result, scene::Scene, tenchu2::visibility::View};
use serde::{Deserialize, Serialize};

pub mod points;
mod spawn;
pub mod sprites;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Sprites(sprites::Sprites),
    Points(points::Points),
}

impl Effect {
    fn priority(&self) -> u8 {
        match self {
            Self::Sprites(_) => 6,
            Self::Points(_) => 5,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Effects {
    pub entries: Vec<Effect>,
}

#[derive(Default)]
pub struct Frame {
    pub sprites: Vec<sprites::Drawing>,
    pub points: Vec<points::Drawing>,
}

impl Effects {
    pub fn insert(&mut self, effect: Effect) -> Result<()> {
        if self.entries.len() >= 62 {
            let index = self
                .entries
                .iter()
                .position(|e| e.priority() >= 4)
                .ok_or("Tenchu 2 effect pool has no replaceable entry")?;
            self.entries.remove(index);
        }
        self.entries.push(effect);
        Ok(())
    }

    pub fn update(&mut self, scene: &mut Scene, view: View) -> Result<Frame> {
        let mut frame = Frame::default();
        let mut index = 0;
        while index < self.entries.len() {
            let alive = match &mut self.entries[index] {
                Effect::Sprites(sprites) => sprites.update(scene, view, &mut frame.sprites)?,
                Effect::Points(points) => points.update(scene, view, &mut frame.points)?,
            };
            if alive {
                index += 1;
            } else {
                self.entries.remove(index);
            }
        }
        Ok(frame)
    }
}

#[cfg(test)]
mod tests;
