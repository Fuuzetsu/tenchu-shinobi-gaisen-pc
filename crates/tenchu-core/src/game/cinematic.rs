use crate::actor::State as ActorState;
use crate::math::Position;
mod camera;
#[cfg(test)]
mod tests;
mod text;

use super::Game;
use crate::{Result, actor::Context, ids::*, scene::COLLIDE, terrain};
use tenchu_assets::script::Command;

#[derive(Clone, Copy, Debug)]
struct Controlled {
    actor: usize,
    loops: i16,
    next: i16,
}

#[derive(Debug, Default)]
pub struct Cinematic {
    pub active: bool,
    pub frame: bool,
    pub sequence: i16,
    pub elapsed: i16,
    pub(super) input: u16,
    pub tail: u8,
    music_cue: i16,
    pub caption: Vec<u8>,
    pub displayed_caption: Vec<u8>,
    caption_image: Option<tenchu_assets::texture::Image>,
    pub tanka: [bool; 6],
    pub tanka_shade: [u8; 6],
    pub sprites: Vec<crate::hud::Sprite>,
    cursor: usize,
    actors: [Option<Controlled>; 5],
    target: usize,
    pan: i16,
    speed: i16,
}

impl Game {
    pub(super) fn begin_cinematic(&mut self, sequence: i16) -> Result<bool> {
        let Some(cursor) = self.mission.scripts.cutscenes.sequence(sequence) else {
            return Ok(false);
        };
        let music = self.mission.scripts.cutscenes.commands[cursor].parameter;
        self.cinematic.sequence = sequence;
        self.cinematic.input = self.input;
        self.cinematic.music_cue = music;
        self.cinematic.cursor = cursor + 1;
        self.cinematic.actors = [None; 5];
        self.cinematic.target = self.player_id;
        self.cinematic.caption.clear();
        let mut ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        for &id in &self.roster.order {
            let actor = self.roster.actors[id].as_mut().unwrap();
            actor.scripted = false;
            if actor.status != ActorState::Dead && actor.attribute & ATTR_SUSPEND == 0 {
                actor.clear_attack(&mut ctx, 3)?;
                let motion = actor.return_normal(&mut ctx, self.player_id)?;
                actor.enter(&ctx, motion, true);
                actor.pad.held = 0;
            }
        }
        if !self.cinematic_commands()? {
            return Ok(false);
        }
        self.cinematic.active = true;
        self.cinematic.elapsed = 0;
        self.intelligence.cinematic = true;
        if self.intelligence.action_halt != crate::ai::Halt::StageEnd {
            self.intelligence.action_halt = crate::ai::Halt::Active;
        }
        self.roster.get_mut(self.player_id).target = None;
        for control in self.cinematic.actors.iter().flatten() {
            self.roster.get_mut(control.actor).scripted = true;
        }
        if music > 0 {
            self.music.push(Some(music));
        }
        Ok(true)
    }

    fn cinematic_actor(&mut self, command: Command) -> Result<bool> {
        let Some(id) = self.roster.find(command.id) else {
            return Ok(false);
        };
        let player_y = self.player().position.y;
        let actor = self.roster.get_mut(id);
        let mut ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        if command.operation == 2 {
            actor.attribute &= !ATTR_SUSPEND;
            actor.motion.mask = 0x7fff;
            if self.cinematic.actors.iter().all(Option::is_some) {
                actor.enter(&ctx, MOT_ENGAGE_STANCE, true);
            }
            actor.velocity = [0; 3];
            ctx.scene.models[actor.parts[0]].attribute |= COLLIDE;
            for &part in &actor.parts {
                ctx.scene.models[part].attribute &= !1;
            }
            if id != self.player_id && actor.health == -1 {
                actor.attribute |= ATTR_CUSTOMAI;
                actor.health = actor.max_health;
            }
            if command.parameter != -1 {
                let [x, y, z] = command.position.map(|v| i32::from(v) * 1000);
                let floor = terrain::level(
                    &ctx.mission.terrain,
                    ctx.cursor,
                    Position::new(x, y - 1000, z),
                    0,
                )?;
                actor.position = Position::new(
                    x,
                    if floor == terrain::NONE {
                        y
                    } else {
                        floor.min(y)
                    },
                    z,
                );
                actor.home = [x, z];
                actor.rotation[1] = command.parameter;
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
                let owner_y = if id == self.player_id {
                    actor.position.y
                } else {
                    player_y
                };
                if actor.position.y.wrapping_sub(owner_y).wrapping_abs() > 20000 {
                    actor.attribute |= ATTR_SUSPEND;
                }
            }
        } else if command.position[0] == -1 {
            actor.health = -1;
            actor.attribute = (actor.attribute | ATTR_SUSPEND | 2) & !ATTR_CUSTOMAI;
            actor.motion.action = -1;
            actor.enter(&ctx, MOT_NORMAL, true);
            actor.play_pose(&mut ctx)?;
            actor.motion.count = actor.motion.count.wrapping_sub(1);
        } else {
            let [motion, loops, next] = command.position;
            let status = (motion as u16 >> 8) as i16;
            if actor.status == ActorState::Dead && !(16..=17).contains(&status) {
                return Ok(false);
            }
            if actor.health > 0 && status == 17 {
                actor.health = 0;
                self.combat.request_life(actor);
            }
            let Some(slot) = self
                .cinematic
                .actors
                .iter()
                .position(|c| c.is_some_and(|c| c.actor == id))
                .or_else(|| self.cinematic.actors.iter().position(Option::is_none))
            else {
                return Ok(false);
            };
            actor.motion.action = -1;
            actor.enter(&ctx, motion, true);
            actor.play_pose(&mut ctx)?;
            actor.motion.count = actor.motion.count.wrapping_sub(1);
            self.cinematic.actors[slot] = Some(Controlled {
                actor: id,
                loops: if loops < 1 { i16::MAX } else { loops },
                next: if next == 0 { MOT_ENGAGE_STANCE } else { next },
            });
            actor.scripted = self.cinematic.active;
            if actor.kind == 0xa7 && motion == MOT_DEAD {
                ctx.effects.sounds.push((Position::ZERO, 0x41));
            }
        }
        Ok(true)
    }

