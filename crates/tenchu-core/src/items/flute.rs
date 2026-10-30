use super::*;
use crate::math::Position;

impl Items {
    pub(super) fn flute(
        &mut self,
        id: usize,
        used: Used<state::Flute>,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let owner = actors.get_mut(used.owner);
        match used.phase {
            state::Flute::Begin => {
                intelligence.alert = 0;
                owner.active_item = item.kind.index() as i16;
                ctx.effects.sounds.push((Position::ZERO, 0x43));
                item.count = 30;
                item.behavior = Behavior::Flute(used.with_phase(state::Flute::Playing));
            }
            state::Flute::Playing => {
                item.count = item.count.wrapping_sub(1);
                if item.count == 0 {
                    owner.active_item = 0;
                    self.dispose(id, actors, ctx)?;
                }
            }
        }
        Ok(())
    }
}
