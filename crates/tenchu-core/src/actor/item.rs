use super::*;
use crate::items::Kind;

impl Actor {
    pub(super) fn item_control(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
    ) -> Result<Option<(i16, bool)>> {
        let complete = self.motion.count == 0 && self.motion.loops != 0;
        let kind = match self.motion.action {
            MOT_ITEM if self.motion.count == 10 => Some(Kind::Caltrops),
            MOT_ITEM_THROW if self.motion.count == 5 => {
                let kind = if self.id == world.player {
                    world.selected_item
                } else {
                    Some(Kind::Grenade)
                };
                kind.filter(|k| matches!(k, Kind::Grenade | Kind::SmokeBomb))
            }
            MOT_ITEM_PLANT if self.motion.count == 5 => Some(Kind::Mine),
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
                crate::items::Request {
                    kind,
                    start: position,
                    end: position,
                },
                self,
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
        kind: Kind,
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
            Some(Kind::Grapple) => MOT_KAGI,
            Some(Kind::Shuriken) => MOT_SYURI,
            Some(Kind::Caltrops) => MOT_ITEM,
            Some(Kind::Grenade | Kind::SmokeBomb) => MOT_ITEM_THROW,
            Some(Kind::Mine) => MOT_ITEM_PLANT,
            None | Some(Kind::Resurrection) => {
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
