use super::{JumpEffects, Request};
use crate::{
    Result,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        input::{ATTACK, DOWN, JUMP, LEFT, RIGHT, UP},
        physics::{CEILING, LANDED, Terrain, UNEVEN},
        terrain::{Query, level},
    },
};

impl Actor {
    pub fn update_jump(
        &mut self,
        request: &mut Request,
        input: u16,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        mut attack: impl FnMut(&mut Actor, &mut Request) -> Result<()>,
    ) -> Result<JumpEffects> {
        let mut effects = JumpEffects::default();
        let clip = resources
            .clips
            .get(&self.motion.clip.ok_or("No Tenchu 2 jump motion")?)
            .ok_or("Missing Tenchu 2 jump motion")?;
        let active = self.motion.loops < 1 && (self.motion.frame < 2 || self.flags & LANDED == 0);
        if active && self.flags & CEILING == 0 {
            if self.motion.frame < 0 {
                self.position.x = self.position.x.wrapping_sub(i32::from(self.velocity.x));
                self.position.z = self.position.z.wrapping_sub(i32::from(self.velocity.z));
            } else {
                let acceleration = match request.action {
                    0x606 => 10,
                    0x607 => 20,
                    _ => 15,
                };
                self.velocity.y = ((i32::from(self.motion.frame) - i32::from(clip.duration >> 1))
                    * acceleration) as i16;
            }
            if input & !self.pad.previous & JUMP != 0
                && self.velocity.y > 0
                && !matches!(request.action, 0x605 | 0x607)
            {
                let mut point = self.position;
                point.y = point.y.wrapping_sub(i32::from(self.height) / 2);
                terrain.options.ignore_steps = true;
                let sampled = terrain.sample(point, self.width.wrapping_add(300));
                terrain.options.ignore_steps = false;
                let blocked = sampled?;
                if blocked != 0
                    && self
                        .change_motion(0x605, Velocity::Keep, resources)?
                        .is_some()
                {
                    request.action = 0x605;
                    request.velocity = Velocity::Keep;
                    effects.voices.push(0xa);
                    effects.wall_jump = true;
                    let direction = terrain
                        .rules
                        .wall_angles
                        .get(usize::from(blocked))
                        .ok_or("Invalid Tenchu 2 wall direction")?;
                    if let Some(angle) = direction {
                        self.rotation.y = angle.wrapping_add(2048);
                        let [x, _, z] = resources.math.velocity(self.rotation.y, -120, 0);
                        self.velocity.x = x;
                        self.velocity.z = z;
                    } else {
                        self.velocity.x = self.velocity.x.wrapping_neg();
                        self.velocity.z = self.velocity.z.wrapping_neg();
                    }
                }
            } else if input & ATTACK != 0 && self.velocity.y > 0 && request.action != 0x607 {
                attack(self, request)?;
            } else if input & (UP | DOWN | LEFT | RIGHT) != 0 {
                let speed = 10;
                let forward = if input & DOWN != 0 {
                    -speed
                } else if input & UP != 0 {
                    speed
                } else {
                    0
                };
                let side = if input & RIGHT != 0 {
                    -speed
                } else if input & LEFT != 0 {
                    speed
                } else {
                    0
                };
                let [dx, _, dz] = resources.math.velocity(self.rotation.y, forward, side);
                let x = self.velocity.x.wrapping_add(dx);
                let z = self.velocity.z.wrapping_add(dz);
                let limit = 100;
                if i32::from(x).abs() <= limit && i32::from(z).abs() <= limit {
                    self.velocity.x = x;
                    self.velocity.z = z;
                }
            }
            if self.flags & UNEVEN != 0
                && (self.velocity.x != 0 || self.velocity.z != 0)
                && self.terrain.height < i32::from(self.height)
            {
                let mut point = self.position;
                point.y = point.y.wrapping_sub(i32::from(self.height));
                if level(terrain.map, terrain.cursor, point, Query::Base)?.is_some() {
                    let surface = terrain
                        .cursor
                        .current
                        .ok_or("Missing Tenchu 2 jump surface")?;
                    let node = &terrain.map.cells[surface.cell].nodes[surface.node];
                    let [x1, z1, x2, z2] = node.bounds.map(i32::from);
                    let gradient = |span| {
                        i32::from(node.dy)
                            .checked_div(span)
                            .ok_or_else(|| "Empty Tenchu 2 jump surface".to_string())
                    };
                    if gradient(x2 - x1)?.abs() >= 2 || gradient(z2 - z1)?.abs() >= 2 {
                        self.update_velocity(Velocity::Stop, resources)?;
                    }
                }
            }
            return Ok(effects);
        }
        if !active
            && request.action == 0x607
            && (self.motion.frame > clip.duration / 2 || self.motion.loops > 0)
        {
            self.rotation.y = self.rotation.y.wrapping_add(2048);
            if let Some(rotation) = self.motion.pose.rotation.first_mut() {
                rotation[1] = rotation[1].wrapping_add(2048);
            }
        }
        request.action = if !active && self.terrain.height <= i32::from(self.height) {
            0x703
        } else {
            0x702
        };
        request.velocity = Velocity::Keep;
        Ok(effects)
    }
}
