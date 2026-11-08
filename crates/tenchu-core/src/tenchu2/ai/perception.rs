use super::{
    Decision, ForcedInput,
    combat::Context,
    local_alert_only,
    navigation::Surroundings,
    sight::Visibility,
    status::{Alert, AlertCause},
};
use crate::{
    Result,
    math::Position,
    tenchu2::{actor::State, input, physics::IN_WATER},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Routine {
    Idle,
    Suspicious,
    Combat,
    Search,
    Watch,
    Corpse,
}

pub trait Reactions {
    fn decide(
        &mut self,
        routine: Routine,
        c: &mut Context<'_>,
        global: &mut Global,
    ) -> Result<Decision>;
    fn action(&mut self, c: &mut Context<'_>, action: i16) -> Result<bool>;
    fn voice(&mut self, c: &mut Context<'_>, voice: u8) -> Result<()>;
    fn reset(&mut self, forget: bool);
    fn remember(&mut self, position: Position);
    fn observe(&mut self, visibility: Visibility);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Global {
    pub alert: Alert,
    pub nearest_distance: i32,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            alert: Alert::default(),
            nearest_distance: i32::MAX,
        }
    }
}

pub struct Scene<'a> {
    pub map: usize,
    pub sequence: u16,
    pub script: u16,
    pub player_distance: i32,
    pub notice_distance: i32,
    pub player_action: i16,
    pub target_is_player: bool,
    pub recovering_confusion: bool,
    pub surroundings: &'a Surroundings,
}

pub struct Outcome {
    pub decision: Decision,
    pub spotted: bool,
}

#[derive(Clone, Copy)]
enum Awareness {
    Calm,
    Suspicious,
    Fighting,
    Searching,
}

impl Awareness {
    fn from_flags(flags: u32) -> Self {
        match flags & 3 {
            0 => Self::Calm,
            1 => Self::Suspicious,
            2 => Self::Fighting,
            _ => Self::Searching,
        }
    }
}

pub fn update(
    c: &mut Context<'_>,
    scene: &Scene<'_>,
    global: &mut Global,
    reactions: &mut impl Reactions,
) -> Result<Outcome> {
    let original_flags = c.actor.flags;
    let base_flags = original_flags & !0x83;
    *c.visibility = global
        .alert
        .visibility(c.actor, c.observed_flags, scene.script, *c.visibility);
    if *c.visibility != Visibility::Unchanged {
        reactions.observe(*c.visibility);
    }
    let training = scene.map == 0 && c.actor.kind & 0xf0 == 0x60;
    if training {
        c.observed_flags &= !3;
    }
    let mut spotted = false;
    let decision = match Awareness::from_flags(c.observed_flags) {
        Awareness::Calm => {
            global.nearest_distance = global.nearest_distance.min(scene.player_distance);
            let decision = reactions.decide(Routine::Idle, c, global)?;
            if !training {
                if *c.visibility == Visibility::Recognized
                    || (c.observed_flags & 0x400000 != 0 && !local_alert_only(c.actor.kind))
                {
                    if c.actor.motion.action != 0x70c {
                        let changed = reactions.action(c, 0x709)?;
                        if !changed && c.resources.registry.edition == Edition::Japan {
                            reactions.voice(c, 8)?;
                        }
                    }
                    c.observed_flags = base_flags | 2;
                    reactions.reset(true);
                    spotted = recognize(
                        c,
                        scene,
                        global,
                        reactions,
                        c.resources.registry.edition == Edition::Europe,
                    )?;
                } else if *c.visibility == Visibility::Suspicious {
                    let forget = global.alert.remaining != 0 && !c.actor.weapon_drawn();
                    reactions.action(c, if forget { 0x700 } else { 0x708 })?;
                    c.observed_flags = base_flags | 1;
                    reactions.reset(forget);
                    if c.random.next15() & 1 != 0 {
                        reactions.voice(c, 7)?;
                    }
                } else if c.actor.weapon_drawn() {
                    reactions.action(c, 0x701)?;
                }
            }
            decision
        }
        Awareness::Suspicious => {
            let routine = if global.alert.remaining == 0 && c.observed_flags & 0x80 == 0 {
                if global.nearest_distance > 0
                    && !local_alert_only(c.actor.kind)
                    && scene.player_distance < scene.notice_distance
                {
                    global.nearest_distance = scene.player_distance.wrapping_neg();
                }
                Routine::Suspicious
            } else {
                searching_radar(c, global);
                if c.observed_flags & 0x80 != 0 {
                    Routine::Corpse
                } else {
                    Routine::Watch
                }
            };
            let decision = reactions.decide(routine, c, global)?;
            if *c.visibility == Visibility::Recognized
                || (c.observed_flags & 0x80000 != 0
                    && matches!(
                        *c.visibility,
                        Visibility::Recognized | Visibility::Suspicious
                    ))
            {
                c.observed_flags = base_flags | 2;
                if original_flags & 0x40 == 0 {
                    reactions.action(c, 0x700)?;
                }
                reactions.reset(true);
                spotted = recognize(c, scene, global, reactions, true)?;
            } else if global.alert.remaining < 2
                && matches!(*c.visibility, Visibility::Hidden | Visibility::Unnoticed)
            {
                if c.actor.weapon_drawn() {
                    reactions.action(c, 0x701)?;
                }
                c.observed_flags = base_flags;
                reactions.reset(false);
            }
            decision
        }
        Awareness::Searching => {
            searching_radar(c, global);
            let decision = reactions.decide(Routine::Search, c, global)?;
            if *c.visibility == Visibility::Recognized || c.observed_flags & 0x400000 != 0 {
                c.observed_flags = base_flags | 2;
            }
            if matches!(Awareness::from_flags(c.observed_flags), Awareness::Fighting) {
                if !local_alert_only(c.actor.kind) {
                    raise(c, global)?;
                }
                reactions.reset(true);
                if c.random.next15() & 1 != 0 && scene.script == 0 {
                    reactions.voice(c, 8)?;
                }
            }
            decision
        }
        Awareness::Fighting => {
            if !local_alert_only(c.actor.kind) {
                if scene.target_is_player {
                    global.nearest_distance = 0;
                } else if global.nearest_distance > 0 {
                    global.nearest_distance = -32768;
                }
            }
            if !c.actor.weapon_drawn() {
                reactions.action(c, 0x700)?;
            }
            if scene.recovering_confusion {
                reactions.reset(false);
            }
            let mut decision = reactions.decide(Routine::Combat, c, global)?;
            raise(c, global)?;
            if decision.held & input::ATTACK != 0 {
                c.observed_flags |= 0x80;
                if scene.player_action == 0xa25
                    || (c.actor.target.is_some()
                        && !scene.target_is_player
                        && c.target()?.state == State::Dead)
                {
                    decision.held &= 0xf0;
                }
            }
            if *c.visibility == Visibility::Hidden {
                c.observed_flags = base_flags | 0x83;
                reactions.remember(c.target()?.position);
                reactions.reset(false);
                if c.random.next15() & 1 != 0 && scene.script == 0 {
                    reactions.voice(c, 9)?;
                }
            }
            if !c.steering.forced && decision.forced.is_none() {
                avoid(c, scene.surroundings, &mut decision)?;
            }
            decision
        }
    };
    Ok(Outcome { decision, spotted })
}

fn searching_radar(c: &Context<'_>, global: &mut Global) {
    if global.nearest_distance > 0 && !local_alert_only(c.actor.kind) {
        global.nearest_distance = -32768;
    }
}

fn raise(c: &Context<'_>, global: &mut Global) -> Result<()> {
    global
        .alert
        .raise(AlertCause::Combat, c.difficulty as u8, c.rules)
}

fn recognize(
    c: &mut Context<'_>,
    scene: &Scene<'_>,
    global: &mut Global,
    reactions: &mut impl Reactions,
    random_voice: bool,
) -> Result<bool> {
    if c.actor.health <= 0 {
        return Ok(false);
    }
    if random_voice && c.random.next15() & 1 != 0 {
        reactions.voice(c, 8)?;
    }
    if !local_alert_only(c.actor.kind) {
        raise(c, global)?;
    }
    Ok(
        !matches!(c.actor.kind & 0xf0, 0 | 0x10 | 0x70 | 0xe0 | 0xf0)
            && scene.target_is_player
            && scene.script == 0
            && scene.sequence != 200,
    )
}

fn avoid(c: &mut Context<'_>, surroundings: &Surroundings, decision: &mut Decision) -> Result<()> {
    if c.actor.flags & IN_WATER == 0 {
        if decision.held & input::DOWN != 0
            && surroundings
                .behind
                .height
                .is_some_and(|height| surroundings.behind.material & 0xf00 != 0 || height > 4000)
        {
            decision.forced = Some(ForcedInput {
                held: input::UP,
                remaining: 30,
            });
            c.steering.forced = true;
        }
        if decision.held & input::UP != 0
            && surroundings
                .ahead
                .height
                .is_some_and(|height| surroundings.ahead.material & 0xf00 != 0 || height > 4000)
        {
            let correction = c.steer()?;
            decision.held = correction.held & (input::LEFT | input::RIGHT);
            decision.forced = correction.forced.or(decision.forced);
        }
    }
    if c.actor.target.is_some()
        && matches!(c.target()?.action, 0x803 | 0x805)
        && c.difficulty != 3
        && c.random.next15() % (c.difficulty as i32 + 1) == 0
    {
        let dodge = c.random.next15() % 3;
        decision.held = if dodge == 0
            && surroundings
                .right
                .height
                .is_some_and(|height| height < 2000)
        {
            c.command(0x113)
        } else if dodge == 1 && surroundings.left.height.is_some_and(|height| height < 2000) {
            c.command(0x114)
        } else {
            input::JUMP
        };
    }
    Ok(())
}
