use super::{
    actor::{Actor, Resources, State, Velocity},
    terrain::{self, Cursor, Options, Probe, Query},
};
use crate::{Result, math::Position};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::{Collision, Rules};

pub use super::actor::flags::{
    BLOCKED, CEILING, FALLING, HIT, IN_WATER, LANDED, TOUCHING, TOUCHING_PLAYER, UNEVEN,
};

mod contacts;
pub use contacts::ContactActor;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Depth {
    Shallow,
    Wading,
    Swimming,
    Submerged,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Water {
    pub depth: Option<Depth>,
    pub surface: i32,
    pub bottom: i32,
}

pub struct Terrain<'a> {
    pub map: &'a mut Collision,
    pub rules: &'a Rules,
    pub cursor: &'a mut Cursor,
    pub scratch: &'a mut Probe,
    pub water: &'a mut Water,
    pub options: Options,
}

impl Terrain<'_> {
    pub(crate) fn sample(&mut self, position: Position, width: i16) -> Result<u16> {
        self.scratch.material = 0;
        self.scratch.blocked = 0;
        self.scratch.sample(
            self.map,
            self.cursor,
            self.rules,
            position,
            width,
            self.options,
        )?;
        Ok(self.scratch.blocked)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallDeath {
    Terrain,
    LongFall,
}

impl Actor {
    pub fn move_on_terrain(
        &mut self,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        is_player: bool,
        mut root_position: impl FnMut(&Actor) -> Result<Position>,
        mut surface_effect: impl FnMut(&Actor, &mut Terrain<'_>) -> Result<i16>,
    ) -> Result<Option<FallDeath>> {
        let at_head = |mut point: Position, height: i16| {
            point.y = point.y.wrapping_sub(i32::from(height));
            point
        };
        if terrain.sample(at_head(self.position, self.height), self.width)? == 0 {
            self.previous = self.position;
        }
        self.position = self.position.wrapping_add(self.velocity.map(i32::from));
        self.rotation.y &= 4095;
        self.hit = None;
        self.last_vertical_velocity = self.velocity.y;
        self.flags &= 0xff30_0fff;
        let root = if self.state == State::Attack {
            Some(root_position(self)?)
        } else {
            None
        };
        let mut query = root.unwrap_or(self.position);
        query.y = self.position.y.wrapping_sub(i32::from(self.height));
        terrain.cursor.current = None;
        self.terrain.sample(
            terrain.map,
            terrain.cursor,
            terrain.rules,
            query,
            self.width,
            terrain.options,
        )?;
        let death = if self.health != 0
            && self.motion.action == 0x702
            && self.terrain.height / 1000 == 25
            && self.velocity.y >= 380
        {
            Some(FallDeath::LongFall)
        } else if self.terrain.material & 0xf00 == 0x400
            && surface_effect(self, terrain)? == 0x13
            && self.terrain.height < 20000
            && self.health != 0
        {
            Some(FallDeath::Terrain)
        } else {
            None
        };
        let death = death.filter(|_| !self.invulnerable);
        if death.is_some() {
            self.health = 0;
            self.previous = Position::new(query.x, self.terrain.floor.unwrap_or(i32::MIN), query.z);
        }
        let material = self.terrain.material & 0xf00;
        terrain.water.depth = None;
        if matches!(material, 0x200 | 0x300) {
            terrain.water.surface = self.terrain.floor.unwrap_or(i32::MIN);
            if material == 0x300 {
                terrain.water.depth = Some(Depth::Submerged);
                terrain.water.bottom = terrain.water.surface;
                self.flags |= IN_WATER;
            } else if self.terrain.height <= i32::from(self.height) {
                terrain.water.bottom = terrain::level(
                    terrain.map,
                    terrain.cursor,
                    Position::new(query.x, terrain.water.surface.wrapping_add(10), query.z),
                    Query::Uncached,
                )?
                .unwrap_or(terrain.water.surface);
                let depth =
                    (terrain.water.bottom as i16).wrapping_sub(terrain.water.surface as i16);
                let depth = if depth < 501 {
                    Depth::Shallow
                } else if depth <= 1000 {
                    Depth::Wading
                } else {
                    Depth::Swimming
                };
                terrain.water.depth = Some(depth);
                if matches!(depth, Depth::Shallow | Depth::Wading) {
                    self.terrain.floor =
                        (terrain.water.bottom != i32::MIN).then_some(terrain.water.bottom);
                    self.terrain.height = terrain.water.bottom.wrapping_sub(query.y);
                }
                self.flags |= IN_WATER;
            }
        }
        match material {
            0x100 => {
                self.flags |= CEILING;
                let surface = terrain
                    .cursor
                    .last
                    .ok_or("Missing Tenchu 2 ceiling surface")?;
                let node = &terrain.map.cells[surface.cell].nodes[surface.node];
                self.position.y = i32::from(node.y) + i32::from(self.height) / 2;
                if self.velocity.y < 0 {
                    self.velocity.y = 20;
                }
            }
            0x300 => {
                let floor = self.terrain.floor.unwrap_or(i32::MIN);
                if floor.wrapping_sub(self.position.y) <= i32::from(self.height) {
                    self.position.y = floor.wrapping_sub(i32::from(self.height));
                    self.velocity.y = 0;
                }
            }
            _ if self.flags & 4 == 0 && self.terrain.height > i32::from(self.height) => {
                if self.flags & IN_WATER == 0 {
                    self.flags |= FALLING;
                }
                if self.velocity.y < 400 {
                    self.velocity.y = self.velocity.y.wrapping_add(20);
                }
            }
            _ if self.velocity.y > 0
                || (self.velocity.y == 0 && self.terrain.height < i32::from(self.height)) =>
            {
                if self.velocity.y > 0 {
                    self.flags |= LANDED;
                }
                if let Some(floor) = self.terrain.floor {
                    self.position.y = floor;
                }
                self.velocity.y = 0;
            }
            _ => {}
        }
        if self.terrain.lower | self.terrain.higher != 0 {
            self.flags |= UNEVEN;
            if self.terrain.height < i32::from(self.height) {
                self.position.y = self.terrain.floor.unwrap_or(i32::MIN);
            }
        }
        if self.terrain.blocked != 0 {
            self.flags |= BLOCKED;
            if self.terrain.floor.is_none() {
                self.position = self.previous;
                let blocked = terrain.sample(at_head(self.position, self.height), 200)?;
                let [x, z] = terrain.rules.escape[usize::from(blocked)];
                self.position.x = self.position.x.wrapping_add(i32::from(x) * 64);
                self.position.z = self.position.z.wrapping_add(i32::from(z) * 64);
            } else {
                let y = self.position.y.wrapping_sub(i32::from(self.height));
                let blocked = terrain.sample(
                    Position::new(self.position.x, y, self.previous.z),
                    self.width,
                )?;
                let dx = i32::from(terrain.rules.escape[usize::from(blocked)][0]);
                if i32::from(self.velocity.x) * dx < 0 {
                    self.position.x = self.previous.x;
                }
                if self.velocity.x == 0 && self.velocity.z == 0 {
                    self.position.x = self.position.x.wrapping_add(dx * 256);
                }
                let blocked = terrain.sample(
                    Position::new(self.previous.x, y, self.position.z),
                    self.width,
                )?;
                let dz = i32::from(terrain.rules.escape[usize::from(blocked)][1]);
                if i32::from(self.velocity.z) * dz < 0 {
                    self.position.z = self.previous.z;
                }
                if self.velocity.x == 0 && self.velocity.z == 0 {
                    self.position.z = self.position.z.wrapping_add(dz * 256);
                }
                if let Some(angle) = terrain.rules.wall_angles[usize::from(self.terrain.blocked)]
                    && self.can_turn_at_wall()
                {
                    let mut delta = i32::from(self.rotation.y.wrapping_sub(angle));
                    if delta.abs() > 2047 {
                        delta -= delta.signum() * 4096;
                    }
                    if delta.abs() < 1800 || !is_player || self.terrain.height != 0 {
                        let slow = matches!(self.state, State::Crouched | State::Jumping);
                        let turn = if slow { self.turn / 4 } else { self.turn };
                        self.rotation.y = self.rotation.y.wrapping_add(if delta > 0 {
                            turn.wrapping_neg()
                        } else {
                            turn
                        });
                        self.update_velocity(Velocity::Motion, resources)?;
                        if slow {
                            self.velocity.x >>= 1;
                            self.velocity.z >>= 1;
                        }
                    }
                }
                let point = if let Some(root) = root {
                    root
                } else {
                    root_position(self)?
                };
                if terrain.sample(at_head(point, self.height), self.width)? != 0
                    || self.state == State::Attack
                {
                    self.motion.mask &= 0x7fff_ffff;
                }
            }
            if self.flags & CEILING == 0 && !self.can_turn_at_wall() {
                self.velocity.x = 0;
                self.velocity.z = 0;
            }
            if self.velocity.y < 0 {
                self.velocity.y = 0;
            }
        }
        Ok(death)
    }

    fn can_turn_at_wall(&self) -> bool {
        matches!(
            self.state,
            State::Normal | State::Ready | State::Crouched | State::Jumping
        )
    }
}
