use super::*;
use crate::game::dismemberment::Part;

impl Actor {
    pub(crate) fn restore_severed(&mut self, scene: &mut Scene) {
        if let Some(part) = self.severed.take() {
            let model = &mut scene.models[self.parts[part.index]];
            model.parent = part.parent;
            model.attribute &= !2;
            model.flags = 0;
            self.motion.mask |= (1 << part.index) | 0x6000;
        }
    }

    pub(super) fn sever_part(
        &mut self,
        ctx: &mut Context,
        world: &ActionContext,
        index: usize,
    ) -> Result<()> {
        if self.severed.is_some()
            || self.id == world.player
            || self.kind & 0xf0 == 0x80 && self.kind != 0x8f
        {
            return Ok(());
        }
        let Some(&id) = self.parts.get(index) else {
            return Ok(());
        };
        let position = ctx.scene.world(id)?.translation;
        let parent = ctx.scene.models[id].parent;
        let rotation = [0, self.rotation[1], 0];
        let mut velocity = if index == 2 {
            ctx.math.velocity(self.rotation[1], 100, 0)
        } else {
            [
                (ctx.random.next15() % 100 - 50) as i16,
                0,
                (ctx.random.next15() % 100 - 50) as i16,
            ]
        };
        velocity[1] = -100;
        self.motion.mask &= !((1 << index) | 0x6000);
        ctx.scene.models[id].parent = Some(0);
        ctx.scene.models[id].attribute |= 2;
        ctx.scene.update(
            id,
            Matrix {
                translation: position,
                rotation: ctx.math.rotate_yxz(rotation),
            },
        );
        self.severed = Some(Part {
            index,
            parent,
            velocity,
            rotation,
            moving: true,
        });
        Ok(())
    }

    pub(super) fn advance_severed(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<()> {
        let Some(part) = self.severed.as_mut().filter(|p| p.moving) else {
            return Ok(());
        };
        let id = self.parts[part.index];
        let mut position = ctx.scene.models[id].local.translation;
        let level = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            position,
            terrain::RETURN_DELTA,
        )?;
        if level <= 0 || level == terrain::NONE {
            let probe = terrain::vector(
                &ctx.mission.terrain,
                ctx.cursor,
                position,
                i32::from(ctx.mission.characters[self.definition].width),
                terrain::STEP_DOWN,
            )?;
            let [x, z] = ctx.body.reflect_move[usize::from(probe.vector)];
            if x != 0 || z != 0 {
                part.velocity[0] = x.wrapping_mul(-50);
                part.velocity[2] = z.wrapping_mul(-50);
            } else {
                part.velocity[0] /= 2;
                part.velocity[2] /= 2;
            }
            position.y = terrain::level(
                &ctx.mission.terrain,
                ctx.cursor,
                position,
                terrain::STEP_DOWN,
            )?;
            part.velocity[1] = part.velocity[1].wrapping_neg() / 2;
            if part.velocity[1].unsigned_abs() < 20 {
                part.moving = false;
                ctx.scene.models[id].attribute &= !2;
            }
            ctx.effects.blood(
                position,
                1,
                30,
                ctx.random,
                &mut crate::effects::Environment {
                    terrain: &ctx.mission.terrain,
                    cursor: ctx.cursor,
                    owner_position: self.position,
                    reference: ctx.camera.reference,
                    clock: world.clock,
                },
            )?;
            world.intelligence.voices.push((self.id, 0x1b));
        } else {
            part.velocity[1] = part.velocity[1].wrapping_add(20);
        }
        position = position.wrapping_add(part.velocity.map(i32::from).into());
        if part.index == 2 {
            part.rotation[0] = part.rotation[0].wrapping_add(part.velocity[1].wrapping_abs());
        }
        ctx.scene.update(
            id,
            Matrix {
                translation: position,
                rotation: ctx.math.rotate_yxz(part.rotation),
            },
        );
        Ok(())
    }

    pub(super) fn settle_severed(&mut self, ctx: &mut Context) -> Result<()> {
        let Some(part) = self.severed.as_mut().filter(|p| p.moving) else {
            return Ok(());
        };
        let id = self.parts[part.index];
        let mut matrix = ctx.scene.models[id].local;
        let mut origin = matrix.translation;
        origin.y = origin.y.wrapping_sub(100);
        matrix.translation.y = terrain::level(&ctx.mission.terrain, ctx.cursor, origin, 0)?;
        ctx.scene.models[id].attribute &= !2;
        ctx.scene.update(id, matrix);
        part.moving = false;
        Ok(())
    }
}
