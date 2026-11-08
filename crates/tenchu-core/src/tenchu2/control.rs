use super::actor::{Actor, State, Velocity};
use crate::{math::Math, random::Random};

mod attack;
mod crouch;
mod entry;
mod item;
mod jump;
mod ledge;
mod sequence;
mod standing;
pub use attack::{AttackContext, AttackPreparation, CombatCue};
pub use crouch::{CrouchCue, CrouchInput};
pub use entry::{AttackInput, AttackStart, JumpEffects, StealthAttack};
pub use item::{ItemInput, ItemSound};
pub use ledge::{LedgeCue, LedgeInput};
pub use sequence::{AttackCue, Projectile};
pub use standing::{StandingCue, StandingInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub action: i16,
    pub velocity: Velocity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Affliction {
    Burning,
    Poison,
    Confusion,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusResult {
    pub effect: Option<Affliction>,
    pub died: bool,
    pub show_health: bool,
    pub staggered: bool,
}

impl Actor {
    pub fn update_status(
        &mut self,
        request: &mut Request,
        random: &mut Random,
        is_player: bool,
        controls_enabled: bool,
    ) -> StatusResult {
        let mut result = StatusResult::default();
        if is_player && !controls_enabled {
            return result;
        }
        let previous = request.action;
        if self.flags & 0x200 != 0 {
            request.action = 0xa10;
            request.velocity = Velocity::Motion;
            result.effect = Some(Affliction::Burning);
        } else if self.flags & 0x100 != 0 && self.motion.frame == 1 {
            if self.motion.loops < 1 && self.state != State::Attack {
                return result;
            }
            if random.next15() % 10 <= i32::from(self.skill) {
                self.flags &= !0x100;
            }
            self.health = self.health.wrapping_sub(5);
            if self.health <= 0 {
                self.health = 0;
                request.action = 0xb07;
                result.died = true;
            } else {
                request.action = 0xa11;
            }
            request.velocity = Velocity::Motion;
            result.effect = Some(Affliction::Poison);
            result.show_health = !is_player;
        } else if self.flags & 0x400 != 0 && random.next15() % 3 == 0 {
            if random.next15() & 1 == 0 {
                self.flags &= !0x400;
            } else {
                result.effect = Some(Affliction::Confusion);
            }
            request.action = 0xa15;
            request.velocity = Velocity::Motion;
        }
        if previous != request.action {
            result.staggered = true;
            if matches!(self.kind & 0xf0, 0xe0 | 0xf0) {
                self.flags &= !0xf00;
            }
            if self.kind & 0xf0 == 0x70 || matches!(self.kind, 0xe5 | 0x1a) {
                request.action = if self.health != 0 {
                    0xa00
                } else if self.terrain.material & 0xf00 == 0 {
                    0xb00
                } else {
                    0xb0a
                };
                request.velocity = Velocity::Motion;
                self.flags &= !0xf00;
            }
        }
        result
    }

    pub fn is_stealth_vulnerable(&self) -> bool {
        !matches!(self.kind & 0xf0, 0 | 0x10 | 0x60 | 0xe0 | 0xf0)
            && self.health > 0
            && !self.weapon_drawn()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TargetQuery {
    Kind(u8),
    Nearby { distance: i32, enemies_only: bool },
}

pub fn find_target(
    actors: &[Actor],
    origin: usize,
    player: Option<usize>,
    query: TargetQuery,
    math: &Math,
) -> Option<usize> {
    match query {
        TargetQuery::Kind(kind) => actors
            .iter()
            .position(|actor| actor.kind == i16::from(kind)),
        TargetQuery::Nearby {
            distance,
            enemies_only,
        } => {
            let position = actors.get(origin)?.position;
            let mut closest = i32::MAX;
            let mut target = None;
            for (index, actor) in actors.iter().enumerate() {
                if index == origin
                    || actor.flags & 8 != 0
                    || (enemies_only
                        && (Some(index) == player
                            || actor.health <= 0
                            || matches!(actor.kind & 0xf0, 0 | 0x10 | 0x60)))
                {
                    continue;
                }
                let delta = actor.position.wrapping_sub(position);
                if delta.x.wrapping_abs() >= 32768 || delta.z.wrapping_abs() >= 32768 {
                    continue;
                }
                let length = math.length(delta);
                if length < closest {
                    closest = length;
                    target = Some(index);
                }
            }
            (closest <= distance).then_some(target).flatten()
        }
    }
}
