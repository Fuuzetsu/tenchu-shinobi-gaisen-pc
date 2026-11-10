use super::{
    Assets, World,
    controls::{Event, Events},
};

mod markers;
mod rice;
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{State as ActorState, Velocity},
        ai::status::{Alert, AlertCause},
        collision::{BODY, WEAPON},
        effects::{
            Effects,
            ribbon::{Sample, Source},
            surface::{Blend, Image, Quad},
        },
        items::{Context, Event as ItemEvent, State, grapple, projectile::Phase},
    },
};

impl World {
    pub(super) fn advance_items(
        &mut self,
        assets: &Assets,
        events: &mut impl Events,
        frame: &mut crate::tenchu2::items::Frame,
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
            let mut resurrection_voice = None;
            let (mut alive, pickup) = match &mut entry.state {
                State::Pickup(pickup) => {
                    pickup.advance(model, &mut context)?;
                    (true, !pickup.falling)
                }
                State::Spear(spear) => (
                    spear.advance(model, &mut context, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Cloud(cloud) => (
                    cloud.advance(model, context.collision, |event| {
                        sink.emit(event, &mut self.random)
                    })?,
                    false,
                ),
                State::Resurrection(resurrection) => {
                    let owner = resurrection.owner;
                    let update = resurrection.advance(
                        &mut self.actors[owner].actor,
                        &mut self.items.return_point,
                        self.controls.transition_ready,
                        context.math,
                        &mut self.random,
                    );
                    context.scene.update(model, resurrection.transform);
                    if let Some(fade) = update.fade {
                        frame.fade = Some(fade);
                    }
                    if let Some(position) = update.smoke {
                        sink.effects.dispersing_smoke(
                            position,
                            Position::new(0, -8, 0),
                            2,
                            &mut self.random,
                        )?;
                    }
                    if let Some(position) = update.voice {
                        resurrection_voice = resurrection.attacker.map(|actor| (actor, position));
                    }
                    if update.returned {
                        self.environment.background_only = false;
                        self.camera.snap = true;
                        self.camera.mode = 0;
                        self.instances[owner].translucent = false;
                    }
                    if !update.alive {
                        self.script = 0;
                    }
                    (update.alive, false)
                }
                State::Marker(marker) => (marker.advance(model, &mut context)?, false),
                State::Teleport(teleport) => {
                    let target = teleport.target.map(|i| {
                        let actor = &self.actors[i].actor;
                        (actor.position, actor.rotation.y)
                    });
                    (
                        teleport.advance(
                            &mut self.actors[teleport.owner].actor,
                            target,
                            &mut self.camera.mode,
                            &mut context,
                        )?,
                        false,
                    )
                }
                State::FireBreath(breath) => (
                    breath.advance(
                        self.actors[breath.owner].actor.rotation.y,
                        &assets.resources.math,
                        |event| sink.emit(event, &mut self.random),
                    )?,
                    false,
                ),
                State::FireBlast(blast) => (blast.advance(model, context.collision)?, false),
                State::Invisibility(invisibility) => {
                    let owner = invisibility.owner;
                    let trail = invisibility.trail;
                    let alive = invisibility
                        .advance(&mut self.script, &mut self.instances[owner].translucent);
                    let source = Source::Transformation(model);
                    if trail && (!alive || !invisibility.trail) {
                        sink.effects.sample_ribbon(source, None);
                    }
                    if invisibility.age < 70 {
                        let position = self.actors[owner].actor.position;
                        let mut corners = [crate::math::Vector3::default(); 4];
                        for (corner, bone) in corners.iter_mut().zip([11, 5, 14, 8]) {
                            let joint = *self.instances[owner]
                                .rig
                                .joints
                                .get(bone)
                                .ok_or("Missing Tenchu 2 invisibility joint")?;
                            *corner = context
                                .scene
                                .world(joint)?
                                .translation
                                .wrapping_sub(position)
                                .map(|v| v as i16);
                        }
                        let matrix = view.matrix.compose(&crate::math::Matrix {
                            translation: position,
                            ..Default::default()
                        });
                        let a = crate::visibility::project(&matrix, corners[0].to_array());
                        let b = crate::visibility::project(&matrix, corners[1].to_array());
                        sink.effects.sample_ribbon(
                            source,
                            Some(Sample {
                                edges: [a.0, b.0],
                                depth: i32::from(a.1 >> 4),
                            }),
                        );
                        frame.quads.push(Quad {
                            blood: None,
                            image: Image::Effect(0),
                            position,
                            parent: None,
                            corners,
                            color: [128; 3],
                            blend: Blend::Add,
                        });
                    }
                    (alive, false)
                }
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
            let blasts = sink.blasts;
            for (position, id) in sink.sounds {
                self.emit(self.player, Event::SoundPosition { position, id }, events)?;
            }
            if let Some((actor, position)) = resurrection_voice {
                let listener = crate::tenchu2::audio::Listener {
                    position,
                    ..self.listener()
                };
                Events::emit(
                    events,
                    super::controls::EventSource {
                        listener,
                        voice: self.instances[actor].voice,
                        index: actor,
                        actor: &self.actors[actor].actor,
                        rig: &self.instances[actor].rig,
                        scene: &mut self.scene,
                    },
                    Event::Voice(9),
                    &mut self.random,
                )?;
            }
            if alive && matches!(self.items.entries[index].state, State::Rice(_)) {
                alive = self.update_rice(index, assets, events)?;
            }
            if matches!(self.items.entries[index].state, State::Marker(_)) {
                alive &= !self.pick_up_marker(index, assets, events)?;
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
            for blast in blasts {
                if self.items.entries.len() == 30 {
                    index = index.saturating_sub(1);
                }
                self.items.insert(
                    State::FireBlast(blast),
                    &mut self.scene,
                    &mut self.collision,
                )?;
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
            State::Pickup(p) => (usize::from(p.item), p.flight.position, None),
            State::Projectile(p) => (p.model() as usize, p.flight.position, Some(p.owner)),
            State::Grenade(g) => (6, g.flight.position, Some(g.owner)),
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
            if dangerous && Some(taker) != owner {
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
                self.controls
                    .select_item(&mut actor.inventory, false, taker == self.player);
            }
            self.controls
                .select_item(&mut actor.inventory, available, taker == self.player);
            if taker == self.player {
                self.emit(taker, Event::Pickup(item), events)?;
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
    blasts: Vec<crate::tenchu2::items::fire::Blast>,
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
            blasts: Vec::new(),
        }
    }

    fn emit(&mut self, cue: ItemEvent, random: &mut Random) -> Result<()> {
        let math = &self.assets.resources.math;
        match cue {
            ItemEvent::FireBlast { owner, position } => {
                self.blasts.push(crate::tenchu2::items::fire::Blast {
                    owner,
                    position,
                    age: 0,
                });
                self.emit(
                    ItemEvent::Explosion {
                        position,
                        kind: 2,
                        contact: true,
                    },
                    random,
                )?;
            }
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
            ItemEvent::DispersingSmoke {
                position,
                direction,
                kind,
            } => self
                .effects
                .dispersing_smoke(position, direction, kind, random)?,
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
