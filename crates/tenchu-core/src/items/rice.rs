use super::*;
use crate::actor::State as ActorState;
use crate::ids::MOT_ITEM_PLANT;

impl Items {
    pub(super) fn rice(
        &mut self,
        id: usize,
        used: Used<state::Rice>,
        actors: &mut Actors,
        halt: crate::ai::Halt,
        player: usize,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner_id = used.owner;
        let owner = actors.get_mut(owner_id);
        match used.phase {
            state::Rice::Begin => {
                if halt == crate::ai::Halt::Running && owner.health > 0 {
                    owner.clear_attack(ctx, 3)?;
                    Self::item_motion(owner, MOT_ITEM_PLANT, ctx);
                    // Rice uses ActSTATE, avoiding ActITEM's mine request at frame 5.
                    owner.status = ActorState::Transition;
                }
                self.slots[id].behavior = Behavior::Rice(used.with_phase(state::Rice::Planting));
            }
            state::Rice::Planting => {
                if owner.motion.action != MOT_ITEM_PLANT {
                    self.slots[id].behavior = Behavior::Rice(used.with_phase(state::Rice::Begin));
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
        }
        Ok(())
    }
}
