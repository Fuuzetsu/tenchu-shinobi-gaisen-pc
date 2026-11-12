use super::*;

impl Controller<'_, '_> {
    pub(super) fn reinforcement_group_position(
        &self,
        choice: usize,
        count: u16,
    ) -> Result<Option<Position>> {
        let mission = &self.ctx.mission;
        let definition =
            if let Some(source) = mission.reinforcement_overrides[choice].and_then(|r| r.actor) {
                &mission.characters[source.definition]
            } else {
                mission
                    .characters
                    .iter()
                    .find(|c| c.kind == mission.reinforcements[choice])
                    .ok_or("Missing reinforcement definition")?
            };
        let width = i32::from(definition.width);
        let height = i32::from(definition.height);
        let spacing = width.max(250) * 2;
        let origin = self.actor.position;
        let starting_edge = terrain::vector(
            &mission.terrain,
            &mut terrain::Cursor::new(&mission.terrain),
            origin,
            width,
            terrain::STEP_DOWN,
        )?
        .vector;
        let solids: Vec<_> = self.ctx.collision.slots[..self.ctx.collision.len]
            .iter()
            .filter(|slot| {
                slot.actor().is_none()
                    && slot.class & (crate::collision::HIT | crate::collision::SOFT) == 0
                    && slot.model.is_some_and(|id| {
                        self.ctx.scene.models[id].attribute & crate::scene::COLLIDE != 0
                    })
            })
            .collect();
        let blocked = |position: Position| {
            let center = position.wrapping_sub(Position::new(0, height / 2, 0));
            solids.iter().any(|slot| {
                (slot.position.x - center.x).abs() < i32::from(slot.size[0]) + width / 2
                    && (slot.position.y - center.y).abs() < i32::from(slot.size[1]) + height / 2
                    && (slot.position.z - center.z).abs() < i32::from(slot.size[2]) + width / 2
            })
        };
        for radius in 1..=i32::from(count) {
            for z in -radius..=radius {
                for x in -radius..=radius {
                    if x.abs() != radius && z.abs() != radius {
                        continue;
                    }
                    let mut position =
                        origin.wrapping_add(Position::new(x * spacing, 0, z * spacing));
                    let mut cursor = terrain::Cursor::new(&mission.terrain);
                    let probe = terrain::vector(
                        &mission.terrain,
                        &mut cursor,
                        position,
                        width,
                        terrain::STEP_DOWN,
                    )?;
                    if probe.level == terrain::NONE
                        || probe.vector != 0
                        || (probe.level - origin.y).abs() > 450
                    {
                        continue;
                    }
                    position.y = probe.level;
                    if self
                        .ctx
                        .math
                        .length_wide(position.wrapping_sub(self.player.position))
                        <= 16500
                    {
                        continue;
                    }
                    let occupied = self
                        .roster
                        .order
                        .iter()
                        .filter_map(|&id| self.roster.actors[id].as_ref())
                        .chain(std::iter::once(&*self.actor))
                        .any(|actor| {
                            let other = &mission.characters[actor.definition];
                            let clearance = (width + i32::from(other.width)) / 2 + width / 2;
                            (actor.position.x - position.x).abs() < clearance
                                && (actor.position.z - position.z).abs() < clearance
                                && actor.position.y > position.y - height
                                && position.y > actor.position.y - i32::from(other.height)
                        });
                    if occupied
                        || blocked(position)
                        || !self.reinforcement_spawn_allowed_at(position, height, true)?
                    {
                        continue;
                    }
                    let delta = position.wrapping_sub(origin);
                    let steps = (delta.x.abs().max(delta.z.abs()) + 249) / 250;
                    let mut clear = true;
                    // A caller can stand at an edge. Permit moving away from it,
                    // but never cross an additional blocked corner on the way out.
                    let mut edge = starting_edge;
                    for step in 1..=steps {
                        let point =
                            origin.wrapping_add(delta.map(|v| {
                                (i64::from(v) * i64::from(step) / i64::from(steps)) as i32
                            }));
                        let probe = terrain::vector(
                            &mission.terrain,
                            &mut cursor,
                            point,
                            width,
                            terrain::STEP_DOWN,
                        )?;
                        if probe.level == terrain::NONE
                            || probe.vector & !edge != 0
                            || (probe.level - origin.y).abs() > 450
                            || blocked(point)
                        {
                            clear = false;
                            break;
                        }
                        edge &= probe.vector;
                    }
                    if clear {
                        return Ok(Some(position));
                    }
                }
            }
        }
        Ok(None)
    }
}
