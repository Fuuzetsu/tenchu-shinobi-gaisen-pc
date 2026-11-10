use super::{Assets, Foot, World};
use crate::{
    Result,
    math::Matrix,
    tenchu2::{
        actor::State,
        ai::{behavior::Events, direction, runtime},
        collision::{DISABLED, WEAPON},
        physics::{ContactActor, FallDeath, Terrain},
        terrain::{self, Options, Query},
    },
};

pub struct Movement {
    pub death: Option<FallDeath>,
    pub voices: Vec<u8>,
}

impl World {
    pub fn move_actor(
        &mut self,
        index: usize,
        assets: &Assets,
        events: &mut impl crate::tenchu2::environment::Events,
    ) -> Result<Movement> {
        let mut events = super::effects::EnvironmentEvents {
            effects: &mut self.effects,
            math: &assets.resources.math,
            rules: &assets.effect_rules,
            instances: &self.instances,
            events,
            spawned: Vec::new(),
        };
        let (before, tail) = self.actors.split_at_mut(index);
        let (agent, after) = tail.split_first_mut().ok_or("Missing Tenchu 2 actor")?;
        let actor = &mut agent.actor;
        let rig = &self.instances[index].rig;
        let avoid_water = index != self.player
            && actor.health != 0
            && !matches!(self.map, 0 | 9 | 16 | 18)
            && actor.motion.action != 0x606
            && actor.terrain.material & 0xf00 == 0;
        let mut terrain = Terrain {
            map: &mut self.terrain,
            rules: &assets.terrain_rules,
            cursor: &mut self.cursor,
            scratch: &mut self.probe,
            water: &mut self.water,
            options: Options {
                avoid_water,
                ignore_steps: false,
            },
        };
        let death = actor.move_on_terrain(
            &mut terrain,
            &assets.resources,
            index == self.player,
            |actor| rig.root_position(actor, &mut self.scene),
            |actor, terrain| {
                use crate::tenchu2::environment::{SurfaceActor, SurfaceContext};
                let focused = index == self.controls.camera_actor;
                let kind = self.environment.surface(
                    SurfaceActor {
                        index,
                        position: actor.position,
                        height: actor.height,
                        health: actor.health,
                        visible: actor.flags & 0x100000 != 0,
                        focused,
                    },
                    &actor.terrain,
                    self.hidden_scene,
                    &mut SurfaceContext {
                        map: terrain.map,
                        cursor: terrain.cursor,
                        math: &assets.resources.math,
                        random: &mut self.random,
                        events: &mut events,
                    },
                )?;
                if kind == 19 && focused && actor.health < 1 {
                    self.camera.mode = 19;
                }
                Ok(kind)
            },
        )?;
        self.terrain_options = terrain.options;
        events.publish_items(&mut self.items, &mut self.scene, &mut self.collision)?;
        let voices = actor.resolve_contacts(
            &self.collision,
            rig.joints[0],
            &assets.resources,
            &mut self.random,
            |id| {
                let other = if id < index {
                    before.get(id)
                } else {
                    id.checked_sub(index + 1).and_then(|id| after.get(id))
                }
                .ok_or("Missing Tenchu 2 collision actor")?;
                Ok(ContactActor {
                    position: other.actor.position,
                    action: other.actor.motion.action,
                    is_player: id == self.player,
                })
            },
        )?;
        Ok(Movement { death, voices })
    }

    pub fn decide_actor(
        &mut self,
        index: usize,
        controllers: [u16; 2],
        assets: &Assets,
        events: &mut impl Events,
    ) -> Result<u16> {
        let held = self.ai.update(
            index,
            &mut self.actors,
            &mut runtime::Frame {
                player: self.player,
                map: self.map,
                sequence: self.sequence,
                script: self.script,
                tick: self.tick,
                difficulty: self.difficulty,
                controllers,
                resources: &assets.resources,
                rules: &assets.combat_rules,
                terrain: &self.terrain,
                cursor: &mut self.cursor,
                collision: &self.collision,
                random: &mut self.random,
                events,
            },
        )?;
        self.sync_attack_contacts(index);
        Ok(held)
    }

    pub fn sync_attack_contacts(&mut self, index: usize) {
        let actor = &self.actors[index].actor;
        for (bone, &joint) in self.instances[index].rig.joints.iter().enumerate() {
            if let Some(slot) = self.collision.get_mut(joint)
                && slot.flags & WEAPON != 0
                && actor.attack.contacts & 1u32.wrapping_shl(bone as u32) == 0
            {
                slot.flags |= DISABLED;
            }
        }
    }

