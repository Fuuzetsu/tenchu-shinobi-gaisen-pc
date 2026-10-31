mod traps;
use super::{Game, Statistics, events::Outcome};
use crate::{
    Result,
    actor::{
        Context, State as ActorState,
        patrol::{Patrol, Point as PatrolPoint},
    },
    ai::Halt,
    ids::*,
    math::Position,
    roster::Roster,
    scene::COLLIDE,
};
use tenchu_assets::trial::Objective;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    PlayerDied,
    ObjectiveLost,
    Spotted,
    TimeExpired,
}
impl Failure {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayerDied => "Mission failed",
            Self::ObjectiveLost => "The person you must protect has died",
            Self::Spotted => "You were spotted",
            Self::TimeExpired => "Time expired",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Playing,
    Success(u16),
    Failure(Failure, u16),
}
#[derive(Debug)]
pub struct State {
    pub phase: Phase,
    pub elapsed: i32,
    pub focus: Option<usize>,
    pub remaining: i16,
    pub music: Option<(usize, bool)>,
    pub notice: Option<(&'static str, u16)>,
    previous: [i16; 4],
    last_trigger: Option<(i16, i16)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            phase: Phase::Playing,
            elapsed: 0,
            focus: None,
            remaining: -1,
            music: None,
            notice: None,
            previous: [0; 4],
            last_trigger: None,
        }
    }
}

pub(super) fn construct(
    roster: &mut Roster,
    character: i16,
    ctx: &mut Context,
) -> Result<(usize, Statistics)> {
    let mission = ctx.mission.trial.as_ref().unwrap().mission.clone();
    for (index, placement) in mission.placements.iter().enumerate() {
        let [x, _, z] = placement.point.world();
        let y = crate::terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            Position::new(x, -10000, z),
            0,
        )?;
        let kind = if index == 0 {
            character
        } else {
            placement.kind
        };
        let yaw = (ctx.random.next15() & 4095) as i16;
        let id = roster.spawn(kind, Position::new(x, y, z), yaw, ctx)?;
        let actor = roster.get_mut(id);
        let walking = !placement.route.is_empty();
        let idle = if walking { 3 } else { 4 };
        let think = if index == 0 {
            0x1111
        } else {
            match kind & 0xf0 {
                0x80 => {
                    idle | 0x30
                        | if ctx.random.next15() & 1 != 0 {
                            0x4400
                        } else {
                            0x4500
                        }
                }
                0x90 => {
                    idle | if index & 1 != 0 { 0x30 } else { 0x40 }
                        | 0x600
                        | if kind == 0x90 { 0x4000 } else { 0x3000 }
                }
                _ if kind == 5 || kind == 6 => {
                    if walking {
                        3
                    } else {
                        0
                    }
                }
                _ if kind == 170 => 0,
                _ => {
                    idle | if index & 1 != 0 { 0x30 } else { 0x40 }
                        | if ctx.random.next15() & 1 != 0 {
                            0x400
                        } else {
                            0x500
                        }
                        | if ctx.random.next15() & 1 != 0 {
                            0x3000
                        } else {
                            0x4000
                        }
                }
            }
        };
        actor.setup_think(think)?;
        if kind == 170 {
            ctx.collision.remove(ctx.scene, actor.parts[0]);
        }
        if walking {
            let mut points: Vec<_> = placement
                .route
                .iter()
                .map(|p| {
                    let [x, _, z] = p.point.world();
                    PatrolPoint {
                        position: [x, z],
                        range: 500,
                        pad: if p.wait == 0 {
                            0
                        } else {
                            (p.facing + 1) as u16 | (((p.wait + 1) as u16) << 8)
                        },
                    }
                })
                .collect();
            if placement.route.first().unwrap().point != placement.route.last().unwrap().point {
                for i in (1..points.len() - 1).rev() {
                    points.push(points[i].clone());
                }
            }
            points.push(PatrolPoint {
                position: [0; 2],
                range: 0,
                pad: 0xffff,
            });
            actor.patrol = Some(Patrol {
                points,
                index: 0,
                count: 0,
            });
            actor.attribute |= ATTR_TRACE;
        }
    }
    let player = roster.order[0];
    let focus = if mission.objective == Objective::ProtectPrincess {
        roster.order[2]
    } else {
        player
    };
    let player_model = roster.get(player).root_model;
    let objective_model = roster.get(focus).root_model;
    let mut stats = Statistics::default();
    for &id in &roster.order {
        let actor = roster.actors[id].as_mut().unwrap();
        actor.target = Some(if actor.kind < 7 || actor.kind & 0xf0 == 0x90 {
            player_model
        } else {
            objective_model
        });
        if actor.max_health >= 0 && id != player && actor.kind != 0xa9 {
            match actor.kind & 0xf0 {
                0x90 => stats.citizens += 1,
                0x80 => {
                    stats.bosses += 1;
                    stats.enemies += 1;
                }
                _ => stats.enemies += 1,
            }
        }
    }
    let target = roster.get(roster.order[1]);
    if target.kind == 6 || target.kind & 0xf0 == 0x80 {
        let delta = target.position.wrapping_sub(roster.get(player).position);
        let actor = roster.get_mut(player);
        actor.rotation[1] = actor.rotation[1].wrapping_add(ctx.math.direction(
            delta.x,
            delta.z,
            actor.rotation[1],
        )?);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    }
    let actor = roster.get_mut(player);
    actor.inventory[0] = 255;
    actor.inventory[1] = 5;
    actor.inventory[crate::items::Kind::Caltrops.index()] = 3;
    Ok((player, stats))
}

