use super::{Assets, World};
use crate::{
    Result,
    actor::{Context, State as SourceState},
    game::{Game, events::Outcome as SourceOutcome},
    ids::{ATTR_SUSPEND, ATTR_WEAPON_DRAWN, MOT_DEAD, MOT_ENGAGE_STANCE, MOT_NORMAL},
    tenchu2::{
        actor::{State, Velocity},
        mission::Outcome,
        rig::WeaponMode,
    },
};
use tenchu_assets::{script::Command, tenchu2::campaign::Objective};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Director {
    pub source: Game,
    pub presenting: bool,
    bindings: Vec<Option<usize>>,
    affected: Vec<usize>,
}

impl Director {
    pub fn new(mut source: Game, world: &mut World, assets: &Assets) -> Result<Self> {
        if source.trial.is_some()
            || source.tick != 0
            || world.tick != 0
            || world.mission.objective != Objective::Scripted
            || assets.level.gaisen.as_ref().map(|s| s.id) != Some(source.mission.stage.id)
        {
            return Err("Gaisen director requires a fresh matching scripted world".into());
        }
        // Placements already belong to the native inventory. The cinematic world
        // must neither collect them again nor show supplies that are already gone.
        for item in &mut source.items.slots {
            if matches!(item.behavior, crate::items::Behavior::Pickup { .. }) {
                source.collision.remove(&mut source.scene, item.model);
                item.behavior = crate::items::Behavior::Inactive;
            }
        }
        let mut bindings = vec![None; source.roster.actors.len()];
        bindings[source.player_id] = Some(world.player);
        for (index, agent) in world
            .actors
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != world.player)
        {
            let id = usize::try_from(agent.slot).map_err(|_| "Invalid Gaisen actor binding")?;
            if source
                .roster
                .actors
                .get(id)
                .and_then(Option::as_ref)
                .is_none()
                || bindings.get(id).is_none_or(Option::is_some)
            {
                return Err("Invalid or duplicate Gaisen actor binding".into());
            }
            bindings[id] = Some(index);
        }
        // Native counterparts retain their own health. Scale only positive
        // script thresholds, preserving exact comparisons and hidden/dead values.
        for event in &mut source.mission.scripts.events {
            if event.mode != 5 || event.value <= 0 {
                continue;
            }
            let id = if event.target == 255 {
                Some(source.player_id)
            } else {
                source.roster.find(i16::from(event.target))
            };
            if let Some(id) = id
                && let Some(index) = bindings[id]
            {
                let maximum = source.roster.get(id).max_health;
                if maximum > 0 {
                    event.value = (i32::from(event.value)
                        * i32::from(world.actors[index].actor.max_health)
                        / i32::from(maximum)) as i16;
                }
            }
        }
        let mut population = world.actors.clone();
        let mut original_bosses = 0;
        for (id, index) in bindings
            .iter()
            .enumerate()
            .filter_map(|(id, i)| i.map(|i| (id, i)))
        {
            let original = source.roster.get_mut(id);
            if original.max_health < 0 {
                population[index].actor.health = -1;
                world.actors[index].actor.invulnerable = true;
            } else if original.kind & 0xf0 == 0x80 {
                original_bosses += 1;
            }
            original.max_health = world.actors[index].actor.max_health;
            if original.health == -1 {
                world.actors[index].actor.health = -1;
                world.set_script_visible(index, false, assets)?;
            }
        }
        world.mission.population = Default::default();
        world.mission.initialize(&population, assets.hero);
        world.mission.population.bosses = world
            .mission
            .population
            .bosses
            .saturating_sub((original_bosses - source.statistics.bosses).max(0) as u8);
        world.scene.finish_tick();
        world.collision.compute(|id| world.scene.world(id))?;
        Ok(Self {
            source,
            presenting: false,
            bindings,
            affected: Vec::new(),
        })
    }

    pub fn native_actor(&self, source_actor: usize) -> Option<usize> {
        self.bindings.get(source_actor).copied().flatten()
    }

    /// Run after a native gameplay update. Original ambient callbacks retain
    /// their order and shared RNG; native mechanisms and scrolling run in World.
    pub fn advance_ambient(&mut self, world: &mut World) -> Result<()> {
        if self.presenting {
            return Err("Cannot advance native ambience during an original cinematic".into());
        }
        let source = &mut self.source;
        source.clock = world.tick as i32;
        source.random = world.random;
        source.camera.position = world.camera.eye;
        source.camera.reference = world.camera.look;
        source.camera.view = world.camera.view;
        source.rendered_view = world.camera.view;
        source.effects.step_ambient(
            &mut source.random,
            crate::effects::View {
                position: source.camera.position,
                matrix: source.camera.view,
            },
            &mut source.scene,
            &mut crate::effects::Environment {
                terrain: &source.mission.terrain,
                cursor: &mut source.cursor,
                clock: source.clock,
                owner_position: world.actors[world.player].actor.position,
                reference: source.camera.reference,
            },
        )?;
        source.misc.step_ambient(
            source.clock,
            &mut Context {
                mission: &mut source.mission,
                math: &source.math,
                body: &source.body,
                cursor: &mut source.cursor,
                random: &mut source.random,
                effects: &mut source.effects,
                scene: &mut source.scene,
                collision: &mut source.collision,
                camera: &mut source.camera,
                drawing: &mut source.drawing,
            },
        )?;
        world.random = source.random;
        Ok(())
    }

    /// Returns true when this tick belongs to the original cinematic. The caller
    /// advances the native world and mission only on false. Input uses T2 pad order.
    pub fn update(&mut self, world: &mut World, assets: &Assets, input: u16) -> Result<bool> {
        let entering = !self.source.cinematic.active;
        if entering {
            self.synchronize(world)?;
        }
        self.presenting = self.source.step_cinematic(input.swap_bytes())?;
        if self.presenting {
            if entering {
                self.affected.clear();
                let (start, _) = self
                    .source
                    .mission
                    .scripts
                    .cutscenes
                    .sequence(self.source.cinematic.sequence)
                    .ok_or("Missing Gaisen cinematic")?;
                for command in self
                    .source
                    .mission
                    .scripts
                    .cutscenes
                    .commands
                    .iter()
                    .skip(start + 1)
                {
                    if matches!(command, Command::Wait { until: 0 }) {
                        break;
                    }
                    if let Command::Actor { id, .. } = command
                        && let Some(id) = self.source.roster.find(*id)
                        && self.bindings[id].is_some()
                        && !self.affected.contains(&id)
                    {
                        self.affected.push(id);
                    }
                }
            }
            if !self.source.cinematic.active {
                self.resume(world, assets)?;
            }
        } else {
            world.random = self.source.random;
            match self.source.events.outcome {
                SourceOutcome::Running => {}
                SourceOutcome::Complete => world.mission.resolve_script(Outcome::AreaCleared)?,
                SourceOutcome::GameOver => world.mission.resolve_script(Outcome::Dead)?,
            }
        }
        Ok(self.presenting)
    }

    fn synchronize(&mut self, world: &World) -> Result<()> {
        let source = &mut self.source;
        source.clock = world.tick as i32;
        source.random = world.random;
        source.effects.texture_moves.clear();
        for effect in source.effects.slots.iter_mut().flatten() {
            if let crate::effects::Effect::TextureScroll(scroll) = effect
                && let Some(native) = world
                    .environment
                    .texture_scrolls
                    .iter()
                    .find(|s| s.destination == scroll.destination)
            {
                *scroll = native.clone();
                source.mission.vram.copy_rect(
                    [
                        scroll.source[0].wrapping_add((scroll.offset[0] / 16) as u16),
                        scroll.source[1].wrapping_add((scroll.offset[1] / 16) as u16),
                        scroll.destination[2],
                        scroll.destination[3],
                    ],
                    [scroll.destination[0], scroll.destination[1]],
                )?;
            }
        }
        source.intelligence.found = i16::from(world.ai.detections);
        source.statistics.criticals = i16::from(world.statistics.stealth_kills);
        source.statistics.murders = i16::from(world.statistics.combat_kills);
        source.statistics.friend_hits = i16::from(world.statistics.innocents);
        source.scene.finish_tick();
        let context = Context {
            mission: &mut source.mission,
            math: &source.math,
            body: &source.body,
            cursor: &mut source.cursor,
            random: &mut source.random,
            effects: &mut source.effects,
            scene: &mut source.scene,
            collision: &mut source.collision,
            camera: &mut source.camera,
            drawing: &mut source.drawing,
        };
        for (id, index) in self
            .bindings
            .iter()
            .enumerate()
            .filter_map(|(id, i)| i.map(|i| (id, i)))
        {
            let Some(actor) = source.roster.actors.get_mut(id).and_then(Option::as_mut) else {
                return Err("Gaisen cinematic removed a bound actor".into());
            };
            let native = &world.actors[index].actor;
            actor.position = native.position;
            actor.previous = native.position;
            actor.rotation = native.rotation.to_array();
            actor.velocity = native.velocity.to_array();
            actor.health = native.health;
            if native.flags & 8 != 0 {
                actor.attribute |= ATTR_SUSPEND;
            } else {
                actor.attribute &= !ATTR_SUSPEND;
            }
            actor.equip(&context, native.weapon_drawn());
            let status = match native.state {
                State::Normal => SourceState::Normal,
                State::Ready => SourceState::Engage,
                State::Crouched => SourceState::Crouch,
                State::Wall => SourceState::Wall,
                State::Ledge => SourceState::Hang,
                State::Swimming => SourceState::Swim,
                State::Jumping => SourceState::Jump,
                State::Special => SourceState::Action,
                State::Item => SourceState::UseItem,
                State::Attack => SourceState::Attack,
                State::Hurt => SourceState::Damaged,
                State::Dead => SourceState::Dead,
            };
            if actor.status != SourceState::Dead || status != SourceState::Dead {
                actor.status = SourceState::Normal;
                actor.motion.loops = 0;
                actor.enter(
                    &context,
                    if status == SourceState::Dead {
                        MOT_DEAD
                    } else if native.weapon_drawn() {
                        MOT_ENGAGE_STANCE
                    } else {
                        MOT_NORMAL
                    },
                    false,
                );
            }
            actor.status = status;
            // Event death gates use loop/frame state. Live T2 animation phases
            // cannot index the unrelated original cinematic clips.
            let dead = status == SourceState::Dead;
            actor.motion.loops = if !dead {
                0
            } else if native.motion.loops < 0 {
                -1
            } else {
                native.motion.loops
            };
            actor.motion.count = if dead {
                let duration = context.mission.clips
                    [actor.motion.clip.ok_or("Missing source death clip")?]
                .duration;
                native.motion.frame.min(duration)
            } else {
                0
            };
            context
                .scene
                .update(actor.root_model, actor.root(context.math));
        }
        Ok(())
    }

    fn resume(&mut self, world: &mut World, assets: &Assets) -> Result<()> {
        world.random = self.source.random;
        for effect in self.source.effects.slots.iter().flatten() {
            if let crate::effects::Effect::TextureScroll(scroll) = effect
                && let Some(native) = world
                    .environment
                    .texture_scrolls
                    .iter_mut()
                    .find(|s| s.destination == scroll.destination)
            {
                *native = scroll.clone();
            }
        }
        world.scene.finish_tick();
        for &id in &self.affected {
            let index = self.bindings[id].ok_or("Missing Gaisen cinematic binding")?;
            let source = self
                .source
                .roster
                .actors
                .get(id)
                .and_then(Option::as_ref)
                .ok_or("Gaisen cinematic removed a bound actor")?;
            let visible = source.health >= 0 && source.attribute & ATTR_SUSPEND == 0;
            let actor = &mut world.actors[index].actor;
            if source.health == 0 && actor.health > 0 {
                super::controls::credit(
                    &mut world.statistics,
                    &mut world.script,
                    actor,
                    index == world.player,
                    false,
                    false,
                );
            }
            let corpse = actor.state == State::Dead && actor.motion.loops == i16::MIN;
            actor.position = source.position;
            actor.previous = source.position;
            actor.rotation = source.rotation.into();
            actor.health = source.health;
            actor.velocity = Default::default();
            actor.last_vertical_velocity = 0;
            actor.hit = None;
            actor.target = (index != world.player).then_some(world.player);
            actor.pad = Default::default();
            actor.attack = Default::default();
            use crate::tenchu2::physics;
            actor.flags &= !(physics::HIT
                | physics::FALLING
                | physics::IN_WATER
                | physics::CEILING
                | physics::BLOCKED
                | physics::LANDED
                | physics::UNEVEN
                | physics::TOUCHING
                | physics::TOUCHING_PLAYER);
            if visible {
                actor.terrain.sample(
                    &world.terrain,
                    &mut Default::default(),
                    &assets.terrain_rules,
                    actor.position.wrapping_sub(crate::math::Position::new(
                        0,
                        i32::from(actor.height),
                        0,
                    )),
                    actor.width,
                    world.terrain_options,
                )?;
            }
            let rig = &mut world.instances[index].rig;
            actor.set_weapon(
                rig,
                index,
                &mut world.collision,
                &assets.resources,
                if source.attribute & ATTR_WEAPON_DRAWN != 0 {
                    WeaponMode::Draw
                } else {
                    WeaponMode::Sheathe
                },
            )?;
            if !corpse {
                let action = if actor.health == 0 {
                    if actor.state == State::Dead {
                        actor.motion.action
                    } else if assets
                        .resources
                        .registry
                        .clip(actor.definition_kind(), 0xb00)
                        .is_some()
                    {
                        0xb00
                    } else {
                        0xb01 + (world.random.next15() % 5) as i16
                    }
                } else if actor.weapon_drawn() {
                    0x100
                } else {
                    0
                };
                actor
                    .change_motion(action, Velocity::Stop, &assets.resources)?
                    .ok_or("Missing native motion after Gaisen cinematic")?;
                actor.motion.frame = if actor.health == 0 {
                    assets.resources.clips[&actor.motion.clip.ok_or("Missing native death clip")?]
                        .duration
                } else {
                    0
                };
                actor
                    .motion
                    .advance(&assets.resources.clips, &assets.spline, true)?;
            }
            actor.set_weapon(
                rig,
                index,
                &mut world.collision,
                &assets.resources,
                WeaponMode::Remove,
            )?;
            world.actors[index].behavior.home = source.position;
            world.set_script_visible(index, visible, assets)?;
            world.publish(index, assets);
        }
        world.controls.focus = None;
        world.controls.stealth = None;
        world.controls.camera_actor = world.player;
        world.tracking = None;
        world.cursor = Default::default();
        world.probe = Default::default();
        world.water = Default::default();
        world.collision.compute(|id| world.scene.world(id))?;
        world.update_camera(assets, true)?;
        Ok(())
    }
}
