use super::{Effect, Effects};
use crate::{
    Result,
    math::{Matrix, Position},
    scene::ModelId,
    visibility::project,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Weapon {
        actor: usize,
        slot: usize,
        part: usize,
    },
    Projectile(ModelId),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sample {
    pub edges: [[i16; 2]; 2],
    pub depth: i32,
}

impl Sample {
    pub fn weapon(matrix: &Matrix, edges: [[i16; 3]; 2]) -> Self {
        let a = project(matrix, edges[0]);
        let b = project(matrix, edges[1]);
        Self {
            edges: [a.0, b.0],
            depth: i32::from(b.1 >> 4),
        }
    }

    pub fn projectile(
        view: &Matrix,
        position: Position,
        velocity: Position,
        shift: u8,
    ) -> Option<Self> {
        let matrix = view.compose(&Matrix {
            translation: position,
            ..Default::default()
        });
        let a = project(
            &matrix,
            [
                (velocity.z >> shift) as i16,
                0,
                (velocity.x.wrapping_neg() >> shift) as i16,
            ],
        );
        let b = project(
            &matrix,
            [
                (velocity.z.wrapping_neg() >> shift) as i16,
                0,
                (velocity.x >> shift) as i16,
            ],
        );
        let depth = i32::from(a.1 >> 4);
        (depth < 1024).then_some(Self {
            edges: [a.0, b.0],
            depth,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ribbon {
    pub source: Source,
    samples: [Sample; 16],
    age: i16,
    interval: i16,
    oldest: usize,
    next: usize,
    count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Drawing {
    pub corners: [[i16; 2]; 4],
    pub colors: [[u8; 3]; 4],
    pub depth: i32,
}

impl Ribbon {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            samples: [Sample::default(); 16],
            age: 0,
            interval: if matches!(source, Source::Projectile(_)) {
                2
            } else {
                4
            },
            oldest: 0,
            next: 0,
            count: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.interval != 0
    }

    pub fn push(&mut self, sample: Option<Sample>) {
        let Some(sample) = sample else {
            self.interval = 0;
            return;
        };
        self.samples[self.next] = sample;
        self.next = (self.next + 1) & 15;
        if self.count < 15 {
            self.count += 1;
        } else {
            self.oldest = (self.oldest + 1) & 15;
        }
    }

    pub fn update(&mut self, drawings: &mut Vec<Drawing>) -> bool {
        let mut blue = (96i32 - i32::from(self.count) * 16).clamp(0, 255) as u8;
        let mut red = blue.saturating_sub(16);
        let age = self.age;
        self.age = self.age.wrapping_add(1);
        let mut alive = true;
        if age >= self.interval {
            if self.oldest == self.next {
                if !self.active() {
                    alive = false;
                }
            } else {
                self.oldest = (self.oldest + 1) & 15;
            }
            self.count = self.count.saturating_sub(1);
            self.age = 0;
        }
        if self.count >= 2 {
            let mut a = (self.oldest + 1) & 15;
            while (a + 1) & 15 != self.next {
                let b = (a + 1) & 15;
                let before = self.samples[a];
                let after = &mut self.samples[b];
                for axis in 0..2 {
                    let shrink = ((i32::from(after.edges[0][axis])
                        - i32::from(after.edges[1][axis]))
                        >> 8) as i16;
                    after.edges[0][axis] = after.edges[0][axis].wrapping_sub(shrink);
                    after.edges[1][axis] = after.edges[1][axis].wrapping_add(shrink);
                }
                let old = [red, red, blue];
                blue = blue.saturating_add(16);
                red = blue.saturating_sub(8);
                let new = [red, red, blue];
                drawings.push(Drawing {
                    corners: [
                        before.edges[0],
                        before.edges[1],
                        after.edges[0],
                        after.edges[1],
                    ],
                    colors: [old, old, new, new],
                    depth: before.depth,
                });
                a = b;
            }
        }
        alive
    }
}

impl Effects {
    pub fn start_ribbon(&mut self, source: Source) -> Result<()> {
        self.sample_ribbon(source, None);
        self.insert(Effect::Ribbon(Ribbon::new(source)))
    }

    pub fn sample_ribbon(&mut self, source: Source, sample: Option<Sample>) {
        if let Some(Effect::Ribbon(ribbon)) = self.entries.iter_mut().rev().find(|effect| {
            matches!(effect, Effect::Ribbon(ribbon) if ribbon.source == source && ribbon.active())
        }) {
            ribbon.push(sample);
        }
    }

    pub fn end_weapon_ribbon(&mut self, actor: usize, slot: usize) {
        for effect in &mut self.entries {
            if let Effect::Ribbon(ribbon) = effect
                && matches!(ribbon.source, Source::Weapon { actor: a, slot: s, .. } if a == actor && s == slot)
            {
                ribbon.push(None);
            }
        }
    }
}

#[cfg(test)]
mod tests;
