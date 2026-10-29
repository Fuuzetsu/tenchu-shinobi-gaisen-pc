use super::Game;
use crate::{Result, actor::Context, ids::*, motion::Motion, scene::ModelId};
use tenchu_assets::{executable::Character, motion::Clip};

#[derive(Default)]
pub struct State {
    pub kind: Option<i16>,
    original: Option<usize>,
    definition: Option<usize>,
    weapons: Vec<ModelId>,
}

impl Game {
    pub fn set_player_moveset(
        &mut self,
        character: Option<&Character>,
        clips: &[Clip],
    ) -> Result<()> {
        let kind = character.map(|c| c.kind);
        if kind == self.moveset.kind {
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
        Ok(())
    }
}
