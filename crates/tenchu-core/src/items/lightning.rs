use super::*;

#[derive(Clone, Debug, Default)]
pub struct Lightning {
    pub start: [i32; 3],
    pub rotation: [i16; 3],
}

impl Items {
    pub(super) fn lightning(
        &mut self,
        id: usize,
        actors: &mut Actors,
        player: usize,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let start = item.lightning.start;
        match item.mode {
            0 => {
                item.count = 15;
                item.mode = 1;
                if item.owner == Some(player) {
                    ctx.effects.sounds.push(([0; 3], 0x39));
                }
            }
            1 => {
                let target = projectile::target(
                    item.owner.ok_or("Lightning has no owner")?,
                    item.lightning.rotation,
                    start,
                    actors,
                    ctx,
                )?
                .1;
                ctx.scene.set_translation(item.model, target);
                item.mode = 2;
                self.contact(id, 100, crate::collision::HIT, ctx)?;
            }
            2 => {
                if clock % 3 == 0 {
                    item.mode = 1;
                }
            }
            mode => return Err(format!("Invalid lightning mode {mode}")),
        }
        let position = ctx.scene.models[self.slots[id].model].local.translation;
        ctx.drawing.bytes += ctx.effects.lightning(
            start,
            position,
            [100, 100, 200],
            ctx.math,
            effects::View {
                position: ctx.camera.position,
                matrix: ctx.camera.view,
            },
            ctx.random,
        );
        if clock & 3 == 0 {
            ctx.effects
                .scatter_bleeds(position, 200, 20, 10, 20, [255, 255, 120], ctx.random);
            ctx.effects.impact(start, 16384, 1, ctx.random);
        }
        let item = &mut self.slots[id];
        let count = item.count;
        item.count = item.count.wrapping_sub(1);
        if count == 0 {
            self.dispose(id, actors, ctx)?;
        }
        Ok(())
    }
}
