use super::{
    Decision,
    combat::Context,
    direction,
    sight::{Body, Ranges, Visibility, scale_range},
};
use crate::{
    Result,
    math::Position,
    scene::ModelId,
    tenchu2::{actor::State, terrain},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Bait {
    pub model: ModelId,
    pub position: Position,
}

pub struct Observation<'a> {
    pub player: usize,
    pub body: Body,
    pub slot: i16,
    pub range: i16,
    pub range_percent: u16,
    pub ignore_player: bool,
    pub ranges: &'a mut Ranges,
}

pub enum Outcome {
    Continue(Decision),
    ResumePatrol,
    LookAround,
}

pub fn update(
    bait: Position,
    c: &mut Context<'_>,
    global: &super::perception::Global,
    frame: &mut super::behavior::Frame<'_, impl super::behavior::Events>,
) -> Result<Outcome> {
    if global.alert.remaining != 0 {
        *c.visibility = Visibility::Suspicious;
        return Ok(Outcome::ResumePatrol);
    }
    *c.visibility = Visibility::Unnoticed;
    if frame.tick.wrapping_add(frame.observation.slot as u32) & 31 == 0 {
        let player = frame.observation.body;
        let delta = player.position.wrapping_sub(c.actor.position);
        let math = &c.resources.math;
        let distance = math.length(Position::new(delta.x, 0, delta.z));
        c.facing = i32::from(direction(math, delta.x, delta.z, c.actor.rotation.y)?).abs();
        let crouched = matches!(player.state, State::Crouched | State::Wall);
        let ranges = &mut frame.observation.ranges;
        let forget = if crouched {
            ranges.crouched.forget
        } else {
            ranges.standing.forget
        };
        if frame.script != 0 || frame.observation.ignore_player {
            *c.visibility = Visibility::Hidden;
        } else if matches!(player.action, 0x804 | 0x400) || (player.action == 0x702 && delta.y > 0)
        {
            *c.visibility = Visibility::Unchanged;
        } else if (player.state == State::Ledge && delta.y >= 0)
            || distance.wrapping_mul(3) / 5 <= delta.y.wrapping_abs()
            || delta.y.wrapping_abs() > 4999
            || scale_range(forget, frame.observation.range_percent) <= distance
            || c.facing >= 700
            || (crouched && c.facing >= 350)
            || (player.action == 0x536 && c.actor.state != State::Swimming)
        {
            *c.visibility = Visibility::Hidden;
        } else {
            ranges.standing.notice = i32::from(frame.observation.range);
            ranges.crouched.notice = ranges.standing.notice * 7 / 10;
            let thresholds = if crouched {
                &ranges.crouched
            } else {
                &ranges.standing
            };
            if distance < scale_range(thresholds.notice, frame.observation.range_percent) {
                let eye = c.actor.position.wrapping_add(Position::new(
                    0,
                    400 - i32::from(c.actor.height),
                    0,
                ));
                let mut ray = delta;
                ray.y = ray
                    .y
                    .wrapping_add(i32::from(c.actor.height) - i32::from(player.height));
                if player.state == State::Crouched {
                    ray.y = ray.y.wrapping_add(i32::from(player.height >> 1));
                }
                frame.cursor.current = None;
                if terrain::segment(frame.terrain, frame.cursor, eye, ray, math)?.is_some() {
                    *c.visibility = Visibility::Hidden;
                    return Ok(Outcome::Continue(Decision::default()));
                }
                c.sight.destination = player.position;
                if distance < scale_range(thresholds.recognize, frame.observation.range_percent) {
                    *c.visibility = Visibility::Recognized;
                    return Ok(Outcome::ResumePatrol);
                }
            }
        }
    }
    let delta = bait.wrapping_sub(c.actor.position);
    let distance = c.resources.math.length(Position::new(delta.x, 0, delta.z));
    Ok(if distance < 300 {
        Outcome::Continue(Decision::default())
    } else if distance < 4000 && delta.y.wrapping_abs() > 2000 {
        Outcome::LookAround
    } else {
        Outcome::Continue(c.steer_to(delta.x, delta.z)?)
    })
}
