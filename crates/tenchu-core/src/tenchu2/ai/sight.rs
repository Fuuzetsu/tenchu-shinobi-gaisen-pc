use super::direction;
use crate::{
    Result,
    math::{Math, Position},
    tenchu2::{actor::State, terrain},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Unchanged,
    Hidden,
    Unnoticed,
    Recognized,
    Suspicious,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thresholds {
    pub notice: i32,
    pub recognize: i32,
    pub forget: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ranges {
    pub standing: Thresholds,
    pub crouched: Thresholds,
}

impl Ranges {
    pub fn new(distance: i32) -> Self {
        let standing = Thresholds {
            notice: distance.wrapping_sub(4000),
            recognize: distance >> 1,
            forget: distance.wrapping_add(2000),
        };
        Self {
            crouched: Thresholds {
                notice: standing.notice.wrapping_mul(7) / 10,
                recognize: standing.recognize.wrapping_mul(7) / 10,
                forget: standing.forget.wrapping_mul(7) / 10,
            },
            standing,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Body {
    pub position: Position,
    pub yaw: i16,
    pub height: i16,
    pub state: State,
    pub action: i16,
    pub in_water: bool,
}

impl From<&crate::tenchu2::actor::Actor> for Body {
    fn from(actor: &crate::tenchu2::actor::Actor) -> Self {
        Self {
            position: actor.position,
            yaw: actor.rotation.y,
            height: actor.height,
            state: actor.state,
            action: actor.motion.action,
            in_water: actor.flags & 0x2000 != 0,
        }
    }
}

pub struct Observer {
    pub body: Body,
    pub kind: i16,
    pub slot: i16,
    pub weapon: i16,
    pub range: i16,
}

pub struct Context<'a> {
    pub tick: u32,
    pub player_position: Position,
    pub player_yaw: i16,
    pub script_mode: u16,
    pub terrain: &'a Collision,
    pub math: &'a Math,
    pub near_object: &'a mut dyn FnMut() -> bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sight {
    pub distance: i32,
    pub angle: i16,
    pub player_angle: i16,
    pub destination: Position,
}

pub struct Ray {
    pub eye: Position,
    pub target: Position,
    pub obstruction: Option<Position>,
}

pub struct Observation {
    pub visibility: Visibility,
    pub reason: &'static str,
    pub ray: Option<Ray>,
}

impl Observation {
    fn new(visibility: Visibility, reason: &'static str) -> Self {
        Self {
            visibility,
            reason,
            ray: None,
        }
    }
}

impl Sight {
    pub fn check(
        &mut self,
        observer: &Observer,
        target: Option<&Body>,
        world: &mut Context<'_>,
        ranges: &mut Ranges,
        cursor: &mut terrain::Cursor,
    ) -> Result<Visibility> {
        self.observe(observer, target, world, ranges, cursor)
            .map(|query| query.visibility)
    }

    pub fn observe(
        &mut self,
        observer: &Observer,
        target: Option<&Body>,
        world: &mut Context<'_>,
        ranges: &mut Ranges,
        cursor: &mut terrain::Cursor,
    ) -> Result<Observation> {
        let Some(target) = target else {
            return Ok(Observation::new(Visibility::Unchanged, "No target"));
        };
        let from = observer.body.position;
        let delta = target.position.wrapping_sub(from);
        self.distance = if delta.x.wrapping_abs() < 32767 && delta.z.wrapping_abs() < 32767 {
            world.math.length(delta)
        } else {
            i32::MAX
        };
        self.angle = direction(world.math, delta.x, delta.z, observer.body.yaw)?;
        self.player_angle = direction(
            world.math,
            from.x.wrapping_sub(world.player_position.x),
            from.z.wrapping_sub(world.player_position.z),
            world.player_yaw,
        )?;
        if world.tick.wrapping_add(observer.slot as u32) & 31 != 0
            || observer.body.state == State::Ledge
            || matches!(target.action, 0x804 | 0x400)
            || (target.action == 0x702 && delta.y > 0)
        {
            return Ok(Observation::new(
                Visibility::Unchanged,
                "Not a perception update / excluded motion",
            ));
        }
        let crouched = matches!(target.state, State::Crouched | State::Wall);
        if target.state == State::Ledge && delta.y >= 0 {
            return Ok(Observation::new(
                Visibility::Hidden,
                "Target hanging above observer",
            ));
        }
        let archer = observer.weapon >> 12 == 4;
        if !archer {
            let vertical_limit = self.distance.wrapping_mul(3) / 5;
            let height = delta.y.wrapping_abs();
            if vertical_limit <= height || (height <= vertical_limit && delta.y < -3500) {
                return Ok(Observation::new(
                    Visibility::Hidden,
                    "Height outside sight range",
                ));
            }
        }
        let forget = if crouched {
            ranges.crouched.forget
        } else {
            ranges.standing.forget
        };
        if forget <= self.distance {
            return Ok(Observation::new(
                Visibility::Hidden,
                "Beyond forget distance",
            ));
        }
        let angle = i32::from(self.angle).abs();
        if angle >= 700 || (crouched && angle >= 350) {
            return Ok(Observation::new(
                Visibility::Unnoticed,
                "Outside sight angle",
            ));
        }
        if target.action == 0x536 && observer.body.state != State::Swimming {
            return Ok(Observation::new(Visibility::Hidden, "Target submerged"));
        }
        ranges.standing.notice = i32::from(observer.range);
        ranges.crouched.notice = ranges.standing.notice * 7 / 10;
        if crouched && (world.near_object)() {
            ranges.standing.notice >>= 1;
            ranges.crouched.notice >>= 1;
        }
        let thresholds = if crouched {
            &ranges.crouched
        } else {
            &ranges.standing
        };
        if thresholds.notice <= self.distance {
            return Ok(Observation::new(
                Visibility::Unnoticed,
                "Beyond notice distance",
            ));
        }
        let mut eye = from;
        eye.y = eye.y.wrapping_sub(i32::from(observer.body.height));
        let target_height = if target.in_water {
            if target.state == State::Swimming && !matches!(target.action, 0x500 | 0x501) {
                0
            } else {
                1300
            }
        } else if target.state == State::Crouched {
            target.height >> 2
        } else {
            target.height
        };
        let mut delta = target.position.wrapping_sub(eye);
        delta.y = delta.y.wrapping_sub(i32::from(target_height));
        cursor.current = None;
        let ray = Ray {
            eye,
            target: eye.wrapping_add(delta),
            obstruction: terrain::segment(world.terrain, cursor, eye, delta, world.math)?,
        };
        if ray.obstruction.is_some() {
            return Ok(Observation {
                visibility: Visibility::Hidden,
                reason: "Terrain blocks sight",
                ray: Some(ray),
            });
        }
        if world.script_mode < 3 && (world.script_mode < 2 || observer.kind & 0xe0 == 0xe0) {
            self.destination = target.position;
        }
        let (distance, recognize) = if archer {
            (
                world.math.length(Position::new(delta.x, 0, delta.z)),
                thresholds.recognize.wrapping_sub(1500),
            )
        } else {
            (self.distance, thresholds.recognize)
        };
        let visibility = if distance < recognize {
            Visibility::Recognized
        } else {
            Visibility::Suspicious
        };
        Ok(Observation {
            visibility,
            reason: "Clear sight ray",
            ray: Some(ray),
        })
    }
}
