use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::{collision, ids::MOT_STATE_PICKUP, scene::CONFLICT};

impl Items {
    pub(crate) fn drop(
        &mut self,
        kind: usize,
        owner: Option<usize>,
        position: Position,
        velocity: Position,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<bool> {
        let id = self.allocate(actors, ctx)?;
        if terrain::level(&ctx.mission.terrain, ctx.cursor, position, 0)? < position[1] {
            return Ok(false);
        }
        self.initialize(
            id,
            Behavior::Pickup {
                owner,
                phase: state::Pickup::Rolling,
            },
            kind,
            position,
            ctx,
        )?;
        let item = &mut self.slots[id];
        item.collision.size = 0;
        item.rolling = Rolling {
            hint: None,
            velocity: velocity.map(|v| v as i16).to_array(),
            status: RollingStatus::Airborne,
        };
        Ok(true)
    }

    pub(crate) fn stay(
        &mut self,
        kind: usize,
        mut position: Position,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        position[1] = terrain::level(&ctx.mission.terrain, ctx.cursor, position, 0)?;
        self.drop(kind, None, position, Position::ZERO, actors, ctx)?;
        Ok(())
    }

    pub(super) fn pickup(
        &mut self,
        id: usize,
        owner: Option<usize>,
        phase: state::Pickup,
        ctx: &mut Context,
        actors: &mut Actors,
        halt: crate::ai::Halt,
    ) -> Result<()> {
        self.draw_icon(id, ctx)?;
        let item = &mut self.slots[id];
        match phase {
            state::Pickup::Rolling => {
                let mut position = ctx.scene.models[item.model].local.translation;
                item.rolling.step(&mut position, ctx)?;
                ctx.scene.set_translation(item.model, position);
                match item.rolling.status {
                    RollingStatus::Water => self.dispose(id, actors, ctx)?,
                    RollingStatus::Ground | RollingStatus::Settled => {
                        self.contact(id, 180, collision::SOFT, ctx)?;
                        self.slots[id].behavior = Behavior::Pickup {
                            owner,
                            phase: state::Pickup::Available,
                        };
                    }
                    _ => {}
                }
            }
            state::Pickup::Available => {
                if ctx.scene.models[item.model].attribute & CONFLICT == 0 {
                    return Ok(());
                }
                let Some(slot) = ctx.collision.result(ctx.scene, item.model, None) else {
                    return Ok(());
                };
                let Some(actor_id) = ctx.collision.slots[slot]
                    .actor()
                    .filter(|id| actors.roster.order.contains(id))
                else {
                    return Ok(());
                };
                let actor = actors.get_mut(actor_id);
                if actor.motion.action == MOT_STATE_PICKUP
                    || !matches!(actor.status, ActorState::Walk | ActorState::Chase)
                {
                    return Ok(());
                }
                if halt == crate::ai::Halt::Running && actor.health > 0 {
                    actor.clear_attack(ctx, 3)?;
                    actor.enter(ctx, MOT_STATE_PICKUP, true);
                }
                item.behavior = Behavior::Pickup {
                    owner: Some(actor_id),
                    phase: state::Pickup::Transfer { actor: actor_id },
                };
                item.count = 0;
            }
            state::Pickup::Transfer { actor: collector } => {
                let actor = actors.get_mut(collector);
                if actor.motion.action != MOT_STATE_PICKUP {
                    item.rolling = Rolling {
                        velocity: [
                            (ctx.random.next15() % 200 - 100) as i16,
                            (ctx.random.next15() % 100 - 200) as i16,
                            (ctx.random.next15() % 200 - 100) as i16,
                        ],
                        ..Rolling::default()
                    };
                    item.behavior = Behavior::Pickup {
                        owner: Some(collector),
                        phase: state::Pickup::Rolling,
                    };
                }
                item.count = item.count.wrapping_add(1);
                if item.count == 10 {
                    ctx.effects.sounds.push((actor.position, 0x0d));
                    if actor.inventory[item.kind] != 255 {
                        actor.inventory[item.kind] = actor.inventory[item.kind].wrapping_add(1);
                    }
                    self.dispose(id, actors, ctx)?;
                }
            }
        }
        Ok(())
    }
}
