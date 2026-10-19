use super::*;

impl Actor {
    fn shoot(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        kind: usize,
        start: [i32; 3],
    ) -> Result<()> {
        let target = self.target.ok_or("Ranged actor has no target")?;
        let target = ctx.scene.models[target].local.translation;
        let distance = ctx.math.length([
            target[0].wrapping_sub(self.position[0]),
            0,
            target[2].wrapping_sub(self.position[2]),
        ]) as i16;
        let velocity = ctx.math.velocity(self.rotation[1], distance.max(1000), 0);
        let mut end = [
            start[0].wrapping_add(i32::from(velocity[0])),
            target[1],
            start[2].wrapping_add(i32::from(velocity[2])),
        ];
        if kind == 21 && ctx.random.next15() % (4 - world.intelligence.difficulty as i32) != 0 {
            end[1] = end[1].wrapping_sub(1000);
        }
        world
            .items
            .request(kind, self, start, end, world.player, world.roster, ctx)
    }

    pub(super) fn gun_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        frame: i16,
        length: i16,
    ) -> Result<()> {
        if self.motion.count == frame {
            let start = ctx
                .scene
                .world(self.parts[13])?
                .transform([0, length, -100]);
            self.shoot(ctx, world, 20, start)?;
            world.intelligence.voices.push((self.id, 2));
        }
        Ok(())
    }

    pub(super) fn bow_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        combo: bool,
    ) -> Result<()> {
        let [min, max] = ctx.mission.weapons.bow_timing[usize::from(combo)];
        if self.motion.count == 1 {
            world.intelligence.voices.push((self.id, 2));
        } else if min <= self.motion.count && self.motion.count < max {
            let weapon = self.weapons[2]
                .as_ref()
                .ok_or("Bow has no arrow ornament")?;
            let model = &mut ctx.scene.models[weapon.model];
            model.local.rotation = Matrix::default().rotation;
            model.flags = 0;
            let matrix = ctx.scene.world(weapon.model)?;
            if let Some(mesh) = ctx.mission.weapons.models[&weapon.kind].first() {
                ctx.drawing.bytes += crate::packets::draw_tmd(
                    mesh,
                    &ctx.camera.view.compose(&matrix),
                    ctx.drawing.fog,
                );
            }
            world.items.weapons.push((weapon.kind, matrix));
        }
        if self.motion.count == max {
            let start = ctx.scene.world(self.parts[13])?.translation;
            self.shoot(ctx, world, 21, start)?;
            world.intelligence.voices.push((self.id, 3));
        }
        Ok(())
    }

    pub(super) fn shuriken_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        let complete = self.motion.count == 0 && self.motion.loops != 0;
        if self.motion.action == MOT_SYURI {
            let mut next = None;
            if self.id != world.player {
                if !complete {
                    return Ok(None);
                }
                next = Some((MOT_SYURI_RECOVER, true));
            }
            if complete {
                self.motion.loops = -1;
            }
            if self.motion.count == 1 {
                let start = ctx.scene.world(self.parts[2])?.translation;
                world
                    .items
                    .request(1, self, start, start, world.player, world.roster, ctx)?;
                ctx.effects.sounds.push((self.position, 0x1e));
            } else if self.inventory[25] != 1 {
                next = Some((MOT_SYURI_RECOVER, true));
                ctx.effects.sounds.push((self.position, 0x1f));
            } else if self.pad.trigger
                & (crate::input::SQUARE | crate::input::CROSS | crate::input::CIRCLE)
                != 0
            {
                if self.id == world.player {
                    self.inventory[25] = 0;
                } else {
                    world.roster.get_mut(world.player).inventory[25] = 0;
                }
                next = Some((self.return_normal(ctx, world.player)?, true));
            }
            return Ok(next);
        }
        if self.motion.action == MOT_SYURI_RECOVER {
            if self.motion.count == 1 && self.id != world.player {
                self.default_item(ctx, world, 1)?;
            }
            if complete {
                return Ok(Some((self.return_normal(ctx, world.player)?, true)));
            }
        }
        Ok(None)
    }
}