impl Game {
    pub(super) fn activate_trial(&mut self, initial: bool) -> Result<()> {
        if self
            .trial
            .as_ref()
            .is_some_and(|t| t.phase != Phase::Playing)
        {
            return Ok(());
        }
        let mut visibility = i16::from(
            self.mission.trial.as_ref().unwrap().mission.objective == Objective::EliminateEnemies,
        );
        let mut active = 0;
        let mut offscreen = 0;
        let position = self.player().position;
        for &id in &self.roster.order {
            let actor = self.roster.actors[id].as_mut().unwrap();
            if actor.kind <= 6 {
                continue;
            }
            let dx = (actor.position.x - position.x) / 100;
            let dz = (actor.position.z - position.z) / 100;
            let distance = self.math.square_root(dx * dx + dz * dz);
            let mut range = if distance < 160 {
                -1
            } else if distance < 200 {
                1
            } else {
                0
            };
            if visibility != 0 {
                let matrix = self
                    .camera
                    .view
                    .compose(&self.scene.world(actor.root_model)?);
                let point = crate::projection::point(&matrix, [0; 3]);
                visibility = if point.z == 0 || point.xy[0].abs() > 320 {
                    -1
                } else {
                    1
                };
            }
            if self.roster.keep_all_active && actor.health >= 0 {
                actor.attribute &= !ATTR_SUSPEND;
                self.scene.models[actor.parts[0]].attribute |= COLLIDE;
                continue;
            }
            if actor.attribute & ATTR_SUSPEND != 0 {
                if actor.status != ActorState::Dead
                    && active < 3
                    && (range == 1 || range != 0 && visibility == -1 && offscreen <= 0)
                {
                    if visibility == -1 {
                        offscreen += 1;
                    }
                    self.scene.models[actor.parts[0]].attribute |= COLLIDE;
                    actor.attribute &= !ATTR_SUSPEND;
                    active += 1;
                }
            } else {
                if visibility == -1 && actor.health == 0 {
                    offscreen -= 1;
                    range = 0;
                }
                if range == 0
                    || (range == 1 || initial) && active >= 3
                    || actor.position.y >= 20000 && actor.status == ActorState::Dead
                {
                    self.scene.models[actor.parts[0]].attribute &= !COLLIDE;
                    actor.attribute |= ATTR_SUSPEND;
                } else {
                    active += 1;
                }
            }
        }
        Ok(())
    }

