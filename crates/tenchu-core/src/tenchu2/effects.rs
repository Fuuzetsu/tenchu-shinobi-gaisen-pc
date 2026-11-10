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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactKind {
    Metal,
    Water,
}

#[derive(Clone, Copy, Debug)]
pub enum Contact {
    Spark { position: Position, parried: bool },
    Splash { position: Position, size: i16 },
    Ripple { position: Position, speed: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emission {
    BurningActor,
    Explosion,
    Embers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Emitter {
    pub kind: Emission,
    pub position: Position,
    pub color: [u8; 3],
    pub size: [i16; 2],
    pub remaining: i32,
}

#[derive(Default)]
pub struct Frame {
    pub contacts: Vec<Contact>,
    pub emitters: Vec<Emitter>,
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
                    if sprites.particles.iter().any(|p| p.age == 0) {
                        match sprites.contact {
                            Some(ContactKind::Metal) => frame.contacts.push(Contact::Spark {
                                position: sprites.position,
                                parried: sprites.frames[0].image == 16,
                            }),
                            Some(ContactKind::Water) => frame.contacts.push(Contact::Splash {
                                position: sprites.position.wrapping_add(Position::new(0, 60, 0)),
                                size: sprites.size[0][0],
                            }),
                            None => {}
                        }
                    }
                    sprites.update(
                        scene,
                        view,
                        &mut self.sprite_rotations,
                        &mut frame.sprites,
                        &mut frame.emitters,
                    )?
                }
                Effect::Points(points) => {
                    if points.contact == Some(ContactKind::Metal) && points.age == 0 {
                        frame.contacts.push(Contact::Spark {
                            position: points.position,
                            parried: false,
                        });
                    }
                    points.update(scene, view, &mut frame.points)?
                }
                Effect::Blood(blood) => blood.update(scene, view, terrain, cursor, &mut frame)?,
                Effect::Pool(pool) => pool.update(&mut frame.quads)?,
                Effect::Ripple(ripple) => {
                    if ripple.age == 0 && ripple.parent.is_none() {
                        frame.contacts.push(Contact::Ripple {
                            position: ripple.position,
                            speed: ripple.speed,
                        });
                    }
                    ripple.update(&mut frame.quads)?
                }
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
