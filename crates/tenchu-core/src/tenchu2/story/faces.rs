use crate::random::Random;
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::story::{Expression, Resources};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Face {
    pub model: i16,
    pub slot: usize,
    pub held: [u8; 2],
    pub cycle: [Option<u8>; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Faces {
    pub actors: Vec<Face>,
    next: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureChange {
    Patch { model: i16, index: u8 },
    Ignite { slot: usize },
}

impl Faces {
    pub fn new(resources: &Resources) -> Self {
        let mut actors: Vec<Face> = Vec::new();
        for (slot, animation) in resources.script.actors.iter().enumerate() {
            if resources.faces.contains_key(&animation.model)
                && !actors.iter().any(|a| a.model == animation.model)
            {
                actors.push(Face {
                    model: animation.model,
                    slot,
                    held: [0; 2],
                    cycle: [Some(0), None],
                });
            }
        }
        Self { actors, next: 0 }
    }

    pub fn update(
        &mut self,
        expressions: &[Expression],
        frame: i16,
        draw: bool,
        random: &mut Random,
    ) -> Vec<TextureChange> {
        let mut changes = Vec::new();
        while let Some(expression) = expressions.get(self.next)
            && i32::from(expression.frame) <= i32::from(frame) + 1
        {
            self.next += 1;
            if let Some(face) = self.actors.iter_mut().find(|a| a.model == expression.model) {
                for (channel, &value) in expression.channels.iter().enumerate() {
                    match value {
                        13 => {
                            face.cycle[channel].get_or_insert(0);
                        }
                        14 => {
                            face.cycle[channel] = None;
                            changes.push(face.patch(channel, face.held[channel]));
                        }
                        12 => changes.push(TextureChange::Ignite { slot: face.slot }),
                        255 => (),
                        _ => {
                            changes.push(face.patch(channel, value));
                            face.held[channel] = value;
                        }
                    }
                }
            }
        }
        if draw {
            let mut uploads = changes
                .iter()
                .filter(|c| matches!(c, TextureChange::Patch { .. }))
                .count();
            for face in &mut self.actors {
                let before = changes.len();
                face.animate(frame, uploads, random, &mut changes);
                uploads += changes.len() - before;
            }
        }
        changes
    }
}

impl Face {
    fn patch(&self, channel: usize, index: u8) -> TextureChange {
        TextureChange::Patch {
            model: self.model,
            index: index + channel as u8 * 16,
        }
    }

    fn animate(
        &mut self,
        frame: i16,
        uploads: usize,
        random: &mut Random,
        changes: &mut Vec<TextureChange>,
    ) {
        if let Some(cycle) = self.cycle[0]
            && self.held[0] & 15 != 8
        {
            self.cycle[0] = Some(match cycle {
                0 if ((i32::from(frame) >> 3) & 15) as usize == self.slot
                    && random.next15() & 7 == 0 =>
                {
                    if uploads < 8 {
                        changes.push(self.patch(0, 7));
                    }
                    1
                }
                0 => 0,
                1 => {
                    changes.push(self.patch(0, 8));
                    2
                }
                2 => {
                    changes.push(self.patch(0, 7));
                    3
                }
                _ => {
                    changes.push(self.patch(0, self.held[0] & 15));
                    0
                }
            });
        }
        if let Some(cycle) = self.cycle[1] {
            self.cycle[1] = Some(match cycle {
                0 if random.next15() & 1 == 0 => {
                    changes.push(self.patch(1, 7));
                    if random.next15() & 1 == 0 { 1 } else { 7 }
                }
                0 => 0,
                3 => {
                    changes.push(self.patch(1, 8));
                    4
                }
                6 => {
                    changes.push(self.patch(1, 7));
                    7
                }
                8 => {
                    if random.next15() & 1 == 0 {
                        9
                    } else {
                        3
                    }
                }
                9 => {
                    changes.push(self.patch(1, self.held[1]));
                    0
                }
                _ => cycle.wrapping_add(1),
            });
        }
    }
}
