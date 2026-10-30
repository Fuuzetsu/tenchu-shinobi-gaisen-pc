use super::*;
use crate::actor::State as ActorState;
use crate::ids::*;
use crate::math::Position;

#[derive(Clone, Debug, Default)]
pub struct Poison {
    pub eater: Option<usize>,
    pub original_think: Option<u8>,
    pub remaining: i16,
}

impl Items {
    pub(super) fn restore_poison_target(&mut self, id: usize, owner: usize, actors: &mut Actors) {
        let item = &mut self.slots[id];
        if let Some(eater) = item
            .poison
            .eater
            .filter(|id| actors.roster.order.contains(id))
            && let Some(think) = item.poison.original_think
        {
            let owner = actors.get(owner).root_model;
            let actor = actors.get_mut(eater);
            actor.think[0] = think;
            actor.target = Some(owner);
        }
        item.poison.eater = None;
    }

    pub(super) fn poison(
        &mut self,
        id: usize,
        used: Used<state::Poison>,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        (player, clock): (usize, i32),
        ctx: &mut Context,
    ) -> Result<()> {
        let model = self.slots[id].model;
        if matches!(used.phase, state::Poison::Delay | state::Poison::Bait) {
            let mut position = ctx.scene.models[model].local.translation;
            self.slots[id].rolling.step(&mut position, ctx)?;
            ctx.scene.set_translation(model, position);
            if self.slots[id].rolling.status == RollingStatus::Water {
                return self.dispose(id, actors, ctx);
            }
        }
        if used.phase != state::Poison::Poisoned {
            self.update_coordinate(id, ctx);
            self.draw_icon(id, ctx)?;
        }
        match used.phase {
            state::Poison::Delay => {
                let poison = &mut self.slots[id].poison;
                poison.remaining = poison.remaining.wrapping_sub(1);
                if poison.remaining <= 0 {
                    self.slots[id].behavior =
                        Behavior::Poison(used.with_phase(state::Poison::Bait));
                }
            }
            state::Poison::Bait => {
                if clock & 1 != 0 {
                    return Ok(());
                }
                let item = &self.slots[id];
                let owner = used.owner;
                let position = ctx.scene.models[model].local.translation;
                let mut nearest = None;
                let mut distance = 10000;
                let mut owner_distance = 10000;
                for &target in &actors.roster.order {
                    let actor = actors.get(target);
                    let Some(range) = target_distance(actor, position, 10000, ctx) else {
                        continue;
                    };
                    if actor.kind & 0xf0 == 0x80 || range >= distance {
                        continue;
                    }
                    if target == owner {
                        owner_distance = range;
                    } else {
                        nearest = Some(target);
                        distance = range;
                    }
                }
                if owner_distance < 500 {
                    let kind = item.kind;
                    self.dispose(id, actors, ctx)?;
                    self.drop(kind, Some(owner), position, Position::ZERO, actors, ctx)?;
                    return Ok(());
                }
                let Some(eater) = nearest else {
                    return Ok(());
                };
                self.restore_poison_target(id, used.owner, actors);
                let owner = actors.get(owner).root_model;
                let actor = actors.get_mut(eater);
                let item = &mut self.slots[id];
                item.poison.eater = Some(eater);
                item.poison.original_think =
                    if actor.target == Some(owner) && actor.attribute & ATTR_PHASE == 0 {
                        let original = actor.think[0];
                        actor.target = Some(model);
                        actor.think[0] = 9;
                        Some(original)
                    } else {
                        None
                    };
                if distance >= 1000
                    || matches!(
                        actor.status,
                        ActorState::Damaged | ActorState::Transition | ActorState::Attack
                    )
                    || actor.health <= 0
                {
                    return Ok(());
                }
                if intelligence.action_halt == crate::ai::Halt::Running {
                    actor.clear_attack(ctx, 3)?;
                    Self::item_motion(actor, MOT_ITEM_DRINK, ctx);
                }
                let (part, position) = if actor.parts.len() > 14 {
                    (14, [0, 50, 0])
                } else {
                    (2, [0, 0, -150])
                };
                ctx.scene.models[model].parent = Some(actor.parts[part]);
                ctx.scene.models[model].local.translation = position.into();
                item.behavior = Behavior::Poison(used.with_phase(state::Poison::Eating));
            }
            state::Poison::Eating => {
                let Some(eater) = self.slots[id]
                    .poison
                    .eater
                    .filter(|id| actors.roster.order.contains(id))
                else {
                    return self.dispose(id, actors, ctx);
                };
                let actor = actors.get(eater);
                if actor.motion.action != MOT_ITEM_DRINK {
                    let position = ctx.scene.world(model)?.translation;
                    ctx.scene.models[model].parent = None;
                    ctx.scene.models[model].local.translation = position;
                    let velocity = [
                        (ctx.random.next15() % 200 - 100) as i16,
                        (ctx.random.next15() % 100 - 200) as i16,
                        (ctx.random.next15() % 200 - 100) as i16,
                    ];
                    let item = &mut self.slots[id];
                    item.rolling = Rolling {
                        velocity,
                        ..Rolling::default()
                    };
                    item.poison.remaining = 30;
                    item.behavior = Behavior::Poison(used.with_phase(state::Poison::Delay));
                } else if actor.motion.count == 55 {
                    self.restore_poison_target(id, used.owner, actors);
                    self.slots[id].poison.eater = Some(eater);
                    let actor = actors.get_mut(eater);
                    let action = actor.return_normal(ctx, player)?;
                    actor.enter(ctx, action, true);
                    self.slots[id].poison.remaining = 600;
                    self.slots[id].behavior =
                        Behavior::Poison(used.with_phase(state::Poison::Poisoned));
                } else if actor.attribute & ATTR_WEAPON_DRAWN != 0 && actor.kind & 0xf0 != 0xa0 {
                    let actor = actors.get_mut(eater);
                    let action = actor.return_normal(ctx, player)?;
                    actor.enter(ctx, action, true);
                }
            }
            state::Poison::Poisoned => {
                let Some(eater) = self.slots[id]
                    .poison
                    .eater
                    .filter(|id| actors.roster.order.contains(id))
                else {
                    return self.dispose(id, actors, ctx);
                };
                let poison = &mut self.slots[id].poison;
                poison.remaining = poison.remaining.wrapping_sub(1);
                let actor = actors.get_mut(eater);
                if poison.remaining == 0 || actor.health <= 0 {
                    return self.dispose(id, actors, ctx);
                }
                if matches!(
                    actor.status,
                    ActorState::Damaged
                        | ActorState::Transition
                        | ActorState::Attack
                        | ActorState::UseItem
                ) || ctx.random.next15() % 30 >= 3
                {
                    return Ok(());
                }
                if intelligence.action_halt == crate::ai::Halt::Running {
                    actor.clear_attack(ctx, 3)?;
                    let action = if actor.kind & 0xf0 == 0xa0 {
                        MOT_DAMAGE
                    } else {
                        MOT_DAMAGE_CHOKE
                    };
                    Self::item_motion(actor, action, ctx);
                    actor.status = ActorState::UseItem;
                }
                intelligence.voices.push((eater, 6));
            }
        }
        Ok(())
    }
}
