use super::*;
use crate::collision;

impl Items {
    pub(super) fn caltrops(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let model = self.slots[id].model;
        let mode = self.slots[id].mode;
        match mode {
            0 => {
                let item = &mut self.slots[id];
                let mut position = ctx.scene.models[model].local.translation;
                item.rolling.step(&mut position, ctx)?;
                ctx.scene.set_translation(model, position);
                match item.rolling.status {
                    4 => {
                        item.mode = 1;
                        self.contact(id, 100, collision::HIT, ctx)?;
                    }
                    1 => {
                        self.dispose(id, actors, ctx)?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            1 if self.hit(id, ctx, actors).is_some() => {
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
            _ => {}
        }
        self.update_coordinate(id, ctx);
        self.draw_icon(id, ctx)
    }
}
