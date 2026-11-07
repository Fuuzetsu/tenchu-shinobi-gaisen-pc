use super::Game;
use crate::{Result, actor::Context, ids::*};
use serde::{Deserialize, Serialize};
use tenchu_assets::motion::Clip;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnarmedStyle {
    #[default]
    Onikage,
    Tatsumaru,
}

impl UnarmedStyle {
    pub fn name(self) -> &'static str {
        match self {
            Self::Onikage => "Onikage (Gaisen)",
            Self::Tatsumaru => "Tatsumaru (Tenchu 2)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub unarmed: bool,
    pub unarmed_style: UnarmedStyle,
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
            unarmed_style: UnarmedStyle::Onikage,
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
    pub weapon_program: Option<i16>,
    motions: Motions<usize>,
    attacks: Vec<AttackMotion>,
    loaded_style: UnarmedStyle,
    held: bool,
    pub(crate) requested: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AttackMotion {
    action: i16,
    clip: usize,
    contact: Option<usize>,
}

impl State {
    pub fn needs_unarmed_motions(&self, style: UnarmedStyle) -> bool {
        self.attacks.is_empty() || self.loaded_style != style
    }

    pub(crate) fn contact(&self, clip: Option<usize>) -> Option<usize> {
        self.attacks
            .iter()
            .find(|a| Some(a.clip) == clip)
            .and_then(|a| a.contact)
    }

    pub(crate) fn unarmed_clip(&self, action: i16) -> Option<usize> {
        let action = if self.loaded_style == UnarmedStyle::Tatsumaru {
            match action {
                MOT_ATTACK_RIGHT2 | MOT_ATTACK_RIGHT2_LEFT => MOT_ATTACK_RIGHT1,
                MOT_ATTACK_LEFT2 | MOT_ATTACK_LEFT2_RIGHT => MOT_ATTACK_LEFT1,
                MOT_ATTACK_LUNGE_BACK => MOT_ATTACK_BACK,
                _ => action,
            }
        } else {
            match action {
                MOT_ATTACK_SLASH2_LEFT | MOT_ATTACK_SLASH2_RIGHT => MOT_ATTACK_SLASH2,
                MOT_ATTACK_RIGHT2_LEFT => MOT_ATTACK_RIGHT2,
                _ => action,
            }
        };
        self.attacks
            .iter()
            .find(|a| a.action == action)
            .map(|a| a.clip)
    }

    pub(crate) fn tatsumaru(&self) -> bool {
        self.loaded_style == UnarmedStyle::Tatsumaru
    }

    pub(crate) fn contains_attack(&self, clip: Option<usize>) -> bool {
        clip.is_some_and(|c| self.attacks.iter().any(|a| a.clip == c))
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
                    weapon_program: None,
                    motions,
                    attacks: Vec::new(),
                    loaded_style: UnarmedStyle::Onikage,
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
            .map(|(a, c)| AttackMotion {
                action: *a,
                clip: self.import_clip(c),
                contact: None,
            })
            .collect();
        let state = self
            .roster
            .get_mut(self.player_id)
            .sheathing
            .as_mut()
            .unwrap();
        state.attacks = attacks;
        state.loaded_style = UnarmedStyle::Onikage;
        Ok(())
    }

    pub fn set_player_tatsumaru(&mut self, moves: &tenchu_assets::tenchu2::Moves) -> Result<()> {
        moves.validate()?;
        if self.player().sheathing.is_none() {
            return Err("Enable player sheathing before importing unarmed attacks".into());
        }
        if self.player().unarmed_attack() {
            return Err("Finish the unarmed attack before changing its moveset".into());
        }
        let actions = [
            MOT_ATTACK,
            MOT_ATTACK_SLASH2,
            MOT_ATTACK_SLASH2_RIGHT,
            MOT_ATTACK_SLASH2_LEFT,
            MOT_ATTACK_SLASH3,
            MOT_ATTACK_SLASH4,
            MOT_UNARMED_FINISH,
            MOT_ATTACK_RIGHT1,
            MOT_ATTACK_LEFT1,
            MOT_ATTACK_LUNGE,
            MOT_ATTACK_BACK,
            MOT_ATTACK_CROUCH,
        ];
        let mut attacks = Vec::new();
        for (source, action) in moves.attacks.iter().zip(actions) {
            let mut clip = source.clip.clone();
            // Keep imported IDs clear of Gaisen's special attack dispatch.
            clip.id += 0x6000;
            let mut timing = source.attack.clone();
            timing.clip = clip.id;
            let table = &mut self.mission.weapons.attacks;
            if let Some(old) = table.iter_mut().find(|a| a.clip == clip.id) {
                *old = timing;
            } else {
                table.push(timing);
            }
            attacks.push(AttackMotion {
                action,
                clip: self.import_clip(&clip),
                contact: Some(source.part),
            });
        }
        let state = self
            .roster
            .get_mut(self.player_id)
            .sheathing
            .as_mut()
            .unwrap();
        state.attacks = attacks;
        state.loaded_style = UnarmedStyle::Tatsumaru;
        Ok(())
    }
}
