use super::Game;
use crate::{Result, actor::Context, ids::*, motion::Motion, scene::ModelId};
use tenchu_assets::{executable::Character, motion::Clip};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub player_movement: bool,
    pub no_idle_taunts: bool,
}

#[derive(Default)]
pub struct State {
    pub kind: Option<i16>,
    pub options: Options,
    original: Option<usize>,
    definition: Option<usize>,
    weapons: Vec<ModelId>,
}

impl State {
    pub fn matches(&self, kind: Option<i16>, options: Options) -> bool {
        self.kind == kind && (kind.is_none() || self.options == options)
    }
}

impl Game {
    pub fn set_player_moveset(
        &mut self,
        character: Option<&Character>,
        clips: &[Clip],
        options: Options,
    ) -> Result<()> {
        let kind = character.map(|c| c.kind);
        if self.moveset.matches(kind, options) {
            return Ok(());
        }
        if self.cinematic.active || self.player().scripted || self.player().health <= 0 {
            return Err("Change movesets with a living player outside a cutscene".into());
        }
        let original = *self
            .moveset
            .original
            .get_or_insert(self.player().definition);
        let base = &self.mission.characters[original];
        let replacement = character.map(|source| {
            let mut result = base.clone();
            let owns_attack = source.motions.iter().any(|r| r.action == MOT_ATTACK && clips.iter().any(|c| c.id == r.clip));
            if owns_attack {
                result.weapon = source.weapon;
                result.motions.retain(|r| !matches!(r.action, MOT_ATTACK..=MOT_ATTACK_LUNGE | MOT_ATTACK_BACK..=MOT_ATTACK_TAUNT));
            }
            for entry in &source.motions {
                if options.player_movement && is_movement(entry.action) {
                    continue;
                }
                if clips.iter().any(|c| c.id == entry.clip) {
                    result.motions.retain(|r| r.action != entry.action);
                    result.motions.push(entry.clone());
                }
            }
            result
        });
        let actor = self.roster.get_mut(self.player_id);
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
        actor.clear_attack(&mut ctx, 3)?;
        if let Some(replacement) = replacement {
            for registration in &replacement.motions {
                if !ctx.mission.clips.iter().any(|c| c.id == registration.clip)
                    && let Some(clip) = clips.iter().find(|c| c.id == registration.clip)
                {
                    ctx.mission.clips.push(clip.clone());
                }
            }
            let index = *self.moveset.definition.get_or_insert_with(|| {
                ctx.mission.characters.push(replacement.clone());
                ctx.mission.characters.len() - 1
            });
            ctx.mission.characters[index] = replacement;
            actor.definition = index;
        } else {
            actor.definition = original;
        }
        actor.rebuild_weapons(&mut ctx, &mut self.moveset.weapons)?;
        actor.motion = Motion::new(&ctx.mission.models[&actor.mesh_kind])?;
        let action = actor.return_normal(&mut ctx, self.player_id)?;
        actor.enter(&ctx, action, false);
        actor.velocity = [0; 3];
        actor.play_pose(&mut ctx)?;
        actor.initialize_rendered(ctx.scene, ctx.math);
        ctx.collision.compute(ctx.scene)?;
        self.moveset.kind = kind;
        self.moveset.options = if kind.is_some() {
            options
        } else {
            Options::default()
        };
        actor.suppress_idle_taunts = self.moveset.options.no_idle_taunts;
        Ok(())
    }
}

fn is_movement(action: i16) -> bool {
    matches!(action,
        MOT_NORMAL_TURN_R | MOT_NORMAL_TURN_L
        | MOT_MOVE..=MOT_MOVE_DASH_LEFT
        | MOT_SWIM..=MOT_SWIM_STROKE
        | MOT_KAGI..=MOT_KAGI_PULL
        | MOT_ENGAGE_TURN_R | MOT_ENGAGE_TURN_L
        | MOT_CHASE..=MOT_CHASE_DASH_FWD
        | MOT_STATE_CLIMB | MOT_STATE_FALL..=MOT_STATE_LAND_FLIP
        | MOT_JUMP..=MOT_JUMP_FLIP
        | MOT_HANG..=MOT_HANG_PULLUP
        | MOT_SQUAT..=MOT_SQUAT_BACKFLIP
        | MOT_STICKON..=MOT_STICKON_SLIDE_R
    )
}
