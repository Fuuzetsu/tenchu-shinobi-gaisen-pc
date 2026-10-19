use super::*;

impl Actor {
    pub(super) fn item_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        let complete = self.motion.count == 0 && self.motion.loops != 0;
        let kind = match self.motion.action {
            MOT_ITEM if self.motion.count == 10 => Some(2),
            MOT_ITEM_THROW if self.motion.count == 5 => {
                let kind = if self.id == world.player {
                    world.selected_item
                } else {
                    Some(4)
                };
                kind.filter(|k| matches!(k, 4 | 5))
            }
            MOT_ITEM_PLANT if self.motion.count == 5 => Some(6),
            MOT_ITEM_KAENGEKI | MOT_ITEM_SHINSOKU => {
                if complete {
                    self.motion.loops = -1;
                }
                return Ok(None);
            }
            _ => None,
        };
        if let Some(kind) = kind {
            let position = ctx.scene.world(self.parts[2])?.translation;
            world.items.request(
                kind,
                self,
                position,
                position,
                world.player,
                world.roster,
                ctx,
            )?;
        }
        Ok(if complete {
            Some((self.return_normal(ctx, world.player)?, true))
        } else {
            None
        })
    }

    pub(super) fn default_item(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        kind: usize,
    ) -> Result<()> {
        world
            .items
            .request_default(kind, self, world.player, world.roster, ctx)
    }

    pub(super) fn selected_item(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        pending: Option<(i16, bool)>,
    ) -> Result<Option<(i16, bool)>> {
        let action = match world.selected_item {
            Some(0) => MOT_KAGI,
            Some(1) => MOT_SYURI,
            Some(2) => MOT_ITEM,
            Some(4 | 5) => MOT_ITEM_THROW,
            Some(6) => MOT_ITEM_PLANT,
            None | Some(10) => {
                ctx.effects.sounds.push((self.position, 0x0c));
                return Ok(pending);
            }
            Some(kind) => {
                self.default_item(ctx, world, kind)?;
                return Ok(pending);
            }
        };
        Ok(Some((action, true)))
    }
}
