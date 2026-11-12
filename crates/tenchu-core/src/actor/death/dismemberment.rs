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
            || if self.id == world.player {
                !world.combat.dismemberment.player
            } else {
                self.kind & 0xf0 == 0x80 && self.kind != 0x8f
            }
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
        let mut matrix = Matrix {
            translation: position,
            rotation: ctx.math.rotate_yxz(part.rotation),
        };
        let floor = supported_height(ctx, self.mesh_kind, part.index, matrix)?;
        let impact = floor.is_some_and(|floor| position.y >= floor);
        if impact {
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
            if part.index != 2 {
                part.rotation[0] = 1024;
                matrix.rotation = ctx.math.rotate_yxz(part.rotation);
            }
            position.y =
                supported_height(ctx, self.mesh_kind, part.index, matrix)?.unwrap_or(position.y);
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
        } else {
            part.velocity[1] = part.velocity[1].wrapping_add(20);
        }
        position = position.wrapping_add(part.velocity.map(i32::from).into());
        if part.index == 2 && part.moving {
            part.rotation[0] = part.rotation[0].wrapping_add(part.velocity[1].wrapping_abs());
        }
        if !part.moving && part.index == 2 {
            part.rotation = world
                .combat
                .dismemberment
                .head_rest
                .rotation(part.rotation, self.root_model);
        }
        matrix.translation = position;
        matrix.rotation = ctx.math.rotate_yxz(part.rotation);
        if let Some(floor) = supported_height(ctx, self.mesh_kind, part.index, matrix)? {
            matrix.translation.y = if part.moving {
                matrix.translation.y.min(floor)
            } else {
                floor
            };
        }
        ctx.scene.update(id, matrix);
        if impact {
            world.voice(self, ctx, self.id, 0x1b);
        }
        Ok(())
    }

    pub(super) fn settle_severed(
        &mut self,
        ctx: &mut Context,
        head_rest: crate::game::dismemberment::HeadRest,
    ) -> Result<()> {
        let Some(part) = self.severed.as_mut().filter(|p| p.moving) else {
            return Ok(());
        };
        let id = self.parts[part.index];
        let mut matrix = ctx.scene.models[id].local;
        if part.index != 2 {
            part.rotation[0] = 1024;
        } else {
            part.rotation = head_rest.rotation(part.rotation, self.root_model);
        }
        matrix.rotation = ctx.math.rotate_yxz(part.rotation);
        if let Some(floor) = supported_height(ctx, self.mesh_kind, part.index, matrix)? {
            matrix.translation.y = floor;
        }
        ctx.scene.models[id].attribute &= !2;
        ctx.scene.update(id, matrix);
        part.moving = false;
        Ok(())
    }
}

fn supported_height(
    ctx: &mut Context,
    kind: i16,
    index: usize,
    matrix: Matrix,
) -> Result<Option<i32>> {
    let Some(part) = ctx
        .mission
        .models
        .get(&kind)
        .and_then(|model| model.get(index))
    else {
        return Ok(None);
    };
    let mut height = None;
    for vertex in part.geometry.iter().flat_map(|mesh| &mesh.vertices) {
        let point = matrix.transform(*vertex);
        let floor = terrain::level(&ctx.mission.terrain, ctx.cursor, point, terrain::STEP_DOWN)?;
        if floor != terrain::NONE {
            let limit = floor - (point.y - matrix.translation.y) - 2;
            height = Some(height.map_or(limit, |old: i32| old.min(limit)));
        }
    }
    Ok(height)
}
