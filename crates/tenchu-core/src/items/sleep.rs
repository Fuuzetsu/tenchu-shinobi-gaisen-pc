use super::*;
use crate::{ai::Intelligence, ids::*};

impl Items {
    pub(super) fn sleep(
        &mut self,
        id: usize,
        used: Used<state::Sleep>,
        actors: &mut Actors,
        intelligence: &mut Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let model = item.model;
        let owner_id = used.owner;
        match used.phase {
            state::Sleep::Begin => {
                let owner = actors.get_mut(owner_id);
                owner.enter(ctx, MOT_ITEM_THROW, true);
                ctx.effects.sounds.push((owner.position, 0x26));
                self.slots[id].behavior = Behavior::Sleep(used.with_phase(state::Sleep::Throwing));
                return Ok(());
            }
            state::Sleep::Throwing => {
                let owner = actors.get(owner_id);
                if owner.motion.action != MOT_ITEM_THROW {
                    return self.dispose(id, actors, ctx);
                }
                if owner.motion.count == 3 {
                    let hand = *owner
                        .parts
                        .get(14)
                        .ok_or("Sleeping gas owner has no throwing hand")?;
                    let position = ctx.scene.world(hand)?.translation;
                    self.slots[id].napalm.count = 0;
                    self.slots[id].behavior =
                        Behavior::Sleep(used.with_phase(state::Sleep::Flying));
                    ctx.scene.set_translation(model, position);
                    self.contact(id, 1000, crate::collision::SOFT, ctx)?;
                }
                return Ok(());
            }
            state::Sleep::Finished => return self.dispose(id, actors, ctx),
            state::Sleep::Flying => {}
        }
        let item = &self.slots[id];
        let count = item.napalm.count;
        let brightness = ctx.math.pair((i32::from(count) * 0x88) as i16)[0] / 64 + 128;
        let position = std::array::from_fn(|i| {
            ctx.scene.models[model].local.translation[i]
                .wrapping_add(i32::from(item.napalm.velocity[i]))
        });
        ctx.scene.set_translation(model, position.into());
        ctx.effects.scatter_bleeds(
            position.into(),
            10,
            crate::effects::Burst {
                spread: 300,
                count: 2,
                lifetime: 10,
                color: [110; 3],
            },
            ctx.random,
        );
        self.slots[id].napalm.count = count.wrapping_add(1);
        if self.slots[id].napalm.count > 100 {
            self.slots[id].behavior = Behavior::Sleep(used.with_phase(state::Sleep::Finished));
        }
        if let Some((_, target)) = self.hit(id, ctx, actors)
            && target != owner_id
        {
            let actor = actors.get_mut(target);
            if !actor.parts.is_empty() {
                ctx.random.next15();
            }
            for _ in 0..3 {
                ctx.random.next15();
            }
            ctx.effects.sounds.push((position.into(), 0x23));
            let position = ctx.scene.models[actor.root_model].local.translation;
            ctx.effects
                .smoke(position, [0, -150, 0], 10, 30, ctx.random)?;
            if actor.health > 0 && actor.motion.action != MOT_ACTION {
                if actor.kind & 0xf0 != 0x80 {
                    actor.equip(ctx, false);
                    actor.enter(ctx, MOT_STATE_SHEATHE, true);
                    actor.think.idle = crate::ai::programs::Idle::Sleep;
                    actor.attribute &= !ATTR_PHASE;
                }
                actor.enter(ctx, MOT_ACTION, true);
                intelligence.voices.push((target, 6));
            }
            return self.dispose(id, actors, ctx);
        }
        if terrain::level(&ctx.mission.terrain, ctx.cursor, position.into(), 0)? == terrain::NONE {
            return self.dispose(id, actors, ctx);
        }
        self.update_coordinate(id, ctx);
        self.draw_sprite(
            model,
            self.sleep_sprite,
            sprite::Sprite {
                graphic: effects::Graphic::Sprite { image: 6, blend: 2 },
                scale: brightness * 2 + 16384,
                color: brightness as u8,
                rotation: i32::from(count) * 45 * 4096,
            },
            ctx,
        )
    }
}
