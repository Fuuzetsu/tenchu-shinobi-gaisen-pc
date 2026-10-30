use super::*;
use crate::ids::MOT_ITEM_PLANT;

impl Items {
    pub(super) fn rice(
        &mut self,
        id: usize,
        actors: &mut Actors,
        halt: crate::ai::Halt,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = item.owner.ok_or("Colored rice has no owner")?;
        let owner = actors.get_mut(owner_id);
        match item.mode {
            0 => {
                if halt == crate::ai::Halt::Running && owner.health > 0 {
                    owner.clear_attack(ctx, 3)?;
                    Self::item_motion(owner, MOT_ITEM_PLANT, ctx);
                    // Rice uses ActSTATE, avoiding ActITEM's mine request at frame 5.
                    owner.status = 8;
                }
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.action != MOT_ITEM_PLANT {
                    self.slots[id].mode = 0;
                    return Ok(());
                }
                if owner.motion.count != 15 {
                    return Ok(());
                }
                let hand = *owner
                    .parts
                    .get(13)
                    .ok_or("Colored rice owner has no hand")?;
                let position = ctx.scene.world(hand)?.translation;
                let velocity = item.rolling.velocity.map(i32::from);
                let action = owner.return_normal(ctx, player)?;
                owner.enter(ctx, action, true);
                self.dispose(id, actors, ctx)?;
                self.drop(8, Some(owner_id), position, velocity.into(), actors, ctx)?;
            }
            mode => return Err(format!("Invalid colored rice mode {mode}")),
        }
        Ok(())
    }
}
