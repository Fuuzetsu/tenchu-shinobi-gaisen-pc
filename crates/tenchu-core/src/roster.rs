use crate::math::Position;
use crate::{
    Result,
    actor::{Actor, Context},
};

#[derive(Debug, Default)]
pub struct Roster {
    pub keep_all_active: bool,
    pub actors: Vec<Option<Actor>>,
    pub order: Vec<usize>,
    reserved: Option<usize>,
    retired: Vec<usize>,
}

pub(crate) struct Actors<'a> {
    pub roster: &'a mut Roster,
    pub active: Option<&'a mut Actor>,
}

impl<'a> Actors<'a> {
    pub fn new(roster: &'a mut Roster) -> Self {
        Self {
            roster,
            active: None,
        }
    }

    pub fn get(&self, id: usize) -> &Actor {
        if let Some(actor) = self.active.as_deref().filter(|a| a.id == id) {
            actor
        } else {
            self.roster.get(id)
        }
    }

    pub fn get_mut(&mut self, id: usize) -> &mut Actor {
        if self.active.as_deref().is_some_and(|a| a.id == id) {
            self.active.as_deref_mut().unwrap()
        } else {
            self.roster.get_mut(id)
        }
    }
}

impl Roster {
    pub(crate) fn distance(&self, math: &crate::math::Math, delta: Position) -> i32 {
        if self.keep_all_active {
            math.length_wide(delta)
        } else {
            math.length(delta)
        }
    }

    pub(crate) fn begin_tick(&mut self) {
        for id in self.retired.drain(..) {
            self.actors[id] = None;
        }
    }

    pub(crate) fn take(&mut self, id: usize) -> Actor {
        assert!(self.reserved.is_none());
        self.reserved = Some(id);
        self.actors[id].take().unwrap()
    }

    pub(crate) fn restore(&mut self, actor: Actor) {
        let id = actor.id;
        assert_eq!(self.reserved, Some(id));
        self.reserved = None;
        if !self.order.contains(&id) {
            self.retired.push(id);
        }
        self.actors[id] = Some(actor);
    }

    pub fn get(&self, id: usize) -> &Actor {
        self.actors[id].as_ref().unwrap()
    }

    pub fn get_mut(&mut self, id: usize) -> &mut Actor {
        self.actors[id].as_mut().unwrap()
    }

    pub fn find(&self, kind: i16) -> Option<usize> {
        self.order
            .iter()
            .copied()
            .find(|&id| self.get(id).script_kind.unwrap_or(self.get(id).kind) == kind)
    }

    pub fn nearest(&self, actor: &Actor, range: i16, math: &crate::math::Math) -> Option<usize> {
        if actor.map.height != 0 {
            return None;
        }
        let mut best = None;
        let mut distance = 20000;
        for &id in &self.order {
            let Some(other) = self.actors[id].as_ref() else {
                continue;
            };
            if id == actor.id
                || other.status == 17
                || other.attribute & crate::ids::ATTR_SUSPEND != 0
                || other.kind & 0xf0 == 0x90
                || other.health < 0
            {
                continue;
            }
            let d = self.distance(
                math,
                Position::new(
                    other.position.x.wrapping_sub(actor.position.x),
                    0,
                    other.position.z.wrapping_sub(actor.position.z),
                ),
            );
            if d < i32::from(range) && d < distance {
                best = Some(id);
                distance = d;
            }
        }
        best
    }

    pub fn spawn(
        &mut self,
        kind: i16,
        position: Position,
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<usize> {
        self.spawn_with(ctx, |id, ctx| Actor::new(id, kind, position, yaw, ctx))
    }

    pub fn spawn_reinforcement(
        &mut self,
        choice: usize,
        position: Position,
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<usize> {
        let kind = ctx.mission.reinforcements[choice];
        let replacement = ctx.mission.reinforcement_overrides[choice];
        self.spawn_with(ctx, |id, ctx| {
            let mut actor = if let Some(source) = replacement.and_then(|r| r.actor) {
                let mut actor = Actor::from_definition(
                    id,
                    source.definition,
                    source.model,
                    position,
                    yaw,
                    ctx,
                )?;
                actor.script_kind = Some(kind);
                actor
            } else {
                Actor::new(id, kind, position, yaw, ctx)?
            };
            if let Some(replacement) = replacement {
                actor.voice_program = replacement.voice;
                actor.health = (i32::from(actor.max_health) * i32::from(replacement.health_percent)
                    / 100)
                    .clamp(1, 32767) as i16;
                actor.max_health = actor.health;
            }
            Ok(actor)
        })
    }

    fn spawn_with(
        &mut self,
        ctx: &mut Context,
        create: impl FnOnce(usize, &mut Context) -> Result<Actor>,
    ) -> Result<usize> {
        if self.order.len() == 40 {
            return Err("Humanoid pool exhausted".into());
        }
        let id = self
            .actors
            .iter()
            .enumerate()
            .position(|(i, actor)| actor.is_none() && self.reserved != Some(i))
            .unwrap_or(self.actors.len());
        let actor = create(id, ctx)?;
        if id == self.actors.len() {
            self.actors.push(Some(actor));
        } else {
            self.actors[id] = Some(actor);
        }
        self.order.push(id);
        Ok(id)
    }

    pub fn remove(&mut self, id: usize, ctx: &mut Context) {
        if let Some(mut actor) = self.actors[id].take() {
            self.remove_detached(&mut actor, ctx);
        }
    }

    pub(crate) fn remove_detached(&mut self, actor: &mut Actor, ctx: &mut Context) {
        ctx.collision.remove(ctx.scene, actor.parts[0]);
        for weapon in actor.weapons.iter().flatten() {
            ctx.collision.remove(ctx.scene, weapon.model);
        }
        actor.weapons.fill(None);
        if let Some(index) = self.order.iter().position(|&i| i == actor.id) {
            self.order.swap_remove(index);
        }
    }
}
