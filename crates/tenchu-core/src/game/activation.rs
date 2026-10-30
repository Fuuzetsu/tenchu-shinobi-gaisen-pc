use super::Game;
use crate::actor::State as ActorState;
use crate::math::Position;
#[cfg(test)]
mod tests;
use crate::{
    Result,
    ids::{ATTR_FLOAT, ATTR_SUSPEND},
    scene::COLLIDE,
    terrain,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Activation {
    pub raw_budget: i16,
    pub budget: i16,
    pub count: i16,
}

impl Activation {
    fn update_budget(&mut self, bytes: u32) {
        self.raw_budget = ((60536 - bytes as i32) / 5000 - 1) as i16;
        self.budget = if self.raw_budget < 2 {
            (self.budget as u16).wrapping_sub(1) as i16
        } else {
            self.raw_budget
        }
        .clamp(3, 6);
        self.count = 0;
    }
}

impl Game {
    fn keeps_active(&self, id: usize) -> bool {
        let actor = self.roster.get(id);
        self.roster.keep_all_active
            && id != self.player_id
            && (actor.script_kind.is_some() || actor.kind != 0xa9)
            && actor.health >= 0
    }

    pub(super) fn activate(&mut self) -> Result<()> {
        if self.roster.keep_all_active {
            for &id in &self.roster.order {
                let keep_active = self.keeps_active(id);
                let actor = self.roster.actors[id].as_mut().unwrap();
                if keep_active && actor.attribute & ATTR_SUSPEND != 0 {
                    actor.attribute &= !ATTR_SUSPEND;
                    self.scene.models[actor.parts[0]].attribute |= COLLIDE;
                }
            }
        }
        if self.clock % 30 != 0 || self.drawing.step != crate::drawing::Step::Normal {
            return Ok(());
        }
        let focus = self.player_id;
        let position = self.roster.get(focus).position;
        let radius = if self.player().motion.action == 0xf05 {
            26000
        } else {
            13000
        };
        self.activation.update_budget(self.drawing.sampled_bytes);
        let training = self.mission.stage.id == 8;
        let first = self.clock == 30;
        for &id in &self.roster.order {
            if id == focus {
                continue;
            }
            let keep_active = self.keeps_active(id);
            let actor = self.roster.actors[id].as_mut().unwrap();
            let distance = self.math.vector_distance(Position::from_fn(|i| {
                actor.position[i].wrapping_sub(position[i])
            }));
            let suspended = actor.attribute & ATTR_SUSPEND != 0;
            let active = if keep_active {
                true
            } else if distance > 17000 {
                false
            } else if actor.kind & 0xf0 == 0x80
                || actor.kind == 0xa9
                || actor.health < 0
                || first
                || training
            {
                true
            } else if (self.visible.len() as i16) < self.activation.budget {
                self.activation.count < self.activation.budget || distance < radius
            } else if distance >= radius {
                false
            } else if !suspended && self.activation.count < self.activation.budget {
                true
            } else {
                self.visible.contains(&id)
            };
            if active {
                if !suspended {
                    self.activation.count = self.activation.count.wrapping_add(1);
                } else if training
                    || actor.health < 0
                    || first
                    || (self.activation.count < self.activation.budget && distance > 13000)
                {
                    actor.attribute &= !ATTR_SUSPEND;
                    self.scene.models[actor.parts[0]].attribute |= COLLIDE;
                    self.activation.count = self.activation.count.wrapping_add(1);
                }
            } else if !suspended && actor.kind != 0x84 {
                if actor.script_kind.is_none()
                    && ((actor.kind == 0x87 && matches!(self.mission.stage.id, 6 | 7))
                        || actor.kind == 0x83)
                {
                    for spawn in &self.mission.story {
                        if spawn.kind == actor.kind {
                            actor.position = spawn.position.into();
                            self.scene.set_translation(actor.root_model, actor.position);
                        }
                    }
                    if actor.kind == 0x83 && actor.health == 0 {
                        actor.health = 1;
                    }
                } else if actor.status != ActorState::Dead && actor.attribute & ATTR_FLOAT == 0 {
                    let query = Position::new(
                        actor.home[0],
                        actor.position.y.wrapping_sub(1500),
                        actor.home[1],
                    );
                    if self.math.vector_distance(query.wrapping_sub(position)) > 17000 {
                        let level = terrain::level(
                            &self.mission.terrain,
                            &mut self.cursor,
                            query,
                            terrain::STEP_DOWN,
                        )?;
                        if level != terrain::NONE {
                            actor.position = Position::new(query[0], level, query[2]);
                            self.scene.set_translation(actor.root_model, actor.position);
                        }
                    }
                }
                actor.attribute |= ATTR_SUSPEND;
                self.scene.models[actor.parts[0]].attribute &= !COLLIDE;
            }
        }
        Ok(())
    }
}
