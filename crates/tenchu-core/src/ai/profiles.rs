use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Combat {
    Original,
    Coward,
    Reckless,
    Cautious,
    Flanker,
    Watchman,
    #[default]
    Mixed,
}

impl Combat {
    pub const ALL: [Self; 7] = [
        Self::Original,
        Self::Coward,
        Self::Reckless,
        Self::Cautious,
        Self::Flanker,
        Self::Watchman,
        Self::Mixed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Coward => "Coward",
            Self::Reckless => "Reckless",
            Self::Cautious => "Cautious",
            Self::Flanker => "Flanker",
            Self::Watchman => "Watchman",
            Self::Mixed => "Seeded mix",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Idle {
    #[default]
    Original,
    Patrol,
    Lookout,
    TracePlayer,
}

impl Idle {
    pub const ALL: [Self; 4] = [
        Self::Original,
        Self::Patrol,
        Self::Lookout,
        Self::TracePlayer,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Patrol => "Follow patrol",
            Self::Lookout => "Stationary lookout",
            Self::TracePlayer => "Player-direction trace",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub combat: Combat,
    pub idle: Idle,
    pub quick_turns: bool,
    pub progression: bool,
    pub seed: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            combat: Combat::Mixed,
            idle: Idle::Original,
            quick_turns: true,
            progression: false,
            seed: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Overrides {
    pub combat: Option<Combat>,
    pub idle: Option<Idle>,
}

pub fn eligible(
    actor: &Actor,
    mission: &tenchu_assets::stage::MissionAssets,
    player: usize,
) -> bool {
    let kind = actor.script_kind.unwrap_or(actor.kind);
    actor.id != player
        && (7..0x80).contains(&kind)
        && (7..0x80).contains(&actor.kind)
        && actor.health > 0
        && !actor.scripted
        && actor.attribute & ATTR_CUSTOMAI != 0
        && !matches!(
            actor.think.combat,
            programs::Combat::None | programs::Combat::Disabled | programs::Combat::Player
        )
        && !mission.story.iter().any(|a| a.kind == kind)
        && !mission
            .scripts
            .events
            .iter()
            .any(|e| e.target != 255 && i16::from(e.target) == kind)
}

impl Settings {
    pub fn applies(
        self,
        actor: &Actor,
        mission: &tenchu_assets::stage::MissionAssets,
        player: usize,
        difficulty: Difficulty,
    ) -> bool {
        self.enabled
            && (!self.progression
                || mission.trial.is_some()
                || mission.stage.order
                    >= match difficulty {
                        Difficulty::Easy => 6,
                        Difficulty::Normal => 4,
                        Difficulty::Hard => 2,
                    })
            && eligible(actor, mission, player)
    }

    pub fn combat(self, actor: &Actor) -> Combat {
        self.assignment(actor.id, actor.kind, actor.behavior.combat)
    }

    pub fn assignment(self, id: usize, kind: i16, override_: Option<Combat>) -> Combat {
        match override_.unwrap_or(self.combat) {
            Combat::Mixed => {
                let seed = u32::from(self.seed)
                    ^ (id as u32).wrapping_mul(0x9e3779b9)
                    ^ u32::from(kind as u16).rotate_left(16);
                let mut random = crate::random::Random::new(seed);
                [
                    Combat::Coward,
                    Combat::Reckless,
                    Combat::Cautious,
                    Combat::Flanker,
                    Combat::Watchman,
                ][random.next15() as usize % 5]
            }
            profile => profile,
        }
    }
}

impl Controller<'_, '_> {
    pub(super) fn combat_profile(&self) -> Option<Combat> {
        let s = self.state.profiles;
        if !s.applies(
            self.actor,
            self.ctx.mission,
            self.player.id,
            self.state.difficulty,
        ) {
            return None;
        }
        let profile = s.combat(self.actor);
        (profile != Combat::Original).then_some(profile)
    }

    pub(super) fn profile_idle(&mut self) -> Result<Option<u16>> {
        let s = self.state.profiles;
        if !s.applies(
            self.actor,
            self.ctx.mission,
            self.player.id,
            self.state.difficulty,
        ) || self.actor.think.idle == programs::Idle::Sleep
            || self.actor.status == ActorState::Action
        {
            return Ok(None);
        }
        Ok(match self.actor.behavior.idle.unwrap_or(s.idle) {
            Idle::Original => None,
            Idle::Patrol => {
                let pad = if self.actor.patrol.is_some() {
                    self.actor.patrol_input(self.ctx.math, self.turn() as i16)?
                } else {
                    self.watch()
                };
                Some(if !s.quick_turns && pad == 0xf000 {
                    0
                } else {
                    pad
                })
            }
            Idle::Lookout => Some(self.watch()),
            Idle::TracePlayer => Some(self.trace()?),
        })
    }

    fn profile_strike(&mut self, range: i32) -> Result<u16> {
        let turn = self.goto(0, 0)? & (LEFT | RIGHT);
        if self.distance() < range && self.degree().abs() < 500 && self.clock > self.attack_clock()
        {
            self.delay_attack();
            Ok(turn | SQUARE)
        } else {
            Ok(turn | if self.distance() >= range { UP } else { 0 })
        }
    }

    pub(super) fn profile_combat(&mut self, profile: Combat) -> Result<u16> {
        if !matches!(
            self.actor.status,
            ActorState::Engage | ActorState::Chase | ActorState::Attack
        ) {
            return Ok(0);
        }
        let range = [2000, 3000, 4000, 18000][self.class()];
        let profile = if profile == Combat::Watchman {
            if self.actor.health < self.actor.max_health / 3
                || self.player.status == ActorState::Attack
            {
                Combat::Cautious
            } else if self.distance() > range + 2000 {
                Combat::Reckless
            } else {
                Combat::Flanker
            }
        } else {
            profile
        };
        let phase = (self.clock as u32).wrapping_add((self.actor.id as u32).wrapping_mul(13)) % 90;
        if self.actor.status == ActorState::Attack {
            let follow_through = match profile {
                Combat::Coward => false,
                Combat::Cautious => self.player.status != ActorState::Attack && phase < 30,
                Combat::Flanker => phase >= 45,
                _ => true,
            };
            return Ok(if follow_through {
                self.succession(range, 600)
            } else {
                0
            });
        }
        match profile {
            Combat::Coward => {
                if matches!(
                    self.actor.recruitment,
                    Recruitment::Available | Recruitment::Requested
                ) && self.ctx.mission.stage.id != 8
                    && self.can_recruit_with_limits(true)
                {
                    if self.actor.brain.timer != 2 {
                        self.actor.brain.chase = [self.player.position.x, self.player.position.z];
                        self.actor.brain.timer = 2;
                        self.actor.brain.count = 0;
                    }
                    let pad = self.alarm_with_limits(true)?;
                    if self.state.detection != Detection::Gone {
                        self.state.detection = Detection::None;
                    }
                    Ok(pad)
                } else {
                    self.escape()
                }
            }
            Combat::Reckless => self.profile_strike(range),
            Combat::Cautious => {
                if self.player.status != ActorState::Attack && phase < 30 {
                    self.profile_strike(range)
                } else {
                    let turn = self.goto(0, 0)? & (LEFT | RIGHT);
                    Ok(turn
                        | if self.distance() < range + 500 {
                            DOWN
                        } else {
                            0
                        })
                }
            }
            Combat::Flanker => {
                if self.distance() > range + 1000 || phase >= 45 {
                    self.profile_strike(range)
                } else if self.actor.status == ActorState::Engage && phase.is_multiple_of(15) {
                    Ok(self.command(
                        if (self.actor.id as u16 ^ self.state.profiles.seed) & 1 == 0 {
                            3
                        } else {
                            4
                        },
                    ))
                } else {
                    Ok(self.goto(0, 0)? & (LEFT | RIGHT))
                }
            }
            Combat::Original | Combat::Mixed | Combat::Watchman => unreachable!(),
        }
    }
}

impl crate::game::Game {
    pub fn set_behavior_profiles(&mut self, settings: Settings) {
        if self.intelligence.profiles == settings {
            return;
        }
        let was_enabled = self.intelligence.profiles.enabled;
        self.intelligence.profiles = settings;
        if !was_enabled && !settings.enabled {
            return;
        }
        for actor in self.roster.actors.iter_mut().flatten() {
            if eligible(actor, &self.mission, self.player_id) {
                actor.brain = Default::default();
                actor.pad = Default::default();
            }
        }
    }

    pub fn set_actor_behavior(&mut self, id: usize, overrides: Overrides) -> Result<()> {
        if self.cinematic.active {
            return Err("Wait for the cutscene to finish before editing guard behavior".into());
        }
        let actor = self
            .roster
            .actors
            .get(id)
            .and_then(Option::as_ref)
            .ok_or("Actor no longer exists")?;
        if !eligible(actor, &self.mission, self.player_id) {
            return Err("Behavior overrides are available for ordinary living guards, outside scripted scenes".into());
        }
        if actor.behavior == overrides {
            return Ok(());
        }
        let active = self.intelligence.profiles.applies(
            actor,
            &self.mission,
            self.player_id,
            self.intelligence.difficulty,
        );
        let actor = self.roster.get_mut(id);
        actor.behavior = overrides;
        if active {
            actor.brain = Default::default();
            actor.pad = Default::default();
        }
        Ok(())
    }
}
