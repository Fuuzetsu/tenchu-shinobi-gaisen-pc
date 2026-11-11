use super::{
    CorpseContext, CorpseSearch,
    behavior::{self, Behavior, Events},
    combat::{self, Context},
    entry,
    navigation::{Navigation, Surroundings},
    perception,
    sight::{self, Body, Ranges, Sight, Visibility},
    status::{AlertCause, Ki},
};
use crate::{
    Result,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State},
        collision::{self, Cover},
        control::{TargetQuery, find_target},
        terrain::Cursor,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{collision::Collision, combat::Rules, layout::Waypoint};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub actor: Actor,
    pub behavior: Behavior,
    pub sight: Sight,
    pub surroundings: Surroundings,
    pub slot: i16,
    pub sight_range: i16,
}

impl Agent {
    pub fn new(actor: Actor, slot: i16, sight_range: i16, route: &[Waypoint]) -> Self {
        Self {
            behavior: Behavior::new(actor.position, route),
            sight: Sight {
                destination: actor.position,
                ..Default::default()
            },
            actor,
            surroundings: Surroundings::default(),
            slot,
            sight_range,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct System {
    pub global: entry::Global,
    pub ranges: Ranges,
    pub cover: Cover,
    pub corpses: CorpseSearch,
    pub facing: i32,
    pub visibility: Visibility,
    pub detections: u8,
    pub last_spotter: Option<usize>,
    #[serde(default = "original_sight")]
    pub sight_percent: u16,
    #[serde(default)]
    pub ignore_player: bool,
    pub quick_shuriken_dodge: bool,
    pub strict_corpse_sight: bool,
}

fn original_sight() -> u16 {
    100
}

impl System {
    pub fn new(sight_distance: i32) -> Self {
        Self {
            global: entry::Global {
                perception: perception::Global {
                    nearest_distance: 0,
                    ..Default::default()
                },
                radar: Ki::Suspicious,
                ..Default::default()
            },
            ranges: Ranges::new(sight_distance),
            cover: Cover::default(),
            corpses: CorpseSearch::default(),
            facing: 0,
            visibility: Visibility::Unchanged,
            detections: 0,
            last_spotter: None,
            sight_percent: 100,
            ignore_player: false,
            quick_shuriken_dodge: false,
            strict_corpse_sight: false,
        }
    }

    pub fn update(
        &mut self,
        index: usize,
        agents: &mut [Agent],
        frame: &mut Frame<'_, impl Events>,
    ) -> Result<u16> {
        let ignore_player = self.ignore_player && frame.script == 0;
        if ignore_player && index == frame.player {
            self.global.perception.alert = Default::default();
        }
        let player = &agents
            .get(frame.player)
            .ok_or("Missing Tenchu 2 player")?
            .actor;
        let player_body = Body::from(player);
        let player_health = player.health;
        let player_sight_percent = player.sight_percent(self.sight_percent);
        let quick_shuriken_yaw = (self.quick_shuriken_dodge
            && player.health > 0
            && player.motion.action == 0x802
            && player.inventory.active.index() == 4)
            .then_some(frame.aim_yaw.unwrap_or(player.rotation.y));
        if index >= agents.len() {
            return Err("Missing Tenchu 2 AI actor".into());
        }
        let (before, tail) = agents.split_at_mut(index);
        let (agent, after) = tail.split_first_mut().unwrap();
        let mut others = Others { before, after };
        let target = agent
            .actor
            .target
            .and_then(|target| others.get(target, &agent.actor).map(combat::Target::from));
        let old_nearest = self.global.perception.nearest_distance;
        let mut c = Context {
            steering: agent.surroundings.steering(
                &agent.actor,
                agent.actor.flags,
                agent.sight.angle,
                agent.behavior.forced.is_some(),
            ),
            actor: &mut agent.actor,
            target,
            sight: &mut agent.sight,
            visibility: &mut self.visibility,
            observed_flags: 0,
            player_held: self.global.player_held,
            facing: self.facing,
            difficulty: usize::from(frame.difficulty),
            resources: frame.resources,
            rules: frame.rules,
            random: frame.random,
        };
        let scene = entry::Scene {
            actor: index,
            player: frame.player,
            player_health,
            player_position: player_body.position,
            tick: frame.tick,
            slot: agent.slot,
            map: frame.map,
            has_suspicious_program: agent.behavior.has_suspicious_program,
        };
        let mut callbacks = Reactions {
            others: &mut others,
            behavior: &mut agent.behavior,
            surroundings: &mut agent.surroundings,
            ranges: &mut self.ranges,
            cover: &mut self.cover,
            corpses: &mut self.corpses,
            cursor: frame.cursor,
            terrain: frame.terrain,
            collision: frame.collision,
            player_body,
            scene: &scene,
            sight_range: agent.sight_range,
            sight_percent: self.sight_percent,
            player_sight_percent,
            ignore_player,
            animated_corpse: frame.animated_corpse,
            script: frame.script,
            controllers: frame.controllers,
            events: frame.events,
        };
        let result = entry::update(&mut c, &scene, &mut self.global, &mut callbacks)?;
        if let entry::Outcome::Complete(held) = result {
            self.facing = c.facing;
            return Ok(held);
        }
        let entry::Outcome::Perception { player_distance } = result else {
            unreachable!()
        };
        callbacks.sample(&mut c, 1500)?;
        let scene = perception::Scene {
            map: frame.map,
            sequence: frame.sequence,
            script: frame.script,
            player_distance,
            notice_distance: sight::scale_range(
                callbacks.ranges.standing.notice,
                player_sight_percent,
            ),
            player_action: player_body.action,
            player: frame.player,
            recovering_confusion: callbacks.behavior.recovering_confusion,
            surroundings: callbacks.surroundings,
        };
        let outcome = if callbacks.ignores(&c) {
            perception::Outcome {
                decision: super::Decision {
                    held: callbacks.unnoticed(&mut c, &mut self.global, player_distance)?,
                    ..Default::default()
                },
                spotted: false,
            }
        } else {
            callbacks.behavior.perceive(
                &mut c,
                &scene,
                &mut self.global.perception,
                &mut behavior::Frame {
                    terrain: frame.terrain,
                    cursor: callbacks.cursor,
                    tick: frame.tick,
                    script: frame.script,
                    quick_shuriken_yaw,
                    controllers: frame.controllers,
                    rotation_phase: &mut self.global.rotation_phase,
                    events: callbacks.events,
                    observation: super::bait::Observation {
                        player: frame.player,
                        body: player_body,
                        slot: agent.slot,
                        range: agent.sight_range,
                        range_percent: player_sight_percent,
                        ignore_player,
                        ranges: callbacks.ranges,
                    },
                },
            )?
        };
        let held = Navigation {
            actor: c.actor,
            target: c
                .actor
                .target
                .and_then(|id| callbacks.others.get(id, c.actor)),
            surroundings: callbacks.surroundings,
            facing: c.facing,
            alert: self.global.perception.alert.remaining,
            player_alive: player_health > 0,
        }
        .finish(
            outcome.decision.held,
            &mut callbacks.behavior.forced,
            c.random,
        );
        if c.actor.health <= 0 && c.actor.state == State::Dead {
            self.global.perception.nearest_distance = old_nearest;
        }
        self.global.radar = Ki::from_distance(self.global.perception.nearest_distance);
        c.actor.flags = c.observed_flags;
        c.actor.pad.update(held);
        self.facing = c.facing;
        if outcome.spotted {
            self.detections = self.detections.wrapping_add(1);
            self.last_spotter = Some(index);
        }
        Ok(held)
    }
}

pub struct Frame<'a, E> {
    pub player: usize,
    pub map: usize,
    pub sequence: u16,
    pub script: u16,
    pub tick: u32,
    pub difficulty: u8,
    pub controllers: [u16; 2],
    pub resources: &'a Resources,
    pub rules: &'a Rules,
    pub terrain: &'a Collision,
    pub cursor: &'a mut Cursor,
    pub collision: &'a collision::Collision,
    pub animated_corpse: Option<crate::math::Position>,
    pub aim_yaw: Option<i16>,
    pub random: &'a mut Random,
    pub events: &'a mut E,
}

pub(crate) struct Others<'a> {
    pub before: &'a mut [Agent],
    pub after: &'a mut [Agent],
}

impl Others<'_> {
    pub fn len(&self) -> usize {
        self.before.len() + 1 + self.after.len()
    }

