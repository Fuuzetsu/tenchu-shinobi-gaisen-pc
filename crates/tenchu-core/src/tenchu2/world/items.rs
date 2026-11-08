use super::{
    Assets, World,
    controls::{Event, Events},
};
use crate::{
    Result,
    tenchu2::{
        actor::{State as ActorState, Velocity},
        ai::status::AlertCause,
        collision::{BODY, WEAPON},
        items::{Context, Event as ItemEvent, State, grapple, projectile::Phase},
    },
};

impl World {
    pub(super) fn advance_items(
        &mut self,
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<()> {
        let mut index = 0;
        while index < self.items.entries.len() {
            let entry = &mut self.items.entries[index];
            let model = entry.model;
            let mut cues = Vec::new();
            let mut context = Context {
                map: &self.terrain,
                cursor: &mut self.cursor,
                math: &assets.resources.math,
                scene: &mut self.scene,
                collision: &mut self.collision,
                player: self.player,
            };
            let (mut alive, pickup) = match &mut entry.state {
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
                            cues.push(ItemEvent::Impact {
                                position,
                                direction,
                            });
                            cues.push(ItemEvent::Sound { position, id: 49 });
                        }
                        (true, false)
                    }
                }
                State::Projectile(projectile) => {
                    let pickup = projectile.phase == Phase::Resting;
                    let alive = projectile.advance(model, &mut context, |event| {
                        cues.push(event);
                        Ok(())
                    })?;
                    (alive, pickup)
                }
                State::Arrow(arrow) => (
                    arrow.advance(model, self.tick, &mut context, |event| {
                        cues.push(event);
                        Ok(())
                    })?,
                    false,
                ),
                State::Gunshot(shot) => (
                    shot.advance(model, &mut context, |event| {
                        cues.push(event);
                        Ok(())
                    })?,
                    false,
                ),
            };
            for cue in cues {
                match cue {
                    ItemEvent::Explosion { position, contact } => {
                        self.effects.explosion(
                            position,
                            1,
                            &assets.resources.math,
                            &mut self.random,
                        )?;
                        self.ai.global.perception.alert.raise(
                            if contact {
                                AlertCause::Corpse
                            } else {
                                AlertCause::Combat
                            },
                            self.difficulty,
                            &assets.combat_rules,
                        )?;
                        self.ai.global.perception.alert.position = Some(position);
                    }
                    ItemEvent::Smoke(position) => self.effects.smoke(
                        position,
                        3,
                        self.hidden_scene,
                        &assets.resources.math,
                        &mut self.random,
                    )?,
                    ItemEvent::Splash(position) => {
                        self.effects.ripple(position, 30)?;
                        self.effects.splash(
                            position,
                            1024,
                            &assets.resources.math,
                            &mut self.random,
                        )?;
                    }
                    ItemEvent::Impact {
                        position,
                        direction,
                    } => {
                        self.effects.impact(
                            position,
                            direction,
                            &assets.resources.math,
                            &assets.effect_rules,
                            &mut self.random,
                        )?;
                    }
                    ItemEvent::Sound { position, id } => {
                        self.emit(self.player, Event::SoundPosition { position, id }, events)?;
                    }
                    ItemEvent::Alert(position) => {
                        let alert = &mut self.ai.global.perception.alert;
                        alert.raise(
                            AlertCause::Suspicion,
                            self.difficulty,
                            &assets.combat_rules,
                        )?;
                        alert.position = Some(position);
                    }
                }
            }
            if alive && pickup {
                alive = !self.pick_up_item(index, assets, events)?;
            }
            if alive {
                index += 1;
            } else {
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
        let State::Projectile(projectile) = &entry.state else {
            return Ok(false);
        };
        let item = projectile.model() as usize;
        let position = projectile.flight.position;
        let dangerous = self
            .collision
            .get_mut(entry.model)
            .is_some_and(|slot| slot.flags & WEAPON != 0);
        let owner = projectile.owner;
        let taker = self.collision.contacts(entry.model).find_map(|contact| {
            let slot = self.collision.slot(contact.index)?;
            if slot.flags & BODY == 0 || (dangerous && slot.owner != Some(owner)) {
                return None;
            }
            let index = slot.owner?;
            let actor = &self.actors.get(index)?.actor;
            (actor.kind & 0xf0 != 0x70
                && !matches!(actor.kind, 0xe5 | 0x1a)
                && actor.flags & 0xf00 == 0
                && matches!(
                    actor.state,
                    ActorState::Normal
                        | ActorState::Ready
                        | ActorState::Crouched
                        | ActorState::Wall
                        | ActorState::Ledge
                        | ActorState::Swimming
                )
                && actor.motion.action != 0x513)
                .then_some(index)
        });
        let Some(taker) = taker else {
            return Ok(false);
        };
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
        Ok(true)
    }
}
