use super::*;

impl Camera {
    pub fn advance(
        &mut self,
        subject: &Subject<'_>,
        c: &mut Context<'_>,
        random: &mut Random,
    ) -> Result<()> {
        let actor = subject.actor;
        let action = actor.motion.action;
        let drawn = usize::from(actor.flags & 0x40 != 0);
        if actor.pad.held & 0x400 != 0
            && !matches!(self.mode, 20 | 19 | 7 | 16 | 22)
            && !self.snap
            && action as u16 & 0xff00 != 0xa00
        {
            self.mode = 6;
        }
        let delta = match self.mode {
            6 | 20 => {
                if self.mode != self.previous
                    && let Some(pitch) = self.initial_pitch
                {
                    self.direction[0] = pitch.clamp(-796, 910);
                }
                match self.previous {
                    12 => self.direction[1] = -1024,
                    11 => self.direction[1] = 1024,
                    _ if self.mode == 6 && self.previous != 6 => self.direction[1] = 0,
                    _ => {}
                }
                let delta = self.first_person(actor, c)?;
                match action {
                    0x702 => self.mode = 14,
                    _ if action as u16 & 0xff00 == 0xa00 => self.mode = 7,
                    _ if action as u16 & 0xff00 == 0xb00 => {
                        self.mode = 16;
                        self.death_style = 0;
                        self.counter = 0;
                    }
                    _ if actor.pad.held & 0x400 == 0 && self.mode == 6 => self.mode = 0,
                    _ => {}
                }
                delta
            }
            21 => {
                self.snap = true;
                self.mode = 17;
                self.held_action = -1;
                self.counter = 35;
                let mut rotation = actor.rotation;
                rotation.y = rotation.y.wrapping_add(2048);
                self.solve(subject, rotation, c.tables.poses[2][drawn], c)?
            }
            22 => {
                self.underwater = false;
                [Position::ZERO; 2]
            }
            19 => {
                if actor.health > 0 && (action == 0x702 || action >= 0xa21) {
                    self.mode = 0;
                }
                [Position::ZERO; 2]
            }
            16 => {
                let mut rotation = actor.rotation;
                let mut pose = c.tables.poses[if self.death_style == 0 { 7 } else { 16 }][drawn];
                if actor.health < 1 && actor.motion.frame == 0 {
                    if self.death_style == 0 {
                        self.roll = self.roll.wrapping_add(4096);
                        self.counter = self.counter.wrapping_add(1);
                        pose[3][1] = pose[3][1].wrapping_add((self.counter as i16).wrapping_mul(8));
                        pose[3][2] = pose[3][2].wrapping_sub((self.counter as i16).wrapping_mul(8));
                    } else {
                        self.orbit = self.orbit.wrapping_add(8);
                        rotation.y = rotation.y.wrapping_add(self.orbit as i16);
                    }
                }
                self.solve(subject, rotation, pose, c)?
            }
            17 => {
                if self.release_held(subject, c)? {
                    return Ok(());
                }
                let mut rotation = actor.rotation;
                if !matches!(action, 0x913 | 0x607 | 0x220 | 0x532) {
                    rotation.y = rotation.y.wrapping_add(2048);
                }
                self.solve(subject, rotation, c.tables.poses[17][drawn], c)?
            }
            _ if action as u16 & 0xfff0 == 0x920 => {
                if self.mode != 15 {
                    self.mode = 15;
                    self.snap = true;
                    [self.eye, self.look] = self.stealth(subject, true, c, random)?;
                    self.update_view(c.math)?;
                    self.previous = self.mode;
                    return Ok(());
                }
                [Position::ZERO; 2]
            }
            _ if action as u16 & 0xff00 == 0x500 => {
                self.direction = [0; 2];
                if action == 0x536 {
                    self.mode = 13;
                    self.solve(subject, actor.rotation, c.tables.poses[15][0], c)?
                } else if action < 0x502 {
                    self.mode = 0;
                    self.solve(subject, actor.rotation, c.tables.poses[0][drawn], c)?
                } else {
                    self.mode = 13;
                    let delta = self.solve(
                        subject,
                        actor.rotation,
                        c.tables.poses[13][usize::from(action >= 0x521)],
                        c,
                    )?;
                    self.snap = false;
                    if action == 0x532 {
                        self.held_action = action;
                        self.mode = 17;
                        self.counter = 1;
                    }
                    delta
                }
            }
            _ => {
                self.direction = [0; 2];
                self.follow_mode(actor, drawn, c)?;
                let pose = c
                    .tables
                    .poses
                    .get((self.mode as u16 & 31) as usize)
                    .ok_or("Invalid Tenchu 2 camera mode")?[drawn];
                let delta = self.solve(subject, actor.rotation, pose, c)?;
                self.snap = false;
                delta
            }
        };
        self.eye = self.eye.wrapping_add(delta[0]);
        self.look = self.look.wrapping_add(delta[1]);
        self.update_view(c.math)?;
        self.previous = self.mode;
        terrain::level(c.map, c.cursor, self.eye, Query::Uncached)?;
        self.underwater = c.cursor.material & 0xf00 == 0x300
            && action as u16 & 0xff00 == 0x500
            && (action == 0x510 || action >= 0x512);
        Ok(())
    }