    pub(super) fn draw_trial_hud(&mut self) {
        let setup = self.mission.trial.as_ref().unwrap();
        let state = self.trial.as_mut().unwrap();
        if let Some((text, ticks)) = state.notice {
            state.notice = (ticks > 1).then_some((text, ticks.saturating_sub(1)));
        }
        if matches!(state.phase, Phase::Failure(Failure::Spotted, _)) {
            return;
        }
        let ticks = state.elapsed;
        if ticks < 107970 || (self.clock / 15) & 1 == 0 {
            for (i, digit) in [
                ticks / 18000 % 6,
                ticks / 1800 % 10,
                10,
                ticks / 300 % 6,
                ticks / 30 % 10,
            ]
            .into_iter()
            .enumerate()
            {
                if i == 0 && digit == 0 {
                    continue;
                }
                let mut image = setup.digits;
                image.size[1] /= 11;
                image.uv[1] = image.uv[1].wrapping_add((digit * i32::from(image.size[1])) as u8);
                let mut sprite = crate::hud::Sprite::new(image, [-24 + i as i16 * 12, -76]);
                sprite.depth = 2;
                self.hud.sprites.push(sprite);
            }
        }
        if setup.mission.objective == Objective::EliminateEnemies && state.phase == Phase::Playing {
            for i in 0..state.remaining {
                let image = setup.candles[(self.random.next15() % 3) as usize];
                let mut sprite = crate::hud::Sprite::new(image, [i * 10 - 60, 90]);
                sprite.depth = 1;
                self.hud.sprites.push(sprite);
            }
            if self.clock % 10 == 0 {
                state.remaining = self
                    .roster
                    .order
                    .iter()
                    .filter(|&&id| {
                        let a = self.roster.get(id);
                        a.kind > 6 && a.kind & 0xf0 != 0x90 && a.status != ActorState::Dead
                    })
                    .count() as i16;
            }
        }
    }

