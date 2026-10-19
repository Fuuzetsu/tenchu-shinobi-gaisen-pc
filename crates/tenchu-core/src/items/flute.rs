use super::*;

impl Items {
    pub(super) fn flute(
        &mut self,
        id: usize,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let owner = actors.get_mut(item.owner.ok_or("Lure flute has no owner")?);
        match item.mode {
            0 => {
                intelligence.alert = 0;
                owner.active_item = item.kind as i16;
                ctx.effects.sounds.push(([0; 3], 0x43));
                item.count = 30;
                item.mode = 1;
            }
            1 => {
                item.count = item.count.wrapping_sub(1);
                if item.count == 0 {
                    owner.active_item = 0;
                    self.dispose(id, actors, ctx)?;
                }
            }
            mode => return Err(format!("Invalid lure flute mode {mode}")),
        }
        Ok(())
    }
}
