use super::{
    Assets, World,
    controls::{Event, Events},
};

mod rice;
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{State as ActorState, Velocity},
        ai::status::{Alert, AlertCause},
        collision::{BODY, WEAPON},
        effects::{Effects, ribbon::Source},
        items::{Context, Event as ItemEvent, State, grapple, projectile::Phase},
    },
};

impl World {
    pub(super) fn advance_items(
        &mut self,
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<()> {
        let view = self.view();
        let mut index = 0;
        while index < self.items.entries.len() {
            let entry = &mut self.items.entries[index];
            let model = entry.model;
            let mut sink = ItemEffects::new(
                assets,
                &mut self.effects,
                &mut self.ai.global.perception.alert,
                self.difficulty,
                self.hidden_scene,
            );
            let mut context = Context {
                map: &self.terrain,
                cursor: &mut self.cursor,
                math: &assets.resources.math,
                scene: &mut self.scene,
                collision: &mut self.collision,
                player: self.player,
            };
            let (mut alive, pickup) = match &mut entry.state {
                State::Powder(powder) => (powder.advance(), false),
                State::SleepBomb(bomb) => (
                    bomb.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Dart(dart) => {
                    let alive = dart.advance(model, view, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?;
                    if !alive || !dart.trail {
                        sink.effects.sample_ribbon(Source::Projectile(model), None);
                    }
                    (alive, false)
                }
                State::Rice(rice) => (
                    rice.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Grapple(grapple) => {
                    if grapple.state == grapple::State::Released {
                        (false, false)
                    } else {
                        let hand = *self.instances[self.player]
                            .rig
                            .joints
                            .get(11)
                            .ok_or("Missing Tenchu 2 grapple hand")?;
                        let hand = context.scene.world(hand)?.translation;
                        if let Some(direction) =
                            grapple.advance(self.items.aim.target, hand, &assets.resources.math)
                        {
                            let position = grapple.transform.translation;
                            sink.emit(
                                ItemEvent::Impact {
                                    position,
                                    direction,
                                },
                                &mut self.random,
                            )?;
                            sink.emit(ItemEvent::Sound { position, id: 49 }, &mut self.random)?;
                        }
                        (true, false)
                    }
                }
                State::Projectile(projectile) => {
                    let pickup = projectile.phase == Phase::Resting;
                    let alive =
                        projectile.advance(model, &self.camera.view, &mut context, |event| {
                            sink.emit(event, &mut self.random)
                        })?;
                    if !alive || !projectile.trail {
                        sink.effects.sample_ribbon(Source::Projectile(model), None);
                    }
                    (alive, pickup)
                }
                State::Arrow(arrow) => (
                    arrow.advance(model, self.tick, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Grenade(grenade) => {
                    let pickup = grenade.phase == Phase::Resting;
                    let alive = grenade.advance(
                        model,
                        &mut context,
                        &mut self.random,
                        |event, random| sink.emit(event, random),
                    )?;
                    (alive, pickup)
                }
                State::Mine(mine) => (
                    mine.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Smoke(smoke) => (
                    smoke.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Caltrop(caltrop) => (
                    caltrop.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Gunshot(shot) => (
                    shot.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
            };
            for (position, id) in sink.sounds {
                self.emit(self.player, Event::SoundPosition { position, id }, events)?;
            }
            if alive && matches!(self.items.entries[index].state, State::Rice(_)) {
                alive = self.update_rice(index, assets, events)?;
            }
            if alive && pickup {
                alive = !self.pick_up_item(index, assets, events)?;
            }
            if alive {
                index += 1;
            } else {
                for agent in &mut self.actors {
                    if agent.behavior.bait.is_some_and(|bait| bait.model == model) {
                        agent.behavior.release_bait(&mut agent.actor, self.player);
                    }
                }
                self.items.remove(index, &mut self.collision);
            }
        }
        Ok(())
    }

    fn pick_up_item(
        &mut self,
        index: usize,
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<bool> {
        let entry = &self.items.entries[index];
        let (item, position, owner) = match &entry.state {
            State::Projectile(p) => (p.model() as usize, p.flight.position, p.owner),
            State::Grenade(g) => (6, g.flight.position, g.owner),
            _ => return Ok(false),
        };
        let model = entry.model;
        let contacts: Vec<_> = self.collision.contacts(model).collect();
        for contact in contacts {
            let slot = self
                .collision
                .slot(contact.index)
                .ok_or("Missing Tenchu 2 pickup contact")?;
            if slot.flags & BODY == 0 {
                if slot.flags & 0x100 != 0
                    && let State::Grenade(grenade) = &mut self.items.entries[index].state
                {
                    let cue = grenade.ignite(model, &mut self.collision)?;
                    let mut sink = ItemEffects::new(
                        assets,
                        &mut self.effects,
                        &mut self.ai.global.perception.alert,
                        self.difficulty,
                        self.hidden_scene,
                    );
                    sink.emit(cue, &mut self.random)?;
                    for (position, id) in sink.sounds {
                        self.emit(self.player, Event::SoundPosition { position, id }, events)?;
                    }
                }
                continue;
            }
            let Some(taker) = slot.owner else {
                continue;
            };
            let dangerous = self
                .collision
                .get_mut(model)
                .is_some_and(|s| s.flags & WEAPON != 0);
            if dangerous && taker != owner {
                continue;
            }
            let actor = &self.actors[taker].actor;
            if actor.kind & 0xf0 == 0x70
                || matches!(actor.kind, 0xe5 | 0x1a)
                || actor.flags & 0xf00 != 0
                || !matches!(
                    actor.state,
                    ActorState::Normal
                        | ActorState::Ready
                        | ActorState::Crouched
                        | ActorState::Wall
                        | ActorState::Ledge
                        | ActorState::Swimming
                )
                || actor.motion.action == 0x513
            {
                continue;
            }
            let actor = &mut self.actors[taker].actor;
            if !matches!(self.camera.mode, 19 | 17) && actor.flags & 0x1000 == 0 {
                let action = if (0x533..=0x535).contains(&actor.motion.action) {
                    0x535
                } else {
                    0x800
                };
                actor.change_motion(action, Velocity::Motion, &assets.resources)?;
            }
            actor.inventory.counts[item] = actor.inventory.counts[item].wrapping_add(1);
            let available = actor
                .inventory
                .selected
                .is_some_and(|i| actor.inventory.count(i) != 0);
            if available {
                actor.inventory.select(false);
            }
            actor.inventory.select(available);
            if taker == self.player {
                self.controls.unlocked_items[item] = true;
                self.emit(taker, Event::SoundPosition { position, id: 7 }, events)?;
            }
            return Ok(true);
        }
        Ok(false)
    }
}

struct ItemEffects<'a> {
    assets: &'a Assets,
    effects: &'a mut Effects,
    alert: &'a mut Alert,
    difficulty: u8,
    hidden: bool,
    sounds: Vec<(Position, u8)>,
}

impl<'a> ItemEffects<'a> {
    fn new(
        assets: &'a Assets,
        effects: &'a mut Effects,
        alert: &'a mut Alert,
        difficulty: u8,
        hidden: bool,
    ) -> Self {
        Self {
            assets,
            effects,
            alert,
            difficulty,
            hidden,
            sounds: Vec::new(),
        }
    }

    fn emit(&mut self, cue: ItemEvent, random: &mut Random) -> Result<()> {
        let math = &self.assets.resources.math;
        match cue {
            ItemEvent::SleepGas { position, phase } => {
                self.effects.sleep_gas(position, phase, math)?
            }
            ItemEvent::Explosion {
                position,
                kind,
                contact,
            } => {
                self.effects.explosion(position, kind, math, random)?;
                if kind != 2 {
                    self.sounds.push((position, 51));
                }
                self.alert.raise(
                    if contact {
                        AlertCause::Corpse
                    } else {
                        AlertCause::Combat
                    },
                    self.difficulty,
                    &self.assets.combat_rules,
                )?;
                self.alert.position = Some(position);
            }
            ItemEvent::Smoke { position, kind } => {
                self.effects
                    .smoke(position, kind, self.hidden, math, random)?
            }
            ItemEvent::Fuse(position) => {
                self.effects
                    .fuse(position, &self.assets.effect_rules, random)?
            }
            ItemEvent::Ribbon { model, sample } => {
                self.effects
                    .sample_ribbon(Source::Projectile(model), sample);
            }
            ItemEvent::Splash(position) => {
                self.effects.ripple(position, 30)?;
                self.effects.splash(position, 1024, math, random)?;
            }
            ItemEvent::Impact {
                position,
                direction,
            } => {
                self.effects.impact(
                    position,
                    direction,
                    math,
                    &self.assets.effect_rules,
                    random,
                )?;
            }
            ItemEvent::Sound { position, id } => self.sounds.push((position, id)),
            ItemEvent::Alert(position) => {
                self.alert.raise(
                    AlertCause::Suspicion,
                    self.difficulty,
                    &self.assets.combat_rules,
                )?;
                self.alert.position = Some(position);
            }
        }
        Ok(())
    }
}
