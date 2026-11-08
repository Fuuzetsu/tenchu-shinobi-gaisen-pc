use super::*;

impl Camera {
    pub(super) fn endpoints(
        &self,
        subject: &Subject<'_>,
        mut rotation: Vector3<i16>,
        pose: Pose,
        preferred: bool,
        c: &mut Context<'_>,
    ) -> Result<[Position; 4]> {
        let actor = subject.actor;
        let mut origin = actor.position;
        if self.mode == 15 {
            origin = subject.root;
            if let Some((target, target_rotation)) = subject.target {
                origin = target;
                rotation = target_rotation;
            }
            if preferred || actor.kind != 19 || subject.target.is_some() {
                let hero = if c.hero < 3 { c.hero } else { 0 };
                let [x, y, z] = c
                    .tables
                    .origins
                    .get(usize::from(actor.motion.action as u16 & 7))
                    .ok_or("Invalid Tenchu 2 stealth camera action")?[hero];
                let [sin, cos] = c.math.pair(rotation.y);
                origin.x = origin.x.wrapping_add(
                    (x.wrapping_mul(cos) >> 12).wrapping_sub(z.wrapping_mul(sin) >> 12),
                );
                origin.y = origin.y.wrapping_add(y);
                origin.z = origin.z.wrapping_sub(
                    (z.wrapping_mul(cos) >> 12).wrapping_add(x.wrapping_mul(sin) >> 12),
                );
            }
        } else {
            if self.mode == 0 {
                rotation.x = rotation.x.wrapping_add(slope(actor, c)?);
            }
            origin.y = origin.y.wrapping_sub(if c.hero == 2 && self.mode != 13 {
                1850
            } else {
                1550
            });
        }
        let matrix = Matrix {
            rotation: c.math.rotate_yxz(rotation.to_array()),
            translation: origin,
        };
        Ok(pose.map(|v| matrix.transform(v)))
    }

    pub(super) fn clip(
        &self,
        focus: Position,
        near: &mut Position,
        far: &mut Position,
        c: &mut Context<'_>,
    ) -> Result<(i32, Position)> {
        let mut position = focus;
        position.y = position.y.wrapping_sub(10);
        if let Some(floor) = terrain::level(c.map, c.cursor, position, Query::Uncached)? {
            let count = if matches!(c.sequence, 58 | 94 | 96 | 189) {
                31
            } else {
                21
            };
            for _ in 0..count {
                let height =
                    terrain::level(c.map, c.cursor, position, Query::Uncached)?.unwrap_or(i32::MIN);
                let blocked = height < floor
                    && c.cursor.material & if self.mode == 13 { 0x300 } else { 0x100 } != 0;
                if blocked {
                    let node = c.cursor.last.map(|id| &c.map.cells[id.cell].nodes[id.node]);
                    let mut limit = position.y.wrapping_add(1010);
                    if self.mode != 13 {
                        if matches!(c.sequence, 94 | 96) && node.is_some_and(|node| node.y < -2500)
                        {
                            limit = position.y.wrapping_add(2510);
                        }
                        if c.sequence == 58 && node.is_some_and(|node| node.y < -600) {
                            limit = limit.wrapping_add(1500);
                        }
                    }
                    far.y = far.y.max(limit);
                    break;
                }
                position.y = position.y.wrapping_sub(200);
            }
        }
        let step = far.wrapping_sub(*near).map(|v| i32::from((v as i16) >> 5));
        near.x = near.x.wrapping_sub(step.x);
        if self.mode != 15 {
            near.y = near.y.wrapping_sub(step.y);
        }
        near.z = near.z.wrapping_sub(step.z);
        position = *near;
        for _ in 0..34 {
            if terrain::level(c.map, c.cursor, position, Query::Uncached)?.is_none() {
                let distance = length(step, c.math).max(1);
                let hit = position.wrapping_sub(Position::new(
                    step.x.wrapping_mul(400 / distance),
                    step.y.wrapping_mul(200 / distance),
                    step.z.wrapping_mul(400 / distance),
                ));
                let span = length(step.map(|v| i32::from((v << 5) as i16)), c.math).max(1);
                let ratio = length(near.wrapping_sub(hit), c.math)
                    .wrapping_shl(12)
                    .checked_div(span)
                    .ok_or("Tenchu 2 camera ray division overflow")?;
                return Ok((ratio, hit));
            }
            position = position.wrapping_add(step);
        }
        Ok((4096, *far))
    }

