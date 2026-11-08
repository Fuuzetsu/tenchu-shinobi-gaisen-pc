use super::{ForcedInput, Probe, SteeringInput};
use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Actor, State},
        input,
        physics::{BLOCKED, FALLING, IN_WATER},
        terrain::{self, Cursor, Query},
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Ground {
    pub height: Option<i32>,
    pub material: u16,
}

impl Default for Ground {
    fn default() -> Self {
        Self {
            height: Some(0),
            material: 0,
        }
    }
}

impl Ground {
    fn probe(self) -> Option<Probe> {
        self.height.map(|height| Probe {
            height,
            material: self.material,
        })
    }

    fn blocks(self, alert: i16) -> bool {
        self.material & 0x300 != 0 || (self.material & 0x400 != 0 && alert == 0)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Surroundings {
    pub ahead: Ground,
    pub behind: Ground,
    pub left: Ground,
    pub right: Ground,
}

impl Surroundings {
    pub fn sample(
        &mut self,
        actor: &Actor,
        tick: u32,
        distance: i16,
        map: &Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<()> {
        let (forward, side, positive, negative) = if tick & 1 == 0 {
            (distance, 0, &mut self.ahead, &mut self.behind)
        } else {
            (0, distance, &mut self.left, &mut self.right)
        };
        let delta = Position::from(
            math.velocity(actor.rotation.y, forward, side)
                .map(i32::from),
        );
        for (probe, point) in [
            (positive, actor.position.wrapping_add(delta)),
            (negative, actor.position.wrapping_sub(delta)),
        ] {
            let point = Position {
                y: point.y.wrapping_sub(2000),
                ..point
            };
            probe.height = terrain::level(map, cursor, point, Query::Relative)?
                .map(|height| height.wrapping_sub(2000));
            probe.material = cursor.material;
        }
        Ok(())
    }

    pub fn steering(&self, actor: &Actor, flags: u32, angle: i16, forced: bool) -> SteeringInput {
        SteeringInput {
            yaw: actor.rotation.y,
            turn: actor.turn,
            target_angle: angle,
            alerted: flags & 3 != 0,
            blocked: flags & 0x48000 != 0,
            forced,
            ahead: self.ahead.probe(),
            left: self.left.height.is_some(),
            right: self.right.height.is_some(),
        }
    }
}

pub struct Navigation<'a> {
    pub actor: &'a Actor,
    pub target: Option<&'a Actor>,
    pub surroundings: &'a Surroundings,
    pub facing: i32,
    pub alert: i16,
    pub player_alive: bool,
}

impl Navigation<'_> {
    pub fn finish(
        &self,
        mut held: u16,
        forced: &mut Option<ForcedInput>,
        random: &mut Random,
    ) -> u16 {
        if let Some(current) = *forced {
            (held, *forced) = current.advance(random);
        }
        let actor = self.actor;
        let ahead = self.surroundings.ahead;
        if actor.state == State::Ledge {
            held = if actor.terrain.floor.is_some()
                && self
                    .target
                    .is_some_and(|target| target.position.y < actor.position.y)
            {
                input::DOWN
            } else {
                input::UP
            };
        } else {
            let jump = held & input::UP != 0
                && self.facing < 512
                && ahead.height.is_some_and(|height| {
                    (-1999..-800).contains(&height)
                        || (ahead.material & 0xf00 == 0
                            && height > 2000
                            && height < 5000
                            && self.target.is_some_and(|target| height < target.position.y))
                });
            if jump {
                forced.get_or_insert(ForcedInput {
                    held: input::UP,
                    remaining: 15,
                });
                held |= (actor.pad.held ^ input::JUMP) & input::JUMP | input::UP;
                if actor.flags & FALLING != 0 {
                    held &= !input::JUMP;
                }
            } else if actor.flags & BLOCKED != 0
                && actor.terrain.floor.is_none()
                && forced.is_none()
            {
                *forced = Some(ForcedInput {
                    held: if random.next15() & 1 != 0 {
                        input::RIGHT
                    } else {
                        input::LEFT
                    },
                    remaining: 30,
                });
            }
        }
        if actor.flags & IN_WATER != 0 {
            held &= !input::JUMP;
        } else {
            for (button, probe) in [(input::UP, ahead), (input::DOWN, self.surroundings.behind)] {
                if held & button != 0
                    && (probe
                        .height
                        .is_some_and(|height| height.wrapping_abs() > 5000)
                        || probe.blocks(self.alert))
                {
                    held &= !(button | input::JUMP);
                }
            }
        }
        if !self.player_alive {
            held &= !input::ATTACK;
        }
        held
    }
}
