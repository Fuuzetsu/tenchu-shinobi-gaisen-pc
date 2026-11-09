use super::Game;
use crate::{Result, actor::Context, ids::*, motion::Motion, scene::ModelId};
use tenchu_assets::{executable::Character, motion::Clip};

pub struct Imported {
    pub character: Character,
    pub clips: Vec<Clip>,
    pub attacks: Vec<(
        tenchu_assets::weapon::Attack,
        tenchu_assets::weapon::ImportedAttack,
    )>,
    pub equipment: tenchu_assets::weapon::Equipment,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    pub player_movement: bool,
    pub no_idle_taunts: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
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
    pub fn set_imported_player_moveset(
        &mut self,
        source: &Imported,
        options: Options,
    ) -> Result<()> {
        if self.moveset.matches(Some(source.character.kind), options) {
            return Ok(());
        }
        self.set_player_moveset(Some(&source.character), &source.clips, options)?;
        for (attack, contacts) in &source.attacks {
            let attacks = &mut self.mission.weapons.attacks;
            if let Some(existing) = attacks.iter_mut().find(|a| a.clip == attack.clip) {
                *existing = attack.clone();
            } else {
                attacks.push(attack.clone());
            }
            self.mission
                .weapons
                .imported_attacks
                .insert(attack.clip, contacts.clone());
        }
        self.import_player_weapons(&source.equipment)?;
        Ok(())
    }

    fn import_player_weapons(
        &mut self,
        equipment: &tenchu_assets::weapon::Equipment,
    ) -> Result<()> {
        use crate::actor::Weapon;
        use crate::math::Matrix;
        use std::sync::Arc;
        let actor = self.roster.get_mut(self.player_id);
        actor.weapons.fill(None);
        actor.imported_weapon_draw = Some(equipment.draw);
        for attachment in &equipment.attachments {
            let model = &attachment.model;
            let weapons = &mut self.mission.weapons;
            let kind = if let Some((&id, _)) = weapons
                .imported_models
                .iter()
                .find(|(_, m)| m.id == model.id)
            {
                id
            } else {
                let id = (i16::MIN..0)
                    .find(|id| !weapons.models.contains_key(id))
                    .ok_or("Too many imported weapons")?;
                Arc::make_mut(&mut weapons.models).insert(id, model.geometry.clone());
                weapons.names.insert(id, model.id.clone());
                Arc::make_mut(&mut weapons.imported_models).insert(id, model.clone());
                id
            };
            let parent = *actor
                .parts
                .get(attachment.part)
                .ok_or("Missing imported weapon attachment")?;
            let slot = attachment.slot;
            let model = *self
                .moveset
                .weapons
                .get(slot)
                .ok_or("Invalid imported weapon slot")?;
            self.scene.models[model].parent = Some(parent);
            self.scene.update(model, Matrix::default());
            actor.weapons[slot] = Some(Weapon {
                kind,
                model,
                rendered: Matrix::default(),
                imported_contact: Some(attachment.melee),
            });
        }
        if actor.attribute & ATTR_WEAPON_DRAWN != 0 {
            equipment.draw.swap(&mut actor.weapons);
        }
        actor.initialize_rendered(&self.scene, &self.math);
        Ok(())
    }

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
                    std::sync::Arc::make_mut(&mut ctx.mission.clips).push(clip.clone());
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

// Gaisen actions with an equivalent in T2's standing, crouch, jump and melee
// controllers. Paired kills and item scripts keep their host registrations.
pub const T2_ACTIONS: &[(i16, i16)] = &[
    (MOT_NORMAL, 0),
    (MOT_NORMAL_TURN_R, 1),
    (MOT_NORMAL_TURN_L, 2),
    (MOT_MOVE, 3),
    (MOT_MOVE_BACK, 4),
    (MOT_ENGAGE_STANCE, 0x100),
    (MOT_ENGAGE_TURN_R, 0x101),
    (MOT_ENGAGE_TURN_L, 0x102),
    (MOT_CHASE, 0x103),
    (MOT_CHASE_BACK, 0x104),
    (MOT_SQUAT, 0x200),
    (MOT_SQUAT_WALK_F, 0x201),
    (MOT_SQUAT_WALK_B, 0x202),
    (MOT_SQUAT_WALK_R, 0x203),
    (MOT_SQUAT_WALK_L, 0x204),
    (MOT_SQUAT_ROLL_F, 0x210),
    (MOT_SQUAT_ROLL_B, 0x211),
    (MOT_SQUAT_ROLL_R, 0x212),
    (MOT_SQUAT_ROLL_L, 0x213),
    (MOT_SQUAT_BACKFLIP, 0x220),
    (MOT_STICKON, 0x300),
    (MOT_STICKON_SLIDE_L, 0x301),
    (MOT_STICKON_SLIDE_R, 0x302),
    (MOT_JUMP, 0x600),
    (MOT_JUMP_FORWARD, 0x601),
    (MOT_JUMP_BACK, 0x602),
    (MOT_JUMP_RIGHT, 0x603),
    (MOT_JUMP_LEFT, 0x604),
    (MOT_JUMP_WALLKICK, 0x605),
    (MOT_JUMP_RUN, 0x606),
    (MOT_JUMP_FLIP, 0x607),
    (MOT_ATTACK, 0x900),
    (MOT_ATTACK_SLASH2, 0x901),
    (MOT_ATTACK_SLASH2_RIGHT, 0x902),
    (MOT_ATTACK_SLASH2_LEFT, 0x903),
    (MOT_ATTACK_SLASH3, 0x904),
    (MOT_ATTACK_SLASH4, 0x905),
    (MOT_UNARMED_FINISH, 0x906),
    (MOT_ATTACK_RIGHT1, 0x907),
    (MOT_ATTACK_RIGHT2, 0x908),
    (MOT_ATTACK_RIGHT2_LEFT, 0x909),
    (MOT_ATTACK_LEFT1, 0x90a),
    (MOT_ATTACK_LEFT2, 0x90b),
    (MOT_ATTACK_LEFT2_RIGHT, 0x90c),
    (MOT_ATTACK_LUNGE, 0x910),
    (MOT_ATTACK_CROUCH, 0x914),
    (MOT_ATTACK_BACK, 0x915),
];