    pub(super) fn step_trial(&mut self) -> Result<()> {
        let mut state = self.trial.take().unwrap();
        state.music = None;
        let result = self.update_trial_result(&mut state);
        self.trial = Some(state);
        result
    }
    fn update_trial_result(&mut self, state: &mut State) -> Result<()> {
        let objective = self.mission.trial.as_ref().unwrap().mission.objective;
        match state.phase {
            Phase::Playing => {
                let counts = [
                    self.statistics.criticals,
                    self.statistics.murders,
                    self.statistics.friend_hits,
                    self.intelligence.found,
                ];
                for (i, count) in counts.into_iter().enumerate() {
                    if count > state.previous[i] {
                        self.clock += [-450, -150, 900, 300][i];
                        state.previous[i] = count;
                        state.notice = Some((
                            [
                                "Stealth kill −15 seconds",
                                "Kill −5 seconds",
                                "Civilian +30 seconds",
                                "Spotted +10 seconds",
                            ][i],
                            90,
                        ));
                        if i == 3 && self.mission.trial.as_ref().unwrap().mission.stealth {
                            if let Some(id) = self.roster.order.iter().copied().find(|&id| {
                                let actor = self.roster.get(id);
                                actor.attribute & ATTR_PHASE == 2
                                    && self.scene.models[actor.parts[0]].conflict.is_some()
                            }) {
                                self.trial_face(self.player_id, id)?;
                            }
                            state.phase = Phase::Failure(Failure::Spotted, 0);
                        }
                    }
                }
                self.clock = self.clock.max(0);
                state.elapsed = self.clock;
                if state.elapsed >= 107970 {
                    state.phase = Phase::Failure(Failure::TimeExpired, 0);
                }
                if self.player().health == 0 || self.player().status == ActorState::Dead {
                    state.phase = Phase::Failure(Failure::PlayerDied, 0);
                }
                if state.phase == Phase::Playing && self.clock % 15 == 0 {
                    let p = self.player();
                    let target = self.roster.get(self.roster.order[1]);
                    let close = |a: &crate::actor::Actor, b: &crate::actor::Actor, range: i32| {
                        (a.position.x - b.position.x).abs() < range
                            && (a.position.z - b.position.z).abs() < range
                            && a.position.y == b.position.y
                    };
                    let grounded = p.map.height == 0;
                    let ready = grounded && p.status != ActorState::Damaged;
                    let success = match objective {
                        Objective::ReachLord => {
                            if target.status == ActorState::Dead {
                                state.phase = Phase::Failure(Failure::ObjectiveLost, 0);
                                state.focus = Some(self.roster.order[1]);
                            }
                            ready && close(p, target, 2000)
                        }
                        Objective::Assassinate => grounded && target.status == ActorState::Dead,
                        Objective::EliminateEnemies => grounded && state.remaining == 0,
                        Objective::ProtectPrincess => {
                            let princess = self.roster.get(self.roster.order[2]);
                            if target.status == ActorState::Dead {
                                state.phase = Phase::Failure(Failure::ObjectiveLost, 0);
                                state.focus = Some(self.roster.order[1]);
                            }
                            if princess.status == ActorState::Dead || princess.health == 0 {
                                state.phase = Phase::Failure(Failure::ObjectiveLost, 0);
                                state.focus = Some(self.roster.order[2]);
                            }
                            ready && close(princess, target, 2000) && princess.position.y <= 0
                        }
                        Objective::FindHerb => ready && close(p, target, 2500),
                    };
                    if success && state.phase == Phase::Playing {
                        state.phase = Phase::Success(0);
                        state.focus = match objective {
                            Objective::ReachLord => Some(self.roster.order[1]),
                            Objective::ProtectPrincess => Some(self.roster.order[2]),
                            _ => None,
                        };
                        for actor in self.roster.actors.iter_mut().flatten() {
                            if actor.health > 0 {
                                actor.health = -1;
                            }
                        }
                    }
                }
            }
            Phase::Success(ticks) => {
                if ticks == 0 {
                    self.begin_trial_result(state, true)?;
                }
                state.phase = Phase::Success(ticks + 1);
                let target = self.roster.get(self.roster.order[1]);
                if target.motion.action == MOT_ACTION_LOOP && target.motion.loops != 0 {
                    self.trial_motion(self.roster.order[1], MOT_NORMAL, false)?;
                }
                if ticks
                    >= if objective == Objective::ProtectPrincess {
                        399
                    } else {
                        200
                    }
                {
                    self.events.outcome = Outcome::Complete;
                    self.clock = state.elapsed;
                }
            }
            Phase::Failure(reason, ticks) => {
                if ticks == 0 {
                    self.begin_trial_result(state, false)?;
                }
                state.phase = Phase::Failure(reason, ticks.saturating_add(1));
                if reason == Failure::Spotted && self.random.next15() % 30 == 0 {
                    let mut position = self.player().position;
                    position.x += (self.random.next15() % 11 - 5) * 1000;
                    position.z += (self.random.next15() % 11 - 5) * 1000;
                    self.effects.sounds.push((position, 0x20));
                    if self.player().motion.action == MOT_ENGAGE_STANCE {
                        let motion = [
                            MOT_CHASE_DASH_BACK,
                            MOT_CHASE_DASH_RIGHT,
                            MOT_CHASE_DASH_LEFT,
                            MOT_ENGAGE_STANCE,
                        ][(self.random.next15() % 4) as usize];
                        self.trial_motion(self.player_id, motion, false)?;
                    }
                }
                let player = self.player();
                let finished = if player.status == ActorState::Dead {
                    player.motion.count == 0
                        && player.motion.loops != 0
                        && (ticks >= 198 || player.map.attribute & 0x200 != 0)
                } else {
                    ticks >= 198
                };
                if finished {
                    self.events.outcome = Outcome::GameOver;
                    self.clock = state.elapsed;
                }
            }
        }
        Ok(())
    }

