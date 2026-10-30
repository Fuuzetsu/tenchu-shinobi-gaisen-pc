use super::*;
use crate::math::Position;
use crate::{camera::Mode, ids::MOT_KAGI, input::TRIANGLE};

impl Items {
    pub(super) fn grapple(
        &mut self,
        id: usize,
        owner_id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        let owner = actors.get(owner_id);
        if !owner.aiming {
            use_item::camera(Mode::Direction, owner, ctx)?;
            return self.dispose(id, actors, ctx);
        }
        if owner.motion.action != MOT_KAGI {
            return self.dispose(id, actors, ctx);
        }
        let rotation = ctx
            .math
            .vector_rotation(ctx.camera.position, ctx.camera.reference)?;
        if owner.pad.held & TRIANGLE != 0 {
            if rotation[0] < 0 {
                self.draw_sight(ctx);
            }
            return Ok(());
        }
        let vector = Matrix {
            rotation: ctx.math.rotate_yxz(rotation),
            ..Matrix::default()
        }
        .apply_large(Position::new(0, 0, -20000));
        let end = std::array::from_fn(|i| ctx.camera.position[i].wrapping_add(vector[i]));
        let target = terrain::ground_ray(
            &ctx.mission.terrain,
            ctx.cursor,
            ctx.math,
            ctx.camera.position,
            end.into(),
            0,
        )?
        .1;
        ctx.camera.target = Position::from_fn(|i| target[i].wrapping_add(vector[i] / 16));
        let root = ctx.scene.models[owner.root_model].local.translation;
        let distance = ctx.math.vector_distance(Position::from_fn(|i| {
            root[i].wrapping_sub(ctx.camera.target[i])
        }));
        if rotation[0] > 0 || distance > 15000 {
            ctx.camera.target = root;
        }
        use_item::camera(Mode::Lock, owner, ctx)?;
        actors.get_mut(owner_id).aiming = false;
        self.dispose(id, actors, ctx)
    }
}
