use super::{
    combat::Context,
    local_alert_only, perception,
    status::{AlertCause, Ki},
};
use crate::{Result, math::Position, tenchu2::actor::State, tenchu2::input};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Global {
    pub perception: perception::Global,
    pub radar: Ki,
    pub previous_radar: Ki,
    pub player_held: u16,
    pub rotation_phase: i16,
}

impl Default for Global {
    fn default() -> Self {
        Self {
            perception: perception::Global::default(),
            radar: Ki::Clear,
            previous_radar: Ki::Clear,
            player_held: 0,
            rotation_phase: 0,
        }
    }
}

pub struct Scene {
    pub actor: usize,
    pub player: usize,
    pub player_health: i16,
    pub player_position: Position,
    pub tick: u32,
    pub slot: i16,
    pub map: usize,
    pub has_suspicious_program: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub actor: usize,
    pub kind: i16,
}

pub trait Reactions {
    fn nearest_target(&mut self, c: &mut Context<'_>, range: i32) -> Result<Option<Target>>;
    fn action(&mut self, c: &mut Context<'_>, action: i16) -> Result<()>;
    fn idle(&mut self, c: &mut Context<'_>, global: &mut Global) -> Result<u16>;
    fn victory(&mut self, c: &mut Context<'_>) -> Result<u16>;
    fn corpses(&mut self, c: &mut Context<'_>, global: &mut Global) -> Result<()>;
    fn confusion(&mut self, c: &mut Context<'_>) -> Result<u16>;
    fn sight(&mut self, c: &mut Context<'_>) -> Result<()>;
    fn sample_player_terrain(&mut self, c: &mut Context<'_>) -> Result<()>;
    fn reset(&mut self, forget: bool);
    fn clear_forced(&mut self);
    fn recover_confusion(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Complete(u16),
    Perception { player_distance: i32 },
}

pub fn update(
    c: &mut Context<'_>,
    scene: &Scene,
    global: &mut Global,
    reactions: &mut impl Reactions,
) -> Result<Outcome> {
    let player = scene.actor == scene.player;
    c.observed_flags = c.actor.flags;
    let base_flags = c.observed_flags & !0x83;
    if player {
        global.previous_radar = global.radar;
        global.radar = Ki::Clear;
        global.perception.nearest_distance = i32::MAX;
        global.rotation_phase = 0;
    } else if local_alert_only(c.actor.kind) && scene.map != 0 {
        if c.actor.health == 0 {
            return Ok(complete(c, 0));
        }
        if scene.tick.wrapping_add(scene.slot as u32).wrapping_sub(1) & 31 == 0 {
            match reactions.nearest_target(c, -10000)? {
                None if c.actor.target.is_some() => {
                    if c.actor.weapon_drawn() {
                        reactions.action(c, 0x701)?;
                    }
                    c.observed_flags = base_flags;
                    reactions.reset(true);
                    c.actor.target = None;
                }
                Some(target)
                    if c.actor.target != Some(target.actor)
                        && target.actor != scene.player
                        && !local_alert_only(target.kind) =>
                {
                    reactions.reset(true);
                    c.actor.target = Some(target.actor);
                }
                _ => {}
            }
        }
    } else if scene.player_health <= 0 && c.actor.state != State::Dead {
        let held = reactions.victory(c)?;
        return Ok(finish(c, global, held));
    }
    let europe = c.resources.registry.edition == Edition::Europe;
    let health_limit = if europe { 1 } else { 0 };
    if c.actor.state != State::Dead && c.actor.health < health_limit {
        let held = if c.observed_flags & 0x10 != 0 {
            reactions.idle(c, global)?
        } else {
            0
        };
        return Ok(complete(c, held));
    }
    let injured = if europe {
        c.actor.state == State::Hurt || (c.actor.state == State::Dead && c.actor.health <= 0)
    } else {
        matches!(c.actor.state, State::Hurt | State::Dead) || c.actor.health == 0
    };
    if injured {
        reactions.clear_forced();
        if !player {
            if c.actor.health <= 0 {
                reactions.corpses(c, global)?;
            } else {
                if global.perception.nearest_distance > 0 {
                    let distance = if (c.actor.flags & 0x800 == 0 || c.actor.motion.loops >= 0)
                        && c.actor.motion.action != 0xa14
                    {
                        -32768
                    } else {
                        player_distance(c, scene)
                    };
                    global.perception.nearest_distance =
                        global.perception.nearest_distance.min(distance);
                    global.radar = Ki::from_distance(global.perception.nearest_distance);
                }
                if c.actor.flags & 0x400 != 0 {
                    reactions.reset(false);
                    reactions.recover_confusion();
                }
            }
        }
        let mut held = 0;
        if c.actor.state == State::Hurt
            && c.actor.flags & 0x400 == 0
            && !player
            && matches!(c.actor.motion.action, 0xa26 | 0xa00..=0xa03)
        {
            reactions.sight(c)?;
            held = input::DOWN
                | if c.sight.angle > 200 {
                    input::RIGHT
                } else if c.sight.angle < -200 {
                    input::LEFT
                } else {
                    0
                };
            if global.perception.alert.remaining > 1 && global.perception.alert.position.is_some() {
                global.perception.alert.position = Some(c.actor.position);
            }
        }
        return Ok(complete(c, held));
    }
    if c.observed_flags & 0x10 == 0 {
        if player {
            global.perception.alert.advance();
        }
        let mut held = 0;
        if !scene.has_suspicious_program {
            if !europe {
                reactions.sample_player_terrain(c)?;
            }
            held = reactions.idle(c, global)?;
            if player {
                global.player_held = held;
            }
        }
        return Ok(finish(c, global, held));
    }
    if c.actor.flags & 0x400 != 0 {
        if !player && c.actor.health > 0 && global.perception.nearest_distance > 0 {
            global.perception.nearest_distance = -32768;
            global.radar = Ki::Searching;
            if !local_alert_only(c.actor.kind) {
                global
                    .perception
                    .alert
                    .raise(AlertCause::Corpse, c.difficulty as u8, c.rules)?;
            }
        }
        let held = reactions.confusion(c)?;
        return Ok(complete(c, held));
    }
    if c.actor.flags & 0x100 != 0
        && c.actor.inventory.counts[2] != 0
        && c.actor.health < c.actor.max_health >> 1
    {
        let held = c.select_item(2)?;
        return Ok(complete(c, held));
    }
    reactions.sight(c)?;
    let player_distance = if c.actor.target == Some(scene.player) {
        c.sight.distance
    } else {
        player_distance(c, scene)
    };
    Ok(Outcome::Perception { player_distance })
}

fn player_distance(c: &Context<'_>, scene: &Scene) -> i32 {
    let delta = scene.player_position.wrapping_sub(c.actor.position);
    if delta.x.wrapping_abs() < 32767 && delta.z.wrapping_abs() < 32767 {
        c.resources.math.length(delta)
    } else {
        i32::MAX
    }
}

fn finish(c: &mut Context<'_>, global: &mut Global, held: u16) -> Outcome {
    global.radar = Ki::from_distance(global.perception.nearest_distance);
    complete(c, held)
}

fn complete(c: &mut Context<'_>, held: u16) -> Outcome {
    c.actor.pad.update(held);
    Outcome::Complete(held)
}
