use crate::{
    Result,
    math::{Math, Position},
    tenchu2::terrain::{self, Cursor, Query},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Flight {
    pub position: Position,
    pub velocity: Position,
    pub gravity: i32,
    pub speed: i32,
    pub floor: i32,
    pub submerged: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Ballistic,
    Directed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Air,
    Ground,
    Outside,
}

impl Flight {
    pub fn settle(
        &mut self,
        falling: bool,
        model: crate::scene::ModelId,
        transform: &mut crate::math::Matrix,
        context: &mut super::Context<'_>,
    ) -> Result<bool> {
        let mut next = falling;
        if !falling {
            let probe = self.position.wrapping_sub(Position::new(0, 300, 0));
            if let Some(floor) =
                terrain::level(context.map, context.cursor, probe, Query::Uncached)?
            {
                self.floor = floor;
                next = floor != self.position.y;
            }
        }
        transform.translation = self.position;
        context.scene.update(model, *transform);
        if falling {
            let blocked = context.collision.contacts(model).any(|hit| {
                context
                    .collision
                    .slot(hit.index)
                    .is_some_and(|slot| slot.flags & crate::tenchu2::collision::BODY == 0)
            });
            if blocked {
                next = false;
            } else {
                self.position.y = self.position.y.wrapping_add(180);
                if self.floor < self.position.y {
                    self.position.y = self.floor;
                    next = false;
                }
            }
        }
        Ok(next)
    }

    pub fn thrown(
        mut origin: Position,
        fallback: Position,
        direction: Position,
        speed: i32,
        map: &Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Self> {
        let normal = math.normalize_vector(direction);
        if terrain::level(map, cursor, origin, Query::Surface)?.is_none() {
            origin = fallback;
        }
        Ok(Self {
            position: origin,
            velocity: normal.map(|v| v.wrapping_mul(speed) >> 12),
            speed,
            gravity: 16,
            ..Default::default()
        })
    }

    pub fn bounce(&mut self) {
        self.speed >>= 1;
        self.gravity = 16;
        self.velocity.x >>= 1;
        self.velocity.y = (self.velocity.y >> 1).wrapping_neg();
        self.velocity.z >>= 1;
    }

    pub fn advance(
        &mut self,
        motion: Motion,
        map: &Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<(Contact, Option<Position>)> {
        let start = self.position;
        let directed = motion == Motion::Directed;
        let step = if directed {
            self.velocity.map(|v| v.wrapping_mul(self.speed) >> 12)
        } else {
            self.velocity
        };
        self.position = self.position.wrapping_add(step);
        if directed {
            self.position.y = self.position.y.wrapping_add(self.gravity);
        }
        let height = terrain::level(map, cursor, self.position, Query::Uncached)?;
        let mut crossed_floor = false;
        let mut water = false;
        if self.speed == 0 {
            if let Some(height) = height {
                self.floor = height;
            }
        } else {
            if self.speed > 500
                && let Some(hit) =
                    terrain::traverse(map, cursor, start, step.map(|v| v as i16), 0, math)?
            {
                self.position = hit;
                self.speed = 0;
                if !directed {
                    self.velocity.x = 0;
                    self.velocity.z = 0;
                }
            }
            if let Some(height) = height {
                self.floor = height;
                // The fast-projectile traversal can replace the current surface.
                let surface = cursor
                    .current
                    .ok_or("Missing Tenchu 2 projectile surface")?;
                match map.cells[surface.cell].nodes[surface.node].attributes & 0xf00 {
                    0x200 => {
                        water = true;
                        let mut below = self.position;
                        below.y = height.wrapping_add(if directed { 300 } else { 100 });
                        if let Some(floor) = terrain::level(map, cursor, below, Query::Uncached)? {
                            self.floor = floor;
                        }
                    }
                    0x300 => water = true,
                    _ => {}
                }
            } else if directed {
                let speed = self.speed;
                self.speed = 0;
                self.position.x = self
                    .position
                    .x
                    .wrapping_sub(self.velocity.x.wrapping_mul(speed) >> 12);
                self.position.z = self
                    .position
                    .z
                    .wrapping_sub(self.velocity.z.wrapping_mul(speed) >> 12);
                crossed_floor = self.floor < self.position.y;
                self.position.y = self.position.y.wrapping_sub(self.gravity);
            } else {
                self.position.x = self.position.x.wrapping_sub(self.velocity.x);
                self.position.z = self.position.z.wrapping_sub(self.velocity.z);
                crossed_floor = self.floor <= self.position.y;
                if !crossed_floor {
                    self.speed = 0;
                    self.velocity.x = 0;
                    self.velocity.z = 0;
                }
                self.position.y = self.position.y.wrapping_sub(self.velocity.y);
            }
        }
        if !directed {
            self.velocity.y = self.velocity.y.wrapping_add(self.gravity);
        }
        if crossed_floor || self.floor <= self.position.y {
            self.position.y = self.floor;
            if !directed {
                self.gravity = 0;
            }
            return Ok((Contact::Ground, None));
        }
        let splash = if water && !self.submerged && height.is_some_and(|y| y <= self.position.y) {
            self.submerged = true;
            Some(if directed {
                let offset = self.velocity.x.wrapping_mul(self.speed) >> 12;
                // The original uses the X component for both splash axes.
                self.position.wrapping_add(Position::new(offset, 0, offset))
            } else {
                self.position
                    .wrapping_add(Position::new(self.velocity.x, 0, self.velocity.z))
            })
        } else {
            None
        };
        let contact = if self
            .position
            .into_iter()
            .any(|v| v.wrapping_abs() > 0x1000000)
        {
            self.gravity = 0;
            self.floor = self.position.y;
            Contact::Outside
        } else {
            Contact::Air
        };
        Ok((contact, splash))
    }
}
