use crate::{
    Result,
    actor::{Actor, Context},
};

#[derive(Debug, Default)]
pub struct Roster {
    pub actors: Vec<Option<Actor>>,
    pub order: Vec<usize>,
}

impl Roster {
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
            .find(|&id| self.get(id).kind == kind)
    }

    pub fn spawn(
        &mut self,
        kind: i16,
        position: [i32; 3],
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<usize> {
        if self.order.len() == 40 {
            return Err("Humanoid pool exhausted".into());
        }
        let id = self
            .actors
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.actors.len());
        let actor = Actor::new(id, kind, position, yaw, ctx)?;
        if id == self.actors.len() {
            self.actors.push(Some(actor));
        } else {
            self.actors[id] = Some(actor);
        }
        self.order.push(id);
        Ok(id)
    }

    pub fn remove(&mut self, id: usize, ctx: &mut Context) {
        if let Some(actor) = self.actors[id].take() {
            ctx.collision.remove(ctx.scene, actor.parts[0]);
            if let Some(index) = self.order.iter().position(|&i| i == id) {
                self.order.swap_remove(index);
            }
        }
    }
}
