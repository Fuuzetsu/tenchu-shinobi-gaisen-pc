use super::*;
use crate::math::Position;

#[derive(Clone, Debug, Default)]
pub struct Lightning {
    pub start: Position,
    pub rotation: [i16; 3],
}

impl Items {
    pub(super) fn lightning(
        &mut self,
        id: usize,
        used: Used<state::Lightning>,
        actors: &mut Actors,
        player: usize,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let start = item.lightning.start;
        match used.phase {
            state::Lightning::Begin => {
                item.count = 15;
                item.behavior = Behavior::Lightning(used.with_phase(state::Lightning::Target));
                if used.owner == player {
                    ctx.effects.sounds.push((Position::ZERO, 0x39));
                }
            }
            state::Lightning::Target => {
                let target =
                    projectile::target(used.owner, item.lightning.rotation, start, actors, ctx)?.1;
                ctx.scene.set_translation(item.model, target);
                item.behavior = Behavior::Lightning(used.with_phase(state::Lightning::Hold));
                self.contact(id, 100, crate::collision::HIT, ctx)?;
            }
            state::Lightning::Hold => {
                if clock % 3 == 0 {
                    item.behavior = Behavior::Lightning(used.with_phase(state::Lightning::Target));
                }
            }
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
            ctx.effects.scatter_bleeds(
                position,
                20,
                crate::effects::Burst {
                    spread: 200,
                    count: 10,
                    lifetime: 20,
                    color: [255, 255, 120],
                },
                ctx.random,
            );
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
