use super::*;
use crate::{
    camera::{Environment, Mode},
    input::TRIANGLE,
};

impl Items {
    pub(super) fn teleport(
        &mut self,
        id: usize,
        actors: &mut Actors,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let owner = self.slots[id].owner.ok_or("Teleport has no owner")?;
        if actors.get(owner).pad.held & TRIANGLE != 0 {
            use_item::camera(Mode::Sight, actors.get(player), ctx)?;
            self.draw_sight(ctx);
            return Ok(());
        }
        let player = actors.get_mut(player);
        let position = ctx.scene.models[player.root_model].local.translation;
        ctx.camera.snap_target(
            position,
            &mut Environment {
                terrain: &ctx.mission.terrain,
                cursor: ctx.cursor,
                math: ctx.math,
                body: ctx.body,
            },
        )?;
        let target = ctx.camera.target;
        if ctx
            .math
            .vector_distance(std::array::from_fn(|i| position[i].wrapping_sub(target[i])))
            < 20000
        {
            player.position = target;
            ctx.scene.set_translation(player.root_model, target);
            ctx.effects
                .scatter_bleeds(target, 1000, 20, 50, 60, [255; 3], ctx.random);
        }
        use_item::camera(Mode::Normal, player, ctx)?;
        self.dispose(id, actors, ctx)
    }
}
