use super::Game;
use crate::{Result, actor::Context, ids::*};
use serde::{Deserialize, Serialize};
use tenchu_assets::motion::Clip;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub unarmed: bool,
    pub speed: u16,
    pub visibility: u16,
    pub damage: u16,
    pub one_hit: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            unarmed: false,
            speed: 100,
            visibility: 100,
            damage: 100,
            one_hit: false,
        }
    }
}

impl Settings {
    pub fn validate(&mut self) {
        self.speed = self.speed.clamp(25, 200);
        self.visibility = self.visibility.clamp(25, 100);
        self.damage = self.damage.clamp(100, 400);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Motions<T> {
    pub idle: T,
    pub turn: T,
    pub draw: T,
    pub sheathe: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub settings: Settings,
    motions: Motions<usize>,
    attacks: Vec<(i16, usize)>,
    held: bool,
    pub(crate) requested: bool,
}

impl State {
    pub fn needs_unarmed_motions(&self) -> bool {
        self.attacks.is_empty()
    }

    pub(crate) fn unarmed_clip(&self, action: i16) -> Option<usize> {
        let action = match action {
            MOT_ATTACK_SLASH2_LEFT | MOT_ATTACK_SLASH2_RIGHT => MOT_ATTACK_SLASH2,
            MOT_ATTACK_RIGHT2_LEFT => MOT_ATTACK_RIGHT2,
            _ => action,
        };
        self.attacks
            .iter()
            .find(|(a, _)| *a == action)
            .map(|(_, c)| *c)
    }

    pub(crate) fn contains_attack(&self, clip: Option<usize>) -> bool {
        clip.is_some_and(|c| self.attacks.iter().any(|(_, index)| *index == c))
    }

    pub fn input(&mut self, held: bool) {
        self.requested = held && !self.held;
        self.held = held;
    }

    pub(crate) fn locomotion(action: i16) -> i16 {
        match action {
            MOT_MOVE => MOT_CHASE,
            MOT_MOVE_BACK => MOT_CHASE_BACK,
            MOT_MOVE_DASH_FWD => MOT_CHASE_DASH_FWD,
            MOT_MOVE_DASH_BACK => MOT_CHASE_DASH_BACK,
            MOT_MOVE_DASH_LEFT => MOT_CHASE_DASH_LEFT,
            MOT_MOVE_DASH_RIGHT => MOT_CHASE_DASH_RIGHT,
            _ => action,
        }
    }

    pub(crate) fn clip(&self, action: i16) -> Option<usize> {
        Some(match action {
            MOT_NORMAL => self.motions.idle,
            MOT_NORMAL_TURN_L | MOT_NORMAL_TURN_R => self.motions.turn,
            MOT_STATE_DRAW => self.motions.draw,
            MOT_STATE_SHEATHE => self.motions.sheathe,
            _ => return None,
        })
    }
}

impl Game {
    pub(crate) fn import_clip(&mut self, source: &Clip) -> usize {
        self.mission
            .clips
            .iter()
            .position(|c| c == source)
            .unwrap_or_else(|| {
                let clips = std::sync::Arc::make_mut(&mut self.mission.clips);
                clips.push(source.clone());
                clips.len() - 1
            })
    }

    pub fn set_player_sheathing(
        &mut self,
        mut settings: Settings,
        motions: Option<&Motions<Clip>>,
    ) -> Result<()> {
        settings.validate();
        if settings.enabled {
            if self.player().sheathing.is_none() {
                let motions = motions.ok_or("Missing player sheathing motions")?;
                if [
                    &motions.idle,
                    &motions.turn,
                    &motions.draw,
                    &motions.sheathe,
                ]
                .iter()
                .any(|c| c.bones.is_empty())
                    || motions.draw.duration <= 0
                    || motions.sheathe.duration <= 0
                {
                    return Err("Invalid player sheathing animations".into());
                }
                let motions = Motions {
                    idle: self.import_clip(&motions.idle),
                    turn: self.import_clip(&motions.turn),
                    draw: self.import_clip(&motions.draw),
                    sheathe: self.import_clip(&motions.sheathe),
                };
                self.roster.get_mut(self.player_id).sheathing = Some(State {
                    settings,
                    motions,
                    attacks: Vec::new(),
                    held: false,
                    requested: false,
                });
            } else {
                self.roster
                    .get_mut(self.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .settings = settings;
            }
        } else if self.player().sheathing.is_none() {
            return Ok(());
        }
        let player = self.roster.get_mut(self.player_id);
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
        if !settings.enabled {
            let stowed = player.attribute & ATTR_WEAPON_DRAWN == 0;
            let unarmed_attack = player.unarmed_attack();
            if unarmed_attack {
                player.clear_attack(&mut ctx, 3)?;
            }
            player.sheathing = None;
            player.equip(&ctx, true);
            if player.health > 0
                && !player.scripted
                && !self.intelligence.cinematic
                && (stowed
                    || matches!(
                        player.motion.action,
                        MOT_NORMAL
                            | MOT_NORMAL_TURN_L
                            | MOT_NORMAL_TURN_R
                            | MOT_STATE_DRAW
                            | MOT_STATE_SHEATHE
                    ))
                && (player.free_stance()
                    || unarmed_attack
                    || matches!(player.motion.action, MOT_STATE_DRAW | MOT_STATE_SHEATHE))
            {
                player.enter(&ctx, MOT_ENGAGE_STANCE, true);
                player.play_pose(&mut ctx)?;
                player.initialize_rendered(ctx.scene, ctx.math);
            }
        } else if matches!(player.motion.action, MOT_CHASE | MOT_CHASE_BACK) {
            player.motion_velocity(&ctx);
        }
        Ok(())
    }

    pub fn set_player_unarmed(&mut self, attacks: &[(i16, Clip)]) -> Result<()> {
        if self.player().sheathing.is_none() {
            return Err("Enable player sheathing before importing unarmed attacks".into());
        }
        if !attacks.iter().any(|(action, _)| *action == MOT_ATTACK)
            || attacks.iter().any(|(action, clip)| {
                !matches!(*action, MOT_ATTACK..=MOT_ATTACK_LUNGE | MOT_ATTACK_BACK..=MOT_ATTACK_LUNGE_BACK)
                    || clip.bones.is_empty() || clip.duration <= 0
                    || !self.mission.weapons.attacks.iter().any(|a| a.clip == clip.id && a.start > 0)
            }) {
            return Err("Unarmed attacks require animated combat clips with valid hit timings".into());
        }
        let attacks = attacks
            .iter()
            .map(|(a, c)| (*a, self.import_clip(c)))
            .collect();
        self.roster
            .get_mut(self.player_id)
            .sheathing
            .as_mut()
            .unwrap()
            .attacks = attacks;
        Ok(())
    }
}