    pub fn get<'a>(&'a self, index: usize, current: &'a Actor) -> Option<&'a Actor> {
        if index == self.before.len() {
            Some(current)
        } else {
            let agent = if index < self.before.len() {
                self.before.get(index)
            } else {
                self.after.get(index - self.before.len() - 1)
            };
            agent.map(|agent| &agent.actor)
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Result<&mut Agent> {
        let agent = if index < self.before.len() {
            self.before.get_mut(index)
        } else if index > self.before.len() {
            self.after.get_mut(index - self.before.len() - 1)
        } else {
            None
        };
        agent.ok_or_else(|| "Missing other Tenchu 2 actor".into())
    }
}

struct Reactions<'a, 'b, E> {
    others: &'a mut Others<'b>,
    behavior: &'a mut Behavior,
    surroundings: &'a mut Surroundings,
    ranges: &'a mut Ranges,
    cover: &'a mut Cover,
    corpses: &'a mut CorpseSearch,
    cursor: &'a mut Cursor,
    terrain: &'a Collision,
    collision: &'a collision::Collision,
    player_body: Body,
    scene: &'a entry::Scene,
    sight_range: i16,
    sight_percent: u16,
    player_sight_percent: u16,
    ignore_player: bool,
    animated_corpse: Option<crate::math::Position>,
    script: u16,
    controllers: [u16; 2],
    events: &'a mut E,
}

impl<E> Reactions<'_, '_, E> {
    fn ignores(&self, c: &Context<'_>) -> bool {
        self.ignore_player
            && self.scene.actor != self.scene.player
            && c.actor
                .target
                .is_none_or(|target| target == self.scene.player)
            && !matches!(
                self.behavior.programs.idle,
                super::Program::PlayerOne | super::Program::PlayerTwo
            )
    }

    fn sample(&mut self, c: &mut Context<'_>, distance: i16) -> Result<()> {
        self.surroundings.sample(
            c.actor,
            self.scene.tick,
            distance,
            self.terrain,
            self.cursor,
            &c.resources.math,
        )?;
        c.steering = self.surroundings.steering(
            c.actor,
            c.observed_flags,
            c.sight.angle,
            self.behavior.forced.is_some(),
        );
        Ok(())
    }
}

impl<E: Events> Reactions<'_, '_, E> {
    fn unnoticed(
        &mut self,
        c: &mut Context<'_>,
        global: &mut entry::Global,
        distance: i32,
    ) -> Result<u16> {
        use entry::Reactions;
        if c.observed_flags & 0x83 != 0 {
            self.behavior.reset(true);
            self.behavior.forced = None;
        }
        c.observed_flags &= !(0x83 | crate::tenchu2::physics::TOUCHING_PLAYER);
        c.actor.flags = c.observed_flags;
        c.steering.alerted = false;
        c.steering.forced = self.behavior.forced.is_some();
        *c.visibility = Visibility::Hidden;
        self.behavior.observe(Visibility::Hidden);
        global.perception.alert = Default::default();
        global.perception.nearest_distance =
            global.perception.nearest_distance.min(distance.max(1));
        // Already committed attacks finish normally; patrols cannot request a new attack/item.
        self.idle(c, global)
            .map(|held| held & !(crate::tenchu2::input::ATTACK | crate::tenchu2::input::ITEM))
    }
}

impl<E: Events> entry::Reactions for Reactions<'_, '_, E> {
    fn nearest_target(&mut self, c: &mut Context<'_>, range: i32) -> Result<Option<entry::Target>> {
        Ok(find_target(
            self.others.len(),
            |index| self.others.get(index, c.actor),
            self.scene.actor,
            Some(self.scene.player),
            TargetQuery::Nearby {
                distance: range.wrapping_abs(),
                enemies_only: range < 0,
            },
            &c.resources.math,
        )
        .and_then(|index| {
            self.others.get(index, c.actor).map(|actor| entry::Target {
                actor: index,
                kind: actor.kind,
            })
        }))
    }

    fn action(&mut self, c: &mut Context<'_>, action: i16) -> Result<()> {
        self.events.action(c.actor, action, c.resources).map(|_| ())
    }

    fn idle(&mut self, c: &mut Context<'_>, global: &mut entry::Global) -> Result<u16> {
        self.behavior
            .idle(
                c,
                &mut global.perception,
                &mut behavior::Frame {
                    terrain: self.terrain,
                    cursor: self.cursor,
                    tick: self.scene.tick,
                    script: self.script,
                    quick_shuriken_yaw: None,
                    controllers: self.controllers,
                    rotation_phase: &mut global.rotation_phase,
                    events: self.events,
                    observation: super::bait::Observation {
                        player: self.scene.player,
                        body: self.player_body,
                        slot: self.scene.slot,
                        range: self.sight_range,
                        range_percent: self.player_sight_percent,
                        ignore_player: self.ignore_player,
                        ranges: self.ranges,
                    },
                },
            )
            .map(|decision| decision.held)
    }

    fn victory(&mut self, c: &mut Context<'_>) -> Result<u16> {
        self.behavior.victory(c, self.events)?;
        Ok(0)
    }

    fn corpses(&mut self, c: &mut Context<'_>, global: &mut entry::Global) -> Result<()> {
        if self.ignore_player {
            return Ok(());
        }
        let observer = self.corpses.observe(
            self.scene.actor,
            self.others.len(),
            |index| self.others.get(index, c.actor),
            &mut CorpseContext {
                player: self.scene.player,
                difficulty: c.difficulty as u8,
                distance: sight::scale_range(self.ranges.standing.notice, self.sight_percent),
                animated_position: self.animated_corpse,
                terrain: self.terrain,
                cursor: self.cursor,
                math: &c.resources.math,
            },
        )?;
        if let Some(index) = observer {
            global
                .perception
                .alert
                .raise(AlertCause::Combat, c.difficulty as u8, c.rules)?;
            let observer = self.others.get_mut(index)?;
            self.events.voice(&observer.actor, 7)?;
            c.actor.flags |= 0x80;
            observer.actor.flags = observer.actor.flags & !3 | 0x81;
            observer.behavior.remember(c.actor.position);
            observer.behavior.reset(false);
        }
        Ok(())
    }

    fn confusion(&mut self, c: &mut Context<'_>) -> Result<u16> {
        self.behavior.confusion(c).map(|decision| decision.held)
    }

    fn sight(&mut self, c: &mut Context<'_>) -> Result<()> {
        let target = c.actor.target.and_then(|id| self.others.get(id, c.actor));
        let range_percent =
            target.map_or(self.sight_percent, |a| a.sight_percent(self.sight_percent));
        c.target = target.map(combat::Target::from);
        let target = target.map(Body::from);
        *c.visibility = c.sight.check(
            &sight::Observer {
                body: Body::from(&*c.actor),
                kind: c.actor.kind,
                slot: self.scene.slot,
                weapon: c
                    .resources
                    .registry
                    .actor(c.actor.kind)
                    .ok_or("Missing Tenchu 2 sight actor")?
                    .weapon,
                range: self.sight_range,
            },
            target.as_ref(),
            &mut sight::Context {
                tick: self.scene.tick,
                player_position: self.player_body.position,
                player_yaw: self.player_body.yaw,
                script_mode: self.script,
                range_percent,
                hidden: self.ignore_player && c.actor.target == Some(self.scene.player),
                terrain: self.terrain,
                math: &c.resources.math,
                near_object: &mut || {
                    self.cover
                        .check(
                            self.collision,
                            self.player_body.position,
                            self.scene.tick as i32,
                        )
                        .is_some()
                },
            },
            self.ranges,
            self.cursor,
        )?;
        if target.is_some() {
            c.facing = i32::from(c.sight.angle).abs();
        }
        c.steering.target_angle = c.sight.angle;
        Ok(())
    }

    fn sample_player_terrain(&mut self, c: &mut Context<'_>) -> Result<()> {
        self.sample(c, 1000)
    }

    fn reset(&mut self, forget: bool) {
        self.behavior.reset(forget);
    }

    fn clear_forced(&mut self) {
        self.behavior.forced = None;
    }

    fn recover_confusion(&mut self) {
        self.behavior.recovering_confusion = true;
    }
}
