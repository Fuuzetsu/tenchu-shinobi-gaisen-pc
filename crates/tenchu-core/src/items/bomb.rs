use super::*;
use crate::collision;
use crate::math::Position;

impl Items {
    fn explosion(position: Position, mine: bool, ctx: &mut Context) -> Result<()> {
        ctx.effects.explosion(position, [0, -25, 0], ctx.random);
        ctx.effects.embers(
            position,
            [75, if mine { 200 } else { 120 }, 75],
            if mine { 10 } else { 8 },
            ctx.random,
        )?;
        ctx.effects.smoke(
            position,
            [0, if mine { -400 } else { -200 }, 0],
            20,
            6,
            ctx.random,
        )?;
        ctx.effects.sounds.push((position, 0x25));
        Ok(())
    }

    fn burn_contact(&mut self, id: usize, mine: bool, actors: &mut Actors, ctx: &mut Context) {
        let Some((_, target)) = self.hit(id, ctx, actors) else {
            return;
        };
        let actor = actors.get(target);
        for _ in 0..if mine { 10 } else { 1 } {
            let part = actor.parts[(ctx.random.next15() as usize) % actor.parts.len()];
            let position = std::array::from_fn(|_| ctx.random.next15() % 200 - 100);
            let time = if mine {
                (ctx.random.next15() % 60 + 60) as i16
            } else {
                120
            };
            ctx.effects.flash(Some(part), position.into(), 12288, time);
        }
    }

    pub(super) fn fire(
        &mut self,
        id: usize,
        used: Used<state::Fire>,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let Some(position) = self.roll_icon(id, actors, ctx)? else {
            return Ok(());
        };
        let ember = position.map(|p| p.wrapping_add(ctx.random.next15() % 50 - 25));
        let time = (ctx.random.next15() % 20) as u8;
        ctx.effects.bleed(ember, [0, -30, 0], time, [255, 255, 0]);
        let item = &mut self.slots[id];
        item.count = item.count.wrapping_sub(1);
        match used.phase {
            state::Fire::Fuse => {
                if item.count == 0 {
                    if ctx.random.next15() % 10 < 2 {
                        let kind = item.kind;
                        let position = ctx.scene.models[self.icons[kind]].local.translation;
                        self.dispose(id, actors, ctx)?;
                        self.stay(kind, position, actors, ctx)?;
                        ctx.effects
                            .smoke_single(position, [0, -100, 0], 10, ctx.random)?;
                        return Ok(());
                    }
                } else {
                    if item.count == 140 {
                        self.contact(id, 500, collision::SOFT, ctx)?;
                    }
                    let Some(slot) = self.first_contact(id, ctx) else {
                        return Ok(());
                    };
                    let contact = &ctx.collision.slots[slot];
                    if !contact
                        .actor()
                        .is_some_and(|owner| actors.roster.order.contains(&owner))
                        && contact.class != collision::HIT
                    {
                        return Ok(());
                    }
                }
                self.slots[id].behavior = Behavior::Fire(used.with_phase(state::Fire::Detonate));
            }
            state::Fire::Detonate => {
                Self::explosion(position, false, ctx)?;
                self.contact(id, 1500, collision::HIT, ctx)?;
                self.slots[id].behavior = Behavior::Fire(used.with_phase(state::Fire::Burning));
                self.slots[id].count = 3;
                intelligence.reset_alert();
            }
            state::Fire::Burning => {
                if item.count == 0 {
                    self.dispose(id, actors, ctx)?;
                }
                self.burn_contact(id, false, actors, ctx);
            }
        }
        Ok(())
    }

    pub(super) fn mine(
        &mut self,
        id: usize,
        used: Used<state::Mine>,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &mut self.slots[id];
        let model = item.model;
        match used.phase {
            state::Mine::Place => {
                let mut position = ctx.scene.models[model].local.translation;
                position[1] = terrain::level(
                    &ctx.mission.terrain,
                    ctx.cursor,
                    position,
                    terrain::STEP_DOWN,
                )?;
                ctx.scene.set_translation(model, position);
                if position[1] == terrain::NONE || ctx.cursor.attribute & 4 != 0 {
                    let owner = actors.get_mut(used.owner);
                    if owner.inventory[item.kind] != 255 {
                        owner.inventory[item.kind] = owner.inventory[item.kind].wrapping_add(1);
                    }
                    return self.dispose(id, actors, ctx);
                }
                self.contact(id, 500, collision::SOFT, ctx)?;
                self.slots[id].behavior = Behavior::Mine(used.with_phase(state::Mine::Armed));
            }
            state::Mine::Armed => {
                if self.hit(id, ctx, actors).is_some() {
                    self.contact(id, 1500, collision::HIT, ctx)?;
                    self.slots[id].behavior =
                        Behavior::Mine(used.with_phase(state::Mine::Detonate));
                }
            }
            state::Mine::Detonate => {
                Self::explosion(ctx.scene.models[model].local.translation, true, ctx)?;
                self.slots[id].behavior = Behavior::Mine(used.with_phase(state::Mine::Burning));
                self.slots[id].count = 3;
                intelligence.reset_alert();
            }
            state::Mine::Burning => {
                self.burn_contact(id, true, actors, ctx);
                let item = &mut self.slots[id];
                item.count = item.count.wrapping_sub(1);
                if item.count == 255 {
                    self.dispose(id, actors, ctx)?;
                }
                return Ok(());
            }
        }
        self.update_coordinate(id, ctx);
        self.draw_icon(id, ctx)
    }
}