    fn begin_trial_result(&mut self, state: &mut State, success: bool) -> Result<()> {
        self.intelligence.action_halt = if success {
            Halt::Active
        } else {
            Halt::StageEnd
        };
        self.roster.get_mut(self.player_id).setup_think(0)?;
        if success {
            let target = self.roster.order[1];
            if self.roster.get(target).kind == 6 {
                self.roster.get_mut(target).setup_think(0)?;
                self.trial_motion(self.player_id, MOT_ENGAGE_STANCE, true)?;
                self.trial_motion(target, MOT_ACTION_LOOP, true)?;
                self.trial_face(self.player_id, target)?;
                self.trial_face(target, self.player_id)?;
            }
            if self
                .roster
                .order
                .get(2)
                .is_some_and(|&id| self.roster.get(id).kind == 5)
            {
                let princess = self.roster.order[2];
                self.roster.get_mut(princess).setup_think(0)?;
                self.trial_motion(princess, 0x108, true)?;
                self.trial_face(target, princess)?;
                self.trial_face(princess, target)?;
            }
            if self.roster.get(target).kind == 170 {
                self.trial_motion(self.player_id, MOT_ENGAGE_STANCE, true)?;
                self.trial_face(self.player_id, target)?;
            }
            let objective = self.mission.trial.as_ref().unwrap().mission.objective;
            let ayame = self.player().kind == 1;
            match objective {
                Objective::ReachLord => state.music = Some((if ayame { 102 } else { 101 }, false)),
                Objective::ProtectPrincess => {
                    state.music = Some((if ayame { 141 } else { 140 }, false))
                }
                Objective::Assassinate | Objective::EliminateEnemies => self
                    .music
                    .push(Some(if self.random.next15() & 1 != 0 { 2 } else { 4 })),
                Objective::FindHerb => self.music.push(Some(if ayame { 38 } else { 37 })),
            }
        } else if let Phase::Failure(reason, _) = state.phase {
            match reason {
                Failure::Spotted => {
                    self.intelligence.voices.push((self.player_id, 6));
                    self.intelligence.action_halt = Halt::Running;
                    self.music.push(Some(10));
                }
                Failure::TimeExpired => {
                    self.intelligence.action_halt = Halt::Running;
                    self.music.push(Some(8));
                }
                _ => {}
            }
        }
        if self.camera.mode != crate::camera::Mode::Fall || success {
            self.camera.set_mode(
                crate::camera::Mode::Critical,
                self.roster.get(state.focus.unwrap_or(self.player_id)),
                &mut crate::camera::Environment {
                    terrain: &self.mission.terrain,
                    cursor: &mut self.cursor,
                    math: &self.math,
                    body: &self.body,
                },
                &mut self.random,
            )?;
        }
        Ok(())
    }

    fn trial_face(&mut self, id: usize, target: usize) -> Result<()> {
        let delta = self
            .roster
            .get(target)
            .position
            .wrapping_sub(self.roster.get(id).position);
        let actor = self.roster.get_mut(id);
        actor.rotation[1] = actor.rotation[1].wrapping_add(self.math.direction(
            delta.x,
            delta.z,
            actor.rotation[1],
        )?);
        Ok(())
    }
    fn trial_motion(&mut self, id: usize, motion: i16, advance: bool) -> Result<()> {
        let actor = self.roster.get_mut(id);
        actor.enter(
            &Context {
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
            },
            motion,
            true,
        );
        if advance {
            actor
                .motion
                .play(&self.mission.clips, &self.math.tables.hermite, true)?;
        }
        Ok(())
    }
}