    fn follow_mode(&mut self, actor: &Actor, drawn: usize, c: &mut Context<'_>) -> Result<()> {
        let action = actor.motion.action;
        self.mode = match action as u16 & 0xff00 {
            0 => {
                self.orbit = 0;
                *c.tables
                    .normal
                    .get((action as u16 & 15) as usize)
                    .ok_or("Invalid Tenchu 2 camera idle action")?
            }
            0x100 => {
                if self.previous == 15 {
                    self.snap = true;
                }
                c.tables.ready[(action as u16 & 31) as usize]
            }
            0x200 => {
                if action == 0x220 {
                    self.held_action = action;
                    self.counter = if drawn == 0 { 0 } else { 3 };
                    17
                } else {
                    4
                }
            }
            0x300 => wall_mode(actor, c)?,
            0x400 => 8,
            0x600 => {
                if action == 0x607 {
                    self.held_action = action;
                    self.held_yaw = actor.rotation.y;
                    self.counter = 18;
                    17
                } else if action == 0x602 {
                    2
                } else {
                    0
                }
            }
            0x700 if action == 0x702 && actor.motion.frame < -9 => 14,
            0x900 if action == 0x913 => {
                self.held_action = action;
                self.counter = 3;
                17
            }
            0xa00 if actor.health < 1 => {
                self.orbit = 0;
                self.counter = 0;
                self.death_style = 0;
                16
            }
            0xa00 if action >= 0xa21 => 7,
            0xb00 => {
                let mut point = actor.position;
                point.y = point.y.wrapping_sub(500);
                terrain::level(c.map, c.cursor, point, Query::Uncached)?;
                self.death_style = i16::from(c.cursor.current.is_some_and(|id| {
                    let [x1, z1, x2, z2] = c.map.cells[id.cell].nodes[id.node]
                        .bounds
                        .map(|v| i32::from(v) * 10);
                    point.x - 2000 > x1
                        && point.x + 2000 < x2
                        && point.z - 2000 > z1
                        && point.z + 2000 < z2
                }));
                self.roll = 0;
                self.orbit = 0;
                self.counter = 0;
                16
            }
            _ => 0,
        };
        Ok(())
    }

    fn release_held(&mut self, subject: &Subject<'_>, c: &mut Context<'_>) -> Result<bool> {
        let actor = subject.actor;
        let action = actor.motion.action;
        if self.held_action == action {
            return Ok(false);
        }
        if self.held_action == 0x607 && self.held_yaw == actor.rotation.y {
            self.mode = 0;
            self.counter = 0;
        }
        self.held_action = -1;
        if action as u16 & 0xff00 == 0x900 {
            self.counter = 1;
            self.held_action = action;
        } else if !matches!(
            action,
            0 | 0x100 | 0x110 | 0x702 | 0x703 | 0x605 | 0x80d | 0x200
        ) {
            self.counter = 0;
        }
        if action != 0x702 {
            self.counter = self.counter.wrapping_sub(1);
        }
        if self.counter >= 0 && !matches!(action, 0x607 | 0x220 | 0x913) {
            return Ok(false);
        }
        if self.mode == 0 {
            return Ok(true);
        }
        self.mode = 0;
        self.held_action = -1;
        let target = Position::new(
            actor.position.x.wrapping_mul(2).wrapping_sub(self.eye.x),
            self.eye.y,
            actor.position.z.wrapping_mul(2).wrapping_sub(self.eye.z),
        );
        let step = target.wrapping_sub(self.look).map(|v| v as i16);
        self.eye =
            terrain::traverse(c.map, c.cursor, self.look, step, 0, c.math)?.unwrap_or(target);
        self.update_view(c.math)?;
        Ok(true)
    }
}

fn wall_mode(actor: &Actor, c: &mut Context<'_>) -> Result<i16> {
    let rotation = Vector3::new(actor.rotation.x, actor.rotation.y, 0);
    let [left, right] = c.tables.wall_probes.map(|v| rotate(v, rotation, c.math));
    let mut mask = 0;
    for (v, sign, bit) in [(left, 1, 1), (right, 1, 2), (left, -1, 8), (right, -1, 4)] {
        let point = actor.position.wrapping_add(Position::new(
            i32::from(v.x) * sign,
            -500,
            i32::from(v.z) * sign,
        ));
        if terrain::level(c.map, c.cursor, point, Query::Uncached)?.is_none() {
            mask |= bit;
        }
    }
    Ok(match mask {
        4 => 12,
        8 => 11,
        _ if mask & 3 == 1 => 10,
        _ if mask & 3 == 2 => 9,
        _ => 0,
    })
}