    pub fn animate_actor(&mut self, index: usize, assets: &Assets) -> Result<()> {
        let view = self.view();
        let actor = &mut self.actors[index].actor;
        if actor.flags & 8 != 0 {
            return Ok(());
        }
        let instance = &mut self.instances[index];
        let rig = &instance.rig;
        let math = &assets.resources.math;
        self.scene.update(
            rig.root,
            Matrix {
                rotation: math.rotate_yxz(actor.rotation.to_array()),
                translation: actor.position,
            },
        );
        let visible =
            index == self.player || view.actor(&self.scene.world(rig.joints[0])?).is_some();
        if index == self.player {
            self.effort = self
                .effort
                .wrapping_add(actor.motion_changes.wrapping_sub(instance.motion_changes));
            self.travel = self
                .travel
                .wrapping_add(math.length(actor.velocity.map(i32::from)));
            if self.travel > 2000 {
                self.effort = self.effort.wrapping_add(1);
                self.travel %= 2000;
            }
        }
        instance.motion_changes = actor.motion_changes;
        let update_pose = self.keep_all_active
            || visible
            || matches!(
                actor.state,
                State::Special | State::Attack | State::Hurt | State::Dead
            );
        let playing = actor.motion.loops >= 0;
        actor
            .motion
            .advance(&assets.resources.clips, &assets.spline, update_pose)?;
        if update_pose && playing {
            actor.motion.publish(&mut self.scene, &rig.joints, math);
        }
        if !visible {
            actor.flags &= !0x100000;
            if !self.keep_all_active {
                return Ok(());
            }
        } else {
            actor.flags |= 0x100000;
        }
        if actor.health == 0 || actor.kind & 0xf0 == 0x70 {
            return Ok(());
        }
        let right = if actor.state == State::Swimming || actor.kind == 0x1a {
            11
        } else {
            14
        };
        for (foot, joint) in instance.feet.iter_mut().zip([8, right]) {
            let joint = *rig.joints.get(joint).ok_or("Missing Tenchu 2 foot joint")?;
            let mut position = self.scene.world(joint)?.translation;
            match actor.terrain.material & 0xf00 {
                0x200 => position.y = self.water.bottom.wrapping_sub(400),
                0x300 => {}
                _ => position.y = position.y.wrapping_sub(500),
            }
            *foot = Foot {
                position,
                floor: terrain::level(&self.terrain, &mut self.cursor, position, Query::Surface)?,
            };
        }
        self.turn_head(index, assets)
    }

    fn turn_head(&mut self, index: usize, assets: &Assets) -> Result<()> {
        let actor = &self.actors[index].actor;
        let target = if index == self.player {
            let Some(target) = self.tracking.and_then(|id| self.actors.get(id)) else {
                return Ok(());
            };
            if target.actor.motion.loops == i16::MIN || target.actor.flags & 0x100000 == 0 {
                let actor = &mut self.actors[index].actor;
                actor.motion.pose.rotation[2][0] = 0;
                actor.motion.pose.rotation[2][1] = 0;
                self.publish_head(index, assets);
                self.tracking = None;
                return Ok(());
            }
            if actor.state == State::Attack && actor.motion.action > 0x91f {
                return Ok(());
            }
            if matches!(
                actor.state,
                State::Normal | State::Ready | State::Ledge | State::Attack
            ) {
                Some(target)
            } else {
                self.head_target(index)
            }
        } else {
            self.head_target(index)
        };
        let Some(target) = target else { return Ok(()) };
        let delta = target.actor.position.wrapping_sub(actor.position);
        let yaw = actor
            .rotation
            .y
            .wrapping_add(actor.motion.pose.rotation[0][1])
            .wrapping_add(actor.motion.pose.rotation[1][1]);
        let angle = direction(&assets.resources.math, delta.x, delta.z, yaw)?;
        if i32::from(angle).abs() < 1800 {
            let head = &mut self.actors[index].actor.motion.pose.rotation[2];
            head[1] = angle.clamp(-900, 900);
            let pitch = (delta.y / 2) as i16;
            if pitch != 0 {
                head[0] = pitch.clamp(-500, 100)
            }
            self.publish_head(index, assets);
        }
        Ok(())
    }

    fn head_target(&self, index: usize) -> Option<&runtime::Agent> {
        let actor = &self.actors[index].actor;
        if actor.flags & 0xf03 != 2 || !matches!(actor.state, State::Ready | State::Attack) {
            return None;
        }
        actor.target.and_then(|id| self.actors.get(id))
    }

    fn publish_head(&mut self, index: usize, assets: &Assets) {
        self.actors[index].actor.motion.pose.publish(
            &mut self.scene,
            &self.instances[index].rig.joints,
            &assets.resources.math,
            1 << 2,
            3,
        );
    }

    pub fn finish_tick(&mut self, assets: &Assets, budget: u8) -> Result<()> {
        self.scene.finish_tick();
        self.environment.flame_frame = (self.tick & 3) as u8;
        for scroll in &mut self.environment.texture_scrolls {
            scroll.step();
        }
        self.tick = self.tick.wrapping_add(1);
        self.update_activation(self.camera.eye, self.camera.look, budget, assets)?;
        self.collision.compute(|model| self.scene.world(model))?;
        Ok(())
    }

    pub fn view(&self) -> super::super::visibility::View {
        super::super::visibility::View {
            eye: self.camera.eye,
            look: self.camera.look,
            focus: if self.environment.background_only {
                self.camera.look
            } else {
                self.actors[self.controls.camera_actor].actor.position
            },
            matrix: self.camera.view,
            distance: self.view_distance,
        }
    }
}
