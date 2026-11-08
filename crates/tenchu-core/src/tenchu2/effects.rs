use crate::{
    Result,
    math::{Math, Position},
    scene::Scene,
    tenchu2::{terrain::Cursor, visibility::View},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

pub mod blood;
pub mod paper;
pub mod points;
pub mod ribbon;
mod spawn;
pub mod sprites;
pub mod surface;
pub mod water;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Effect {
    Sprites(sprites::Sprites),
    Points(points::Points),
    Blood(blood::Blood),
    Pool(blood::Pool),
    Ripple(water::Ripple),
    Ribbon(ribbon::Ribbon),
    Paper(paper::Paper),
}

impl Effect {
    fn priority(&self) -> u8 {
        match self {
            Self::Ribbon(_) => 0,
            Self::Sprites(_) => 6,
            Self::Points(_) => 5,
            Self::Blood(_) => 8,
            Self::Pool(_) => 9,
            Self::Ripple(_) => 12,
            Self::Paper(_) => 11,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Effects {
    pub entries: Vec<Effect>,
    pub blood_phase: bool,
    sprite_rotations: Vec<i32>,
}

#[derive(Default)]
pub struct Frame {
    pub sprites: Vec<sprites::Drawing>,
    pub points: Vec<points::Drawing>,
    pub quads: Vec<surface::Quad>,
    pub ribbons: Vec<ribbon::Drawing>,
    pub blood_sounds: Vec<Position>,
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

    pub fn update(
        &mut self,
        scene: &mut Scene,
        view: View,
        terrain: &Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Frame> {
        let mut frame = Frame::default();
        let mut index = 0;
        while index < self.entries.len() {
            let alive = match &mut self.entries[index] {
                Effect::Paper(paper) => paper.update(math, &mut frame.quads),
                Effect::Ribbon(ribbon) => ribbon.update(&mut frame.ribbons),
                Effect::Sprites(sprites) => {
                    sprites.update(scene, view, &mut self.sprite_rotations, &mut frame.sprites)?
                }
                Effect::Points(points) => points.update(scene, view, &mut frame.points)?,
                Effect::Blood(blood) => blood.update(scene, view, terrain, cursor, &mut frame)?,
                Effect::Pool(pool) => pool.update(&mut frame.quads)?,
                Effect::Ripple(ripple) => ripple.update(&mut frame.quads)?,
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
