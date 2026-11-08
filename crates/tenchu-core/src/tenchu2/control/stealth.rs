use super::{Request, StealthAttack, StealthPlayback};
use crate::{
    Result,
    math::Position,
    tenchu2::actor::{Actor, Resources, State, Velocity},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::registry::StealthEffect;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StealthSession {
    pub playback: StealthPlayback,
    pub origin: Position,
}

pub struct StealthInput {
    pub hero: usize,
    pub victim: usize,
    pub style: StealthAttack,
    pub penalties: u8,
    pub story_damage: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StealthCue {
    VictimWeapons,
    VictimCollision,
    KillCredit,
    Root,
    Recover,
    ReleaseGrip,
    Voice(u8),
    Tint,
    FinishContacts,
}

impl StealthAttack {
    pub fn index(self) -> usize {
        match self {
            Self::Drawing => 0,
            Self::Falling => 1,
            Self::Crouching => 2,
            Self::Front => 3,
            Self::Back => 4,
            Self::Right => 5,
            Self::Left => 6,
        }
    }

    fn actions(self) -> (i16, i16) {
        let offset = match self {
            Self::Drawing => 6,
            Self::Falling => 4,
            Self::Crouching => 5,
            Self::Front => 0,
            Self::Back => 1,
            Self::Right => 2,
            Self::Left => 3,
        };
        (0x920 + offset, 0xb10 + offset)
    }
}

impl Actor {
    pub fn start_stealth(
        &mut self,
        victim: &mut Actor,
        request: &mut Request,
        input: &mut StealthInput,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Actor, StealthCue) -> Result<()>,
    ) -> Result<StealthSession> {
        let events = resources
            .registry
            .stealth_events(input.hero, input.style.index())
            .ok_or("Missing Tenchu 2 stealth sequence")?;
        let (action, victim_action) = input.style.actions();
        let clip = resources
            .registry
            .actor(self.kind)
            .and_then(|a| a.motions.iter().find(|m| m.action == victim_action))
            .ok_or("Missing Tenchu 2 paired victim motion")?
            .clip;
        let session = StealthSession {
            playback: StealthPlayback {
                hero: input.hero,
                style: input.style.index(),
                next: usize::from(
                    victim.flags & 3 == 2
                        && events
                            .first()
                            .is_some_and(|e| matches!(e.effect, StealthEffect::VictimVoice(_))),
                ),
            },
            origin: self.position,
        };
        if matches!(input.style, StealthAttack::Drawing | StealthAttack::Falling)
            && input.penalties != 0
            && self.flags & 2 != 0
        {
            input.penalties -= 1;
        }
        self.target = Some(input.victim);
        victim.rotation.y = self.rotation.y;
        victim.position.x = self.position.x;
        victim.position.z = self.position.z;
        let health = victim.health;
        victim.health = 0;
        *request = Request {
            action,
            velocity: Velocity::Motion,
        };
        input.story_damage = input
            .story_damage
            .wrapping_add((i32::from(health) + 100) * 10);
        cue(self, victim, StealthCue::VictimWeapons)?;
        cue(self, victim, StealthCue::VictimCollision)?;
        victim.motion.clip = Some(clip);
        victim
            .motion
            .select(-1, victim.kind, &resources.registry, &resources.clips)?
            .ok_or("Missing Tenchu 2 paired victim clip")?;
        victim.state = State::Dead;
        victim.motion.action = victim_action;
        let clip = resources
            .clips
            .get(&clip)
            .ok_or("Missing Tenchu 2 paired victim clip")?;
        // Paired victims use the raw clip velocity even in the PAL controller.
        victim.set_horizontal_velocity(clip.forward_speed, clip.side_speed, &resources.math);
        cue(self, victim, StealthCue::KillCredit)?;
        Ok(session)
    }

    pub fn update_stealth(
        &mut self,
        victim: &Actor,
        request: &mut Request,
        session: &StealthSession,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, StealthCue) -> Result<()>,
    ) -> Result<()> {
        let clip = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 stealth motion")?)
            .ok_or("Missing Tenchu 2 stealth motion")?;
        let flags = if self.motion.action == 0x924 {
            0x2e000
        } else {
            0x2f000
        };
        if self.position.y != victim.position.y && self.motion.action != 0x924
            || victim.target.is_none()
            || self.flags & flags != 0
        {
            if i32::from(self.motion.frame) < i32::from(clip.duration) * 2 / 3 {
                *request = Request {
                    action: 0x900,
                    velocity: Velocity::Motion,
                };
                cue(self, request, StealthCue::Voice(5))?;
            } else {
                cue(self, request, StealthCue::Recover)?;
            }
            if self.kind == 2 {
                cue(self, request, StealthCue::ReleaseGrip)?;
            }
            self.target = None;
            self.position = session.origin;
        }
        if i32::from(self.motion.frame) == i32::from(clip.duration) - 1 || self.motion.loops > 0 {
            cue(self, request, StealthCue::Root)?;
            self.motion.pose.translation[0].x = 0;
            self.motion.pose.translation[0].z = 0;
            cue(self, request, StealthCue::Recover)?;
            if self.motion.action != request.action
                && self.state == State::Attack
                && self.motion.loops < 1
            {
                cue(self, request, StealthCue::FinishContacts)?;
            }
            self.change_motion(request.action, request.velocity, resources)?;
            request.velocity = Velocity::Keep;
            self.motion.frame = 0;
            let start = resources
                .clips
                .get(&self.motion.clip.ok_or("No Tenchu 2 post-kill motion")?)
                .and_then(|c| c.bones.first())
                .and_then(|keys| keys.first())
                .ok_or("Missing Tenchu 2 post-kill root")?
                .value[1];
            self.rotation.y = self
                .rotation
                .y
                .wrapping_add(self.motion.pose.rotation[0][1].wrapping_sub(start));
        }
        if self.motion.frame >= 2 {
            cue(self, request, StealthCue::Tint)?;
        }
        Ok(())
    }
}