    pub(super) fn first_person(
        &mut self,
        actor: &Actor,
        c: &mut Context<'_>,
    ) -> Result<[Position; 2]> {
        let held = actor.pad.held;
        let vertical = if held & 0x40 != 0 {
            35
        } else if held & 0x10 != 0 {
            -35
        } else {
            0
        };
        let horizontal = if held & 0x20 != 0 {
            35
        } else if held & 0x80 != 0 {
            -35
        } else {
            0
        };
        self.direction[0] = self.direction[0].wrapping_sub(vertical).clamp(-796, 910);
        self.direction[1] = self.direction[1]
            .wrapping_add(horizontal)
            .clamp(-1024, 1024);
        let [sin, cos] = c.math.pair(actor.rotation.y);
        let forward = Position::new((sin * 600) >> 12, 0, (cos * 600) >> 12);
        let mut rotation = actor.rotation;
        rotation.x = rotation.x.wrapping_add(self.direction[0]);
        rotation.y = rotation.y.wrapping_add(self.direction[1]);
        let rotated = rotate([0, 0, 1800], rotation, c.math).map(i32::from);
        let mut anchor = actor.position;
        anchor.y = anchor.y.wrapping_sub(
            match ((0x510..0x600).contains(&actor.motion.action), c.hero == 2) {
                (true, true) => 700,
                (true, false) => 400,
                (false, true) => 1850,
                (false, false) => 1550,
            },
        );
        if actor.motion.action == 0x401 {
            anchor = anchor.wrapping_add(forward);
        } else {
            avoid_walls(&mut anchor, 1000, c)?;
            anchor = anchor.wrapping_sub(forward);
        }
        let mut probe = anchor;
        let mut ceiling = None;
        for _ in 0..15 {
            let height = terrain::level(c.map, c.cursor, probe, Query::Uncached)?;
            if height.is_none()
                && c.cursor.last.is_some_and(|id| {
                    c.map.cells[id.cell].nodes[id.node].attributes & 0xf00 == 0x100
                })
            {
                if c.cursor.material & 0x100 != 0 {
                    ceiling = c
                        .cursor
                        .last
                        .map(|id| i32::from(c.map.cells[id.cell].nodes[id.node].y) * 10);
                }
                break;
            }
            probe.y = probe.y.wrapping_sub(200);
        }
        if rotated.y <= 0 {
            anchor.y = anchor.y.wrapping_add(rotated.y >> 2);
            if let Some(ceiling) = ceiling.filter(|&y| y < actor.position.y) {
                anchor.y = anchor.y.max(ceiling.wrapping_add(1000));
            }
        } else {
            anchor.y = anchor
                .y
                .wrapping_sub(rotated.y.wrapping_add(i32::from((rotated.y / 3) as i16)));
            if let Some(ceiling) =
                ceiling.filter(|&y| y < actor.position.y && anchor.y.wrapping_sub(y) < 500)
            {
                anchor = anchor.wrapping_add(forward.map(|v| (v * 3) >> 1));
                anchor.y = ceiling.wrapping_add(500);
            }
        }
        Ok([
            anchor
                .wrapping_add(rotated)
                .wrapping_sub(self.eye)
                .map(|v| v / 8),
            anchor
                .wrapping_sub(rotated)
                .wrapping_sub(self.look)
                .map(|v| v / 4),
        ])
    }
}

fn slope(actor: &Actor, c: &mut Context<'_>) -> Result<i16> {
    c.cursor.current = None;
    let Some(start) = terrain::level(c.map, c.cursor, actor.position, Query::Relative)? else {
        return Ok(0);
    };
    let [sin, cos] = c.math.pair(actor.rotation.y);
    let point = actor
        .position
        .wrapping_sub(Position::new(sin / 16, 0, cos / 16));
    c.cursor.current = None;
    let Some(end) = terrain::level(c.map, c.cursor, point, Query::Relative)? else {
        return Ok(0);
    };
    let delta = end.wrapping_sub(start);
    if delta < -699 {
        return Ok(0);
    }
    Ok(c.math.atan2(delta, 256)?.clamp(-341, 341))
}

pub(super) fn avoid_walls(position: &mut Position, margin: i32, c: &mut Context<'_>) -> Result<()> {
    let mut mask = 0;
    for (x, z, bit, axis, sign) in [
        (0, -200, 1, 2, -1),
        (-200, 0, 2, 0, -1),
        (0, 200, 4, 2, 1),
        (200, 0, 8, 0, 1),
    ] {
        c.cursor.current = None;
        if let Some(hit) =
            terrain::traverse(c.map, c.cursor, *position, [x, 0, z].into(), 12, c.math)?
            && (hit[axis].wrapping_sub(position[axis])).wrapping_mul(sign) <= margin + 200
        {
            mask |= bit;
        }
    }
    if mask % 5 == 0 {
        let limit = if margin > 500 { 1000 } else { 600 };
        for (x, z, bits) in [
            (-200, -200, 3),
            (-200, 200, 6),
            (200, -200, 9),
            (200, 200, 12),
        ] {
            c.cursor.current = None;
            if let Some(hit) =
                terrain::traverse(c.map, c.cursor, *position, [x, 0, z].into(), 12, c.math)?
                && (hit
                    .x
                    .wrapping_sub(position.x)
                    .wrapping_mul(i32::from(x.signum()))
                    <= limit
                    || hit
                        .z
                        .wrapping_sub(position.z)
                        .wrapping_mul(i32::from(z.signum()))
                        <= limit)
            {
                mask |= bits;
            }
        }
    }
    let [x, z] = c.tables.wall_push[mask];
    position.x = position.x.wrapping_add(margin.wrapping_mul(i32::from(x)));
    position.z = position.z.wrapping_add(margin.wrapping_mul(i32::from(z)));
    Ok(())
}
