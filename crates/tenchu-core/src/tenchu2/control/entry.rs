use super::Request;
use crate::{
    Result,
    math::Math,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        ai::direction,
        input::{DOWN, LEFT, RIGHT, UP},
        physics::{FALLING, IN_WATER},
    },
};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JumpEffects {
    pub voices: Vec<u8>,
    pub splash: bool,
}

impl Actor {
    pub fn jump_input(
        &mut self,
        request: &mut Request,
        input: u16,
        resources: &Resources,
        is_player: bool,
    ) -> Result<JumpEffects> {
        let mut effects = JumpEffects::default();
        let previous = request.action;
        self.commit_root(&resources.math);
        let (action, forward, side) =
            if self.pad.consume(&resources.commands.jumping) == Some(0x607) {
                (
                    0x607,
                    if resources.registry.edition == Edition::Europe {
                        90
                    } else {
                        120
                    },
                    0,
                )
            } else if previous == 0x111 {
                (0x606, 150, 0)
            } else if input & UP != 0 {
                (0x601, 100, 0)
            } else if input & DOWN != 0 {
                (0x602, -100, 0)
            } else if input & RIGHT != 0 {
                (0x603, 0, -100)
            } else if input & LEFT != 0 {
                (0x604, 0, 100)
            } else {
                (0x600, 0, 0)
            };
        request.action = action;
        if self
            .change_motion(action, Velocity::Keep, resources)?
            .is_none()
        {
            if matches!(action, 0x606 | 0x607)
                || self
                    .change_motion(0x600, Velocity::Keep, resources)?
                    .is_none()
            {
                request.action = previous;
            }
            if request.action == previous {
                return Ok(effects);
            }
        }
        if request.action == 0x600 {
            self.update_velocity(Velocity::Stop, resources)?;
        } else {
            let scale = |v: i16| {
                if resources.registry.edition == Edition::Europe {
                    (i32::from(v) * 6 / 5) as i16
                } else {
                    v
                }
            };
            let [x, _, z] = resources
                .math
                .velocity(self.rotation.y, scale(forward), scale(side));
            self.velocity.x = x;
            self.velocity.z = z;
        }
        if request.action == 0x606 {
            effects.voices.push(0x15);
            if is_player && resources.registry.edition == Edition::Japan {
                effects.voices.push(0xc);
            }
        }
        effects.splash =
            self.terrain.height <= i32::from(self.height) && self.flags & IN_WATER != 0;
        Ok(effects)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StealthAttack {
    Drawing,
    Falling,
    Crouching,
    Front,
    Back,
    Right,
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackStart {
    None,
    Strike { action: i16, velocity: Velocity },
    Draw { crouched: bool },
    Stealth(StealthAttack),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackInput {
    pub draw_weapon: bool,
    pub start: AttackStart,
}

impl Actor {
    /// NPC controllers pass no stealth candidate; the player passes the nearest actor.
    pub fn attack_input(
        &self,
        request: Request,
        input: u16,
        target: Option<&Actor>,
        edition: Edition,
        map: i16,
        math: &Math,
    ) -> Result<AttackInput> {
        let mut draw_weapon = false;
        if let Some(target) = target
            && self.flags & 0x2e000 == 0
            && target.is_stealth_vulnerable()
            && target.kind & 0xf0 != 0x70
            && target.flags & 0x2f000 == 0
            && !matches!(target.motion.action, 0x70c | 0xa13)
            && self.terrain.blocked == 0
        {
            draw_weapon = !self.weapon_drawn() && self.kind < 2;
            if let Some(style) = self.stealth_style(target, request.action, map, math)? {
                return Ok(AttackInput {
                    draw_weapon,
                    start: AttackStart::Stealth(style),
                });
            }
        }
        let start = if self.weapon_drawn() || draw_weapon {
            if self.flags & FALLING != 0 && self.terrain.blocked != 0 {
                AttackStart::None
            } else {
                let (action, velocity) = if self.state == State::Jumping || request.action == 0x702
                {
                    (0x911, Velocity::Keep)
                } else if self.state == State::Crouched {
                    (0x914, Velocity::Motion)
                } else if request.action == 0x111 {
                    (0x910, Velocity::Motion)
                } else if edition == Edition::Japan && request.action == 0x112 {
                    (0x913, Velocity::Motion)
                } else if input & UP != 0 {
                    (
                        if input & DOWN != 0 { 0x915 } else { 0x900 },
                        Velocity::Motion,
                    )
                } else if input & RIGHT != 0 {
                    (0x907, Velocity::Motion)
                } else if input & LEFT != 0 {
                    (0x90a, Velocity::Motion)
                } else if edition == Edition::Europe && input & DOWN != 0 {
                    (0x913, Velocity::Motion)
                } else {
                    (0x900, Velocity::Motion)
                };
                AttackStart::Strike { action, velocity }
            }
        } else if self.flags & FALLING != 0 {
            AttackStart::None
        } else {
            AttackStart::Draw {
                crouched: self.state == State::Crouched,
            }
        };
        Ok(AttackInput { draw_weapon, start })
    }

    fn stealth_style(
        &self,
        target: &Actor,
        action: i16,
        map: i16,
        math: &Math,
    ) -> Result<Option<StealthAttack>> {
        let angle = direction(
            math,
            self.position.x.wrapping_sub(target.position.x),
            self.position.z.wrapping_sub(target.position.z),
            target.rotation.y,
        )?;
        let mut style = if (self.state == State::Jumping || action == 0x702)
            && self.position.y < target.position.y
        {
            StealthAttack::Falling
        } else {
            if self.position.y != target.position.y || self.flags & FALLING != 0 {
                return Ok(None);
            }
            if target.motion.action == 0x700 {
                StealthAttack::Drawing
            } else if self.state == State::Crouched {
                StealthAttack::Crouching
            } else if i32::from(angle).abs() < 512 {
                StealthAttack::Front
            } else if i32::from(angle).abs() > 1536 {
                StealthAttack::Back
            } else if angle > 0 {
                StealthAttack::Right
            } else {
                StealthAttack::Left
            }
        };
        if self.kind == 2 {
            if map == 12 {
                if style == StealthAttack::Falling {
                    return Ok(None);
                }
                style = StealthAttack::Drawing;
            } else {
                if matches!(
                    style,
                    StealthAttack::Falling | StealthAttack::Front | StealthAttack::Back
                ) && (0x56..0x5b).contains(&target.kind)
                {
                    return Ok(None);
                }
                if style == StealthAttack::Drawing {
                    style = StealthAttack::Back;
                }
            }
        }
        Ok(Some(style))
    }

    pub fn start_strike(
        &mut self,
        start: AttackStart,
        request: &mut Request,
        resources: &Resources,
    ) -> Result<()> {
        match start {
            AttackStart::Strike { action, velocity } => {
                let old = self.motion.action;
                if self.change_motion(action, velocity, resources)?.is_some() {
                    self.commit_root(&resources.math);
                    request.action = action;
                } else {
                    request.action = old;
                }
                request.velocity = Velocity::Keep;
            }
            AttackStart::Draw { crouched } => {
                request.action = 0x700;
                request.velocity = Velocity::Motion;
                if crouched
                    && self
                        .change_motion(0x700, Velocity::Motion, resources)?
                        .is_some()
                {
                    self.motion.frame = -10;
                }
            }
            AttackStart::None | AttackStart::Stealth(_) => {}
        }
        Ok(())
    }
}