    fn cinematic_commands(&mut self) -> Result<bool> {
        loop {
            let command = *self
                .mission
                .scripts
                .cutscenes
                .commands
                .get(self.cinematic.cursor)
                .ok_or("Cinematic reached the table end without a wait command")?;
            match command.operation {
                0 if command.parameter == -1 => self.music.push(None),
                1 => return Ok(command.id != 0),
                2 | 3 => {
                    if !self.cinematic_actor(command)? {
                        return Ok(false);
                    }
                }
                4 | 5 => {
                    if !self.cinematic_camera_command(command)? {
                        return Ok(false);
                    }
                }
                6 => {
                    self.cinematic.pan = command.id;
                    self.cinematic.speed = if command.parameter == 0 {
                        20
                    } else {
                        command.parameter
                    };
                }
                7 => match command.id {
                    1 => self.effects.blood(
                        command.position.map(|v| i32::from(v) * 10).into(),
                        command.parameter,
                        30,
                        &mut self.random,
                        &mut crate::effects::Environment {
                            terrain: &self.mission.terrain,
                            cursor: &mut self.cursor,
                            clock: self.clock,
                            owner_position: self.roster.get(self.player_id).position,
                            reference: self.camera.reference,
                        },
                    )?,
                    3 => self.effects.fade(
                        command.position.map(|v| v as u8),
                        i32::from(command.parameter),
                        self.clock,
                    ),
                    _ => (),
                },
                8 => {
                    self.cinematic.caption = if command.id == -1 {
                        Vec::new()
                    } else {
                        self.mission.scripts.cutscenes.text(command.id)?.to_vec()
                    };
                    if self.mission.stage.id == 10
                        && self.player().kind == 0
                        && let Some(&digit) = self
                            .cinematic
                            .caption
                            .first()
                            .filter(|b| b.is_ascii_digit())
                    {
                        if digit == b'0' {
                            self.cinematic.tanka.fill(false);
                        } else if let Some(visible) =
                            self.cinematic.tanka.get_mut(usize::from(digit - b'1'))
                        {
                            *visible = true;
                        }
                        self.cinematic.caption.clear();
                    }
                }
                _ => (),
            }
            self.cinematic.cursor += 1;
        }
    }

    pub(super) fn advance_cinematic(&mut self) -> Result<()> {
        if !self.cinematic.active {
            return Ok(());
        }
        let ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        for slot in &mut self.cinematic.actors {
            let Some(control) = *slot else { continue };
            let actor = self.roster.get_mut(control.actor);
            if control.loops <= actor.motion.loops {
                if control.next == -1 {
                    actor.motion.loops = -1;
                    actor.velocity[0] = 0;
                    actor.velocity[2] = 0;
                } else if actor.status != ActorState::Dead {
                    actor.enter(&ctx, control.next, true);
                    actor.scripted = false;
                    *slot = None;
                }
            }
        }
        self.cinematic.elapsed = self.cinematic.elapsed.wrapping_add(1);
        let wait = ctx.mission.scripts.cutscenes.commands[self.cinematic.cursor].id;
        if self.cinematic.elapsed >= wait {
            self.cinematic.cursor += 1;
            if !self.cinematic_commands()? {
                self.finish_cinematic()?;
            }
        }
        Ok(())
    }

    fn finish_cinematic(&mut self) -> Result<()> {
        self.intelligence.cinematic = false;
        if self.intelligence.action_halt != crate::ai::Halt::StageEnd {
            self.intelligence.action_halt = crate::ai::Halt::Running;
        }
        self.camera
            .normal(self.roster.get(self.player_id), &self.math)?;
        let ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        for control in self.cinematic.actors.iter().flatten() {
            let actor = self.roster.get_mut(control.actor);
            actor.scripted = false;
            if actor.status != ActorState::Dead {
                let motion = if actor.attribute & ATTR_WEAPON_DRAWN != 0 {
                    MOT_ENGAGE_STANCE
                } else if actor.kind & 0xf0 == 0x80 {
                    MOT_STATE_DRAW
                } else {
                    MOT_NORMAL
                };
                actor.enter(&ctx, motion, true);
            }
        }
        // VSync(60) leaves an intentional audio tail without updating the world.
        self.cinematic.tail = if self.cinematic.music_cue > 0 { 30 } else { 0 };
        if self.cinematic.tail == 0 {
            self.cinematic.active = false;
            self.music.push(None);
        }
        Ok(())
    }
}
