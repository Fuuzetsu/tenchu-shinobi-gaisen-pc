use super::*;
use crate::{camera::Mode, ids::MOT_SYURI, input::TRIANGLE};

impl Items {
    pub(super) fn sight_shot(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Aimed shuriken has no owner")?;
        let owner = actors.get_mut(owner_id);
        if !owner.aiming {
            if owner.inventory[item.kind] != 255 {
                owner.inventory[item.kind] = owner.inventory[item.kind].wrapping_add(1);
            }
            use_item::camera(Mode::Direction, owner, ctx)?;
            return self.dispose(id, actors, ctx);
        }
        if owner.motion.action != MOT_SYURI {
            let position = ctx.scene.world(item.model)?.translation;
            let kind = item.kind;
            let velocity = [
                ctx.random.next15() % 200 - 100,
                ctx.random.next15() % 100 - 200,
                ctx.random.next15() % 200 - 100,
            ];
            self.drop(kind, Some(owner_id), position, velocity.into(), actors, ctx)?;
            return self.dispose(id, actors, ctx);
        }
        let item = &mut self.slots[id];
        if item.count != 0 {
            item.count -= 1;
        }
        if owner.pad.held & TRIANGLE != 0 {
            if item.count == 0 {
                use_item::camera(Mode::Sight, owner, ctx)?;
                self.draw_sight(ctx);
            }
            return Ok(());
        }
        let aimed = item.count == 0;
        let start = ctx.scene.models[item.model].local.translation;
        let kind = item.kind;
        let (rotation, position) = if aimed {
            (
                ctx.math
                    .vector_rotation(ctx.camera.position, ctx.camera.reference)?,
                ctx.camera.position,
            )
        } else {
            (owner.rotation, start)
        };
        let end = projectile::target(owner_id, rotation, position, actors, ctx)?.1;
        self.dispose(id, actors, ctx)?;
        if aimed {
            use_item::camera(Mode::Lock, actors.get(owner_id), ctx)?;
        }
        self.launch(kind, owner_id, start, end, actors, ctx)
    }
}
