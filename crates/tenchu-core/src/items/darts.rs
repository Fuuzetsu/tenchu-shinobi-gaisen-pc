use super::*;
use crate::math::Position;
use crate::{actor::trail::Trail, ids::MOT_ITEM_THROW};

impl Items {
    pub(super) fn request_darts(
        &mut self,
        owner: usize,
        start: Position,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        actors.get_mut(owner).enter(ctx, MOT_ITEM_THROW, true);
        for _ in 0..8 {
            let id = self.allocate(actors, ctx)?;
            self.initialize(id, Processor::Darts, 15, Some(owner), start, ctx)?;
            self.slots[id].collision.size = 0;
            let mut rotation = actors.get(owner).rotation;
            self.slots[id].rotation = rotation;
            rotation[1] = rotation[1].wrapping_add((ctx.random.next15() % 512 - 256) as i16);
            let end = projectile::target(owner, rotation, start, actors, ctx)?.1;
            let item = &mut self.slots[id];
            item.flight = flight::Flight::new(start, end, 4096, 1024, 400, ctx);
            item.trail = Some(Trail::new(item.model, [[30, 0, 0], [-30, 0, 0]]));
            item.count = 8;
        }
        ctx.effects.sounds.push((actors.get(owner).position, 0x4c));
        Ok(())
    }

    pub(super) fn darts(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        self.fly(id, ctx)?;
        self.update_coordinate(id, ctx);
        self.draw_model(id, 3, ctx)?;
        self.draw_trail(id, ctx)?;
        let item = &self.slots[id];
        let model = item.model;
        let position = ctx.scene.models[model].local.translation;
        if item.flight.rolling && item.rolling.status != 0 {
            ctx.effects
                .scatter_bleeds(position, 0, 25, 10, 10, [255, 255, 0], ctx.random);
            ctx.effects.sounds.push((position, 0x31));
            self.dispose(id, actors, ctx)?;
        }
        if self.hit(id, ctx, actors).is_some() {
            ctx.effects.impact(position, 16384, 2, ctx.random);
            ctx.effects.sounds.push((position, 0x30));
            ctx.collision.remove(ctx.scene, model);
        }
        Ok(())
    }
}
