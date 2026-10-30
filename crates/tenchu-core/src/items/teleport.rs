use super::*;
use crate::{
    camera::{Environment, Mode},
    input::TRIANGLE,
};

impl Items {
    pub(super) fn teleport(
        &mut self,
        id: usize,
        owner_id: usize,
        actors: &mut Actors,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let owner = owner_id;
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
        if ctx.math.vector_distance(position.wrapping_sub(target)) < 20000 {
            player.position = target;
            ctx.scene.set_translation(player.root_model, target);
            ctx.effects.scatter_bleeds(
                target,
                20,
                crate::effects::Burst {
                    spread: 1000,
                    count: 50,
                    lifetime: 60,
                    color: [255; 3],
                },
                ctx.random,
            );
        }
        use_item::camera(Mode::Normal, player, ctx)?;
        self.dispose(id, actors, ctx)
    }
}
