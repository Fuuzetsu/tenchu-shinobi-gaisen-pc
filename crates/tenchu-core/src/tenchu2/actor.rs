use super::{input::Pad, motion::Motion, terrain::Probe};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::{
    model::Part,
    tenchu2::{Edition, commands::Commands, motions::Clip, registry::Registry, weapons::Weapons},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Resources {
    pub registry: Registry,
    pub clips: BTreeMap<i16, Clip>,
    pub weapons: Weapons,
    pub commands: Commands,
    pub math: Math,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Normal,
    Ready,
    Crouched,
    Wall,
    Ledge,
    Swimming,
    Jumping,
    Special,
    Item,
    Attack,
    Hurt,
    Dead,
}

impl State {
    pub fn from_action(action: i16) -> Result<Self> {
        Ok(match action >> 8 {
            0 => Self::Normal,
            1 => Self::Ready,
            2 => Self::Crouched,
            3 => Self::Wall,
            4 => Self::Ledge,
            5 => Self::Swimming,
            6 => Self::Jumping,
            7 => Self::Special,
            8 => Self::Item,
            9 => Self::Attack,
            10 => Self::Hurt,
            11 => Self::Dead,
            _ => return Err(format!("Unknown Tenchu 2 action state {action:#x}")),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Velocity {
    Stop,
    Keep,
    Motion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub action: i16,
    pub changed: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attack {
    pub index: Option<usize>,
    pub contacts: u32,
    pub trails: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub kind: i16,
    pub state: State,
    pub flags: u32,
    pub health: i16,
    pub width: i16,
    pub height: i16,
    pub turn: i16,
    pub skill: i16,
    pub terrain: Probe,
    pub position: Position,
    pub previous: Position,
    pub rotation: Vector3<i16>,
    pub velocity: Vector3<i16>,
    pub last_vertical_velocity: i16,
    pub hit: Option<usize>,
    pub motion: Motion,
    pub pad: Pad,
    pub attack: Attack,
}

impl Actor {
    pub fn new(kind: i16, parts: &[Part], resources: &Resources) -> Result<Self> {
        let definition = resources
            .registry
            .actor(kind)
            .ok_or("Unknown Tenchu 2 actor")?;
        let mut motion = Motion::new(parts)?;
        motion
            .select(0, kind, &resources.registry, &resources.clips)?
            .ok_or("Missing Tenchu 2 idle motion")?;
        let (width, height) = dimensions(kind, parts)?;
        Ok(Self {
            kind,
            state: State::Normal,
            flags: 0,
            health: if kind == 0x68 {
                i16::MAX
            } else {
                definition.health
            },
            width,
            height,
            turn: definition.turn,
            skill: definition.skill,
            terrain: Probe::default(),
            position: Position::default(),
            previous: Position::default(),
            rotation: Vector3::default(),
            velocity: Vector3::default(),
            last_vertical_velocity: 0,
            hit: None,
            motion,
            pad: Pad::default(),
            attack: Attack::default(),
        })
    }

    pub fn change_motion(
        &mut self,
        mut action: i16,
        velocity: Velocity,
        resources: &Resources,
    ) -> Result<Option<Change>> {
        let changed = self.motion.action != action;
        if changed {
            if self.state == State::Attack && self.motion.loops < 1 && self.weapon_drawn() {
                self.attack.contacts = 0;
                self.attack.trails = 0;
            }
            if self
                .motion
                .select(action, self.kind, &resources.registry, &resources.clips)?
                .is_none()
            {
                if action != 0x110 {
                    return Ok(None);
                }
                action = if self.weapon_drawn() { 0x100 } else { 0 };
                let selected =
                    self.motion
                        .select(action, self.kind, &resources.registry, &resources.clips)?;
                if selected.is_none() && !self.weapon_drawn() {
                    return Ok(None);
                }
            }
            self.state = State::from_action(action)?;
        }
        self.update_velocity(velocity, resources)?;
        Ok(Some(Change { action, changed }))
    }

    pub fn overlay_motion(&mut self, action: i16, resources: &Resources) -> Result<Option<i16>> {
        if self.motion.frame >= 4
            || self
                .motion
                .overlay(
                    action,
                    0xe3e,
                    self.kind,
                    &resources.registry,
                    &resources.clips,
                )?
                .is_none()
        {
            return Ok(None);
        }
        self.motion.action = action;
        self.motion.loops = 0;
        self.state = State::from_action(action)?;
        Ok(Some(action))
    }

    pub fn update_velocity(&mut self, mode: Velocity, resources: &Resources) -> Result<()> {
        match mode {
            Velocity::Stop => {
                self.velocity.x = 0;
                self.velocity.z = 0;
            }
            Velocity::Keep => {}
            Velocity::Motion => {
                let clip = resources
                    .clips
                    .get(&self.motion.clip.ok_or("No Tenchu 2 motion")?)
                    .ok_or("Missing Tenchu 2 motion")?;
                let speed = |v: i16| {
                    if resources.registry.edition == Edition::Europe {
                        (i32::from(v) * 6 / 5) as i16
                    } else {
                        v
                    }
                };
                let [x, _, z] = resources.math.velocity(
                    self.rotation.y,
                    speed(clip.forward_speed),
                    speed(clip.side_speed),
                );
                self.velocity.x = x;
                self.velocity.z = z;
            }
        }
        Ok(())
    }

    pub fn commit_root(&mut self, math: &Math) {
        let root = Matrix {
            rotation: math.rotate_yxz(self.rotation.to_array()),
            translation: self.position,
        };
        let point = root
            .apply_large(self.motion.pose.translation[0])
            .wrapping_add(self.position);
        self.position.x = point.x;
        self.position.z = point.z;
    }

    pub fn normal_motion(&mut self, math: &Math) -> i16 {
        if matches!(self.state, State::Normal | State::Ready | State::Attack) {
            self.commit_root(math);
        }
        let action = if self.health == 0 {
            0xb00
        } else if self.pad.held & super::input::CROUCH != 0
            && self.terrain.height <= i32::from(self.height)
        {
            0x200
        } else if self.weapon_drawn() {
            0x100
        } else {
            0
        };
        self.motion.mask = u32::MAX;
        self.rotation.x = 0;
        self.rotation.z = 0;
        action
    }

    pub fn weapon_drawn(&self) -> bool {
        self.flags & 0x40 != 0
    }
}

pub fn dimensions(kind: i16, parts: &[Part]) -> Result<(i16, i16)> {
    let mut bounds = [0i16; 3];
    for part in parts {
        let mut point = part.position;
        let mut parent = part.parent;
        while parent >= 0 {
            let part = parts
                .iter()
                .find(|p| p.child == parent)
                .ok_or("Missing Tenchu 2 joint")?;
            point = std::array::from_fn(|axis| point[axis].wrapping_add(part.position[axis]));
            parent = part.parent;
        }
        bounds[0] = bounds[0].max(point[0].wrapping_abs());
        bounds[2] = bounds[2].max(point[2].wrapping_abs());
        if i32::from(bounds[1]) < -i32::from(point[1]) {
            bounds[1] = point[1].wrapping_neg();
        }
    }
    let diameter = |bounds: [i16; 3]| bounds[0].wrapping_add(bounds[2]).wrapping_mul(2);
    Ok(match kind {
        0x71 => {
            let size = diameter(bounds);
            (size, size)
        }
        0x75 | 0xe5 => (bounds[1], bounds[1]),
        0x79 | 0x7a => (200, 1000),
        _ => {
            if matches!(kind, 0..=2 | 0xf6 | 0xf8..=0xf9) {
                bounds[0] = 125;
                bounds[2] = 125;
            }
            (diameter(bounds).min(450), bounds[1].wrapping_add(200))
        }
    })
}
