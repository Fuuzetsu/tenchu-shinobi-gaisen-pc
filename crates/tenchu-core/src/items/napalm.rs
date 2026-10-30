use super::*;
use crate::math::Position;

#[derive(Clone, Debug, Default)]
pub struct Napalm {
    pub velocity: [i16; 3],
    pub count: u8,
}

impl Items {
    pub(crate) fn request_napalm(
        &mut self,
        owner: usize,
        start: Position,
        end: Position,
        clock: i32,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        use_item::consume(actors.get_mut(owner), Kind::Napalm);
        let id = self.allocate(actors, ctx)?;
        if clock & 1 == 0 {
            return Ok(());
        }
        self.initialize(
            id,
            Behavior::Napalm(Used::new(owner, state::Napalm::Begin)),
            Kind::Napalm,
            start,
            ctx,
        );
        self.slots[id].collision.size = 0;
        self.slots[id].napalm.velocity =
            std::array::from_fn(|i| end[i].wrapping_sub(start[i]) as i16);
        Ok(())
    }

    pub(super) fn napalm(
        &mut self,
        id: usize,
        used: Used<state::Napalm>,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        match used.phase {
            state::Napalm::Begin => {
                self.slots[id].napalm.count = 0;
                self.slots[id].behavior = Behavior::Napalm(used.with_phase(state::Napalm::Flying));
                return Ok(());
            }
            state::Napalm::Finished => return self.dispose(id, actors, ctx),
            state::Napalm::Flying => {}
        }
        let item = &self.slots[id];
        let model = item.model;
        let count = item.napalm.count;
        let square = i32::from(count).pow(2);
        let position = std::array::from_fn(|i| {
            ctx.scene.models[model].local.translation[i]
                .wrapping_add(i32::from(item.napalm.velocity[i]) * square / 100)
        });
        ctx.scene.set_translation(model, position.into());
        let brightness = (ctx.random.next15() % 25 - 26 - i32::from(count) * 230 / 20) as u8;
        let rotation = (ctx.random.next15() % 360) * 4096;
        let scale = square * 4096 / 50 + 4096;
        if count == 10 {
            self.contact(id, 500, crate::collision::HIT, ctx)?;
        }
        self.slots[id].napalm.count = count.wrapping_add(1);
        if self.slots[id].napalm.count > 20 {
            self.slots[id].behavior = Behavior::Napalm(used.with_phase(state::Napalm::Finished));
        }
        if let Some((_, target)) = self.hit(id, ctx, actors) {
            let actor = actors.get(target);
            let part = if actor.parts.is_empty() {
                0
            } else {
                ctx.random.next15() as usize % actor.parts.len()
            };
            let part = *actor
                .parts
                .get(part)
                .ok_or("Napalm target has no model parts")?;
            let local = std::array::from_fn(|_| ctx.random.next15() % 200 - 100);
            ctx.effects.flash(Some(part), local.into(), 12288, 60);
        }
        if terrain::level(&ctx.mission.terrain, ctx.cursor, position.into(), 0)? == terrain::NONE {
            return self.dispose(id, actors, ctx);
        }
        self.update_coordinate(id, ctx);
        for (sprite, image, blend, color) in [
            (self.napalm_sprites[1], 6, 3, (255 - brightness) / 3),
            (self.napalm_sprites[0], 7, 2, brightness),
        ] {
            self.draw_sprite(
                model,
                sprite,
                sprite::Sprite {
                    graphic: effects::Graphic::Sprite { image, blend },
                    scale,
                    color,
                    rotation,
                },
                ctx,
            )?;
        }
        Ok(())
    }
}
