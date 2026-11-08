use super::aim::Aim;
use crate::math::{Math, Matrix, Position};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Flying,
    Attached,
    Released,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grapple {
    pub owner: usize,
    pub state: State,
    pub transform: Matrix,
    pub direction: Position,
    pub speed: i32,
    pub countdown: i32,
    pub origin: Position,
    pub pull_direction: Position,
    pub pull_distance: i32,
    pub rope: Vec<Position>,
}

impl Grapple {
    pub fn launch(
        owner: usize,
        origin: Position,
        direction: Position,
        aim: &mut Aim,
        math: &Math,
    ) -> Option<Self> {
        if direction.y >= 0 || aim.reach.is_none() || aim.distance == 0 {
            return None;
        }
        let mut countdown = aim.distance / 600;
        let mut direction = direction;
        direction.y = direction
            .y
            .wrapping_sub(countdown.wrapping_mul(countdown).wrapping_mul(16) >> 3);
        direction = math.normalize_vector(direction);
        aim.phase = aim.phase.wrapping_add(1);
        let mut angle = aim.phase.wrapping_mul(2) & 127;
        countdown = countdown.wrapping_add(countdown.wrapping_mul(angle) >> 10);
        if aim.phase & 1 != 0 {
            angle = -angle;
        }
        let [sine, cosine] = math.pair(angle as i16);
        direction.x = direction
            .x
            .wrapping_mul(cosine)
            .wrapping_sub(direction.z.wrapping_mul(sine))
            >> 12;
        direction.z = direction
            .x
            .wrapping_mul(sine)
            .wrapping_add(direction.z.wrapping_mul(cosine))
            >> 12;
        direction = math.normalize_vector(direction);
        Some(Self {
            owner,
            state: State::Flying,
            transform: Matrix {
                rotation: basis(direction, math),
                translation: origin,
            },
            direction,
            speed: 600,
            countdown,
            origin,
            pull_direction: Position::ZERO,
            pull_distance: 0,
            rope: Vec::new(),
        })
    }

    pub fn status(&self) -> crate::tenchu2::control::GrappleStatus {
        use crate::tenchu2::control::GrappleStatus;
        match self.state {
            State::Flying => GrappleStatus::Flying,
            State::Attached => GrappleStatus::Attached,
            State::Released => GrappleStatus::Released,
        }
    }

    pub fn release(&mut self) {
        self.state = State::Released;
    }

    pub fn advance(&mut self, target: Position, hand: Position, math: &Math) -> Option<Position> {
        match self.state {
            State::Released => None,
            State::Attached => {
                self.origin = self
                    .origin
                    .wrapping_add(self.pull_direction.map(|v| v.wrapping_mul(400) >> 12));
                self.pull_distance = self.pull_distance.wrapping_sub(400);
                if self.pull_distance <= 0 {
                    self.state = State::Released;
                }
                None
            }
            State::Flying if self.speed == 0 => {
                if let Some(&end) = self.rope.last() {
                    let count = self.rope.len();
                    for (i, point) in self
                        .rope
                        .iter_mut()
                        .take(count.saturating_sub(1))
                        .enumerate()
                    {
                        let line = hand.wrapping_add(
                            end.wrapping_sub(hand)
                                .map(|v| v.wrapping_mul(i as i32) / count as i32),
                        );
                        *point = Position::from_fn(|axis| {
                            (point[axis].wrapping_mul(3).wrapping_add(line[axis])) >> 2
                        });
                    }
                }
                let old_count = self.countdown;
                self.countdown = self.countdown.wrapping_sub(1);
                if old_count < 1 {
                    self.state = State::Attached;
                    let delta = self.transform.translation.wrapping_sub(self.origin);
                    self.pull_direction = math.normalize_vector(delta);
                    self.pull_distance = math.length(delta.map(|v| i32::from(v as i16)));
                    if self.pull_direction.y > 0 {
                        self.state = State::Released;
                    }
                }
                None
            }
            State::Flying => {
                let target_direction =
                    math.normalize_vector(target.wrapping_sub(self.transform.translation));
                self.direction = math.normalize_vector(Position::from_fn(|axis| {
                    self.direction[axis]
                        .wrapping_mul(self.countdown)
                        .wrapping_add(target_direction[axis])
                        >> 4
                }));
                self.countdown = self.countdown.wrapping_sub(1);
                self.transform.translation = self
                    .transform
                    .translation
                    .wrapping_add(self.direction.map(|v| v.wrapping_mul(self.speed) >> 12));
                let impact = if self.countdown < 0 {
                    self.transform.translation = target;
                    self.speed = 0;
                    self.countdown = 4;
                    Some(target_direction)
                } else {
                    None
                };
                if self.rope.len() < 64 {
                    self.rope.push(self.transform.translation);
                }
                if self.rope.len() > 62 {
                    self.state = State::Released;
                }
                impact
            }
        }
    }
}

fn basis(direction: Position, math: &Math) -> [[i16; 3]; 3] {
    let mut basis = super::facing(direction, -4096, math);
    for row in &mut basis {
        row[1] = row[1].wrapping_neg();
    }
    basis
}
