use super::*;
use crate::collision;

impl Items {
    pub(super) fn caltrops(
        &mut self,
        id: usize,
        used: Used<state::Caltrops>,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let model = self.slots[id].model;
        let phase = used.phase;
        match phase {
            state::Caltrops::Falling => {
                let item = &mut self.slots[id];
                let mut position = ctx.scene.models[model].local.translation;
                item.rolling.step(&mut position, ctx)?;
                ctx.scene.set_translation(model, position);
                match item.rolling.status {
                    RollingStatus::Settled => {
                        item.behavior = Behavior::Caltrops(used.with_phase(state::Caltrops::Armed));
                        self.contact(id, 100, collision::HIT, ctx)?;
                    }
                    RollingStatus::Water => {
                        self.dispose(id, actors, ctx)?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            state::Caltrops::Armed if self.hit(id, ctx, actors).is_some() => {
                let position = ctx.scene.models[model].local.translation;
                ctx.effects.scatter_bleeds(
                    position,
                    20,
                    crate::effects::Burst {
                        spread: 0,
                        count: 10,
                        lifetime: 15,
                        color: [127, 0, 0],
                    },
                    ctx.random,
                );
                ctx.effects.sounds.push((position, 0x30));
                self.dispose(id, actors, ctx)?;
                return Ok(());
            }
            state::Caltrops::Armed => {}
        }
        self.update_coordinate(id, ctx);
        self.draw_icon(id, ctx)
    }
}
