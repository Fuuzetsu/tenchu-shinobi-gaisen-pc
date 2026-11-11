use crate::visuals::pack::Pack;
use crate::{
    settings::Settings,
    visuals::appearance::library::{Body, Library},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tenchu_assets::{
    Result,
    model::Part,
    tenchu2::{BONES, characters, motions, registry, weapons},
    weapon::{Contact, ImportedAttack, kind},
};
use tenchu_core::{
    game::{
        Game,
        moveset::{Options, T2_ACTIONS},
    },
    math::Matrix,
    motion::Pose,
    tenchu2::{
        actor::{self, State},
        motion::Motion,
        rig::{Rig, WeaponMode},
        world::{Assets, World},
    },
};

mod convert;
pub mod story;
use convert::{convert_equipment, convert_moves, convert_weapons, native_parts};
pub(super) use convert::{native_sources, placement_parts};

#[derive(Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Selection {
    body: String,
    moveset: String,
    weapon: String,
    options: Options,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Override {
    applied: Selection,
    pub custom: Option<Custom>,
    pub voice: Option<Arc<tenchu_assets::audio::voice::VoiceSet>>,
    pub weapon_voice: Option<Arc<tenchu_assets::audio::voice::VoiceSet>>,
    original: Option<Original>,
    weapon_pool: Vec<Vec<usize>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Custom {
    pub id: String,
    pub pack: Arc<Pack>,
}

impl Custom {
    fn selected(selection: &str, pack: &Arc<Pack>) -> Result<Option<Self>> {
        let Some(id) = selection.strip_prefix("pack:") else {
            return Ok(None);
        };
        pack.player_models
            .get(id)
            .ok_or("Selected body is missing from the model folder")?
            .validate()?;
        Ok(Some(Self {
            id: id.into(),
            pack: pack.clone(),
        }))
    }

    fn same(a: Option<&Self>, b: Option<&Self>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => a.id == b.id && Arc::ptr_eq(&a.pack, &b.pack),
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Original {
    definition: registry::Actor,
    rig: Rig,
    turn: i16,
    drawn: bool,
}

impl Override {
    #[cfg(test)]
    pub(super) fn validate_state(&self, world: &World, assets: &Assets) -> Result<()> {
        let valid_model = |id: usize| id < world.scene.models.len();
        if self.original.as_ref().is_some_and(|original| {
            !assets.models.contains_key(&original.definition.model)
                || !valid_model(original.rig.root)
                || original.rig.joints.iter().any(|&id| !valid_model(id))
                || original
                    .rig
                    .weapons
                    .iter()
                    .flatten()
                    .any(|&id| !valid_model(id))
        }) || self
            .weapon_pool
            .iter()
            .flatten()
            .any(|&id| !valid_model(id))
        {
            return Err("Invalid saved original rig or weapon attachment".into());
        }
        if let Some(custom) = &self.custom {
            let body = custom
                .pack
                .player_models
                .get(&custom.id)
                .ok_or("Missing saved custom player model")?;
            body.validate()?;
            for (index, part) in body.parts.iter().enumerate() {
                let key = format!("player/{}/{index}", custom.id);
                if !custom.pack.models.contains_key(&key)
                    || part.skin.is_some() && !custom.pack.skins.contains_key(&key)
                {
                    return Err("Missing saved custom player geometry".into());
                }
            }
            for image in custom
                .pack
                .textures
                .values()
                .chain(custom.pack.images.values())
            {
                let [width, height] = image.size;
                if width == 0
                    || height == 0
                    || (width as usize)
                        .checked_mul(height as usize)
                        .and_then(|n| n.checked_mul(4))
                        != Some(image.rgba.len())
                {
                    return Err("Invalid saved custom player texture".into());
                }
            }
            for patch in custom.pack.model_patches.values().flatten() {
                patch.validate()?;
            }
        }
        Ok(())
    }

    pub fn diagnostic_snapshot(&self) -> serde_json::Value {
        serde_json::json!({"body": self.applied.body, "moveset": self.applied.moveset,
            "weapon": self.applied.weapon,
            "custom": self.custom.as_ref().map(|c| &c.id),
            "source_voice": self.voice.as_ref().map(|v| format!("{:?}", v.dialect))})
    }

    pub fn sync(
        &mut self,
        world: &mut World,
        assets: &mut Assets,
        donor: &mut Game,
        library: &Library,
        bodies: &Arc<Pack>,
        settings: &Settings,
    ) -> Result<bool> {
        let custom = Custom::selected(&settings.player_model, bodies)?;
        let custom_changed = !Custom::same(self.custom.as_ref(), custom.as_ref());
        let body_id = if custom.is_some() {
            ""
        } else {
            settings.player_model.as_str()
        };
        let moveset_id = settings.moveset_id().unwrap_or_default();
        let options = if settings.model_moveset {
            settings.model_moveset_options
        } else {
            Options::default()
        };
        let moveset = library
            .moveset(settings)?
            .filter(|body| compatible_moveset(body));
        let weapon = library.weapon(&settings.weapon_model)?;
        let voice = library.voice(&settings.voice_model)?;
        let voice = voice
            .or_else(|| moveset.and_then(|body| body.moveset_voice()))
            .cloned();
        let weapon_voice = weapon
            .or(moveset)
            .and_then(|body| body.moveset_voice())
            .cloned();
        if self.applied.body == body_id
            && self.applied.moveset == moveset_id
            && self.applied.weapon == settings.weapon_model
            && self.applied.options == options
        {
            self.voice = voice;
            self.weapon_voice = weapon_voice;
            self.custom = custom;
            return Ok(custom_changed);
        }
        let selection = Selection {
            body: body_id.into(),
            moveset: moveset_id.into(),
            weapon: settings.weapon_model.clone(),
            options,
        };
        let rig_changed = self.applied.body != selection.body
            || self.applied.moveset != selection.moveset
            || self.applied.options != selection.options;
        let owner = world.player;
        let actor = &world.actors[owner].actor;
        if actor.health <= 0
            || !matches!(actor.state, State::Normal | State::Ready | State::Crouched)
            || !world.mission.input_allowed()
        {
            return Ok(false);
        }
        let body = if selection.body.is_empty() {
            None
        } else {
            Some(
                library
                    .get(&selection.body)
                    .ok_or("Selected model is unavailable")?,
            )
        };
        if moveset.is_none() && settings.model_moveset && !settings.moveset_model.is_empty() {
            return Err("This moveset needs projectile or boss-script support that is not connected to native T2 yet. Its appearance can still be selected.".into());
        }
        let original = self.original.get_or_insert_with(|| Original {
            definition: assets.resources.registry.actor(actor.kind).unwrap().clone(),
            rig: world.instances[owner].rig.clone(),
            turn: actor.turn,
            drawn: actor.weapon_drawn(),
        });
        let replacement_parts = body
            .map(|body| native_parts(body, &assets.models[&original.definition.model].parts))
            .transpose()?;
        let mut definition = original.definition.clone();
        definition.kind = -1;
        let mut imported = BTreeMap::new();
        let mut weapon_parts = None;
        if let Some(source) = moveset {
            source.apply_moveset(donor, Options::default())?;
            imported = convert_moves(source, donor, &mut definition, assets, selection.options)?;
            if definition.weapon < 0 {
                weapon_parts = Some(convert_weapons(
                    donor,
                    definition.weapon,
                    &imported,
                    assets,
                )?);
            }
        } else {
            assets.resources.registry.replace_imported_weapon_events([]);
        }
        if let Some(weapon) = weapon {
            let equipment = weapon.equipment.as_ref().unwrap();
            let contacts = assets
                .resources
                .weapons
                .parts(definition.weapon)
                .filter(|part| part.radius > 0)
                .cloned()
                .collect();
            definition.weapon = match equipment.draw {
                tenchu_assets::weapon::Draw::Fixed => i16::MIN,
                tenchu_assets::weapon::Draw::Single => i16::MIN + 0x100,
                tenchu_assets::weapon::Draw::Dual => i16::MIN + 0x200,
            };
            weapon_parts = Some(convert_equipment(
                equipment,
                definition.weapon,
                contacts,
                assets,
            ));
        }
        if let Some(body) = body {
            let name = "override-player".to_owned();
            let parts = replacement_parts.unwrap();
            assets
                .model_sources
                .insert(name.clone(), native_sources(body, parts.len()));
            assets.models.insert(
                name.clone(),
                characters::Rig {
                    parts,
                    vram: (*body.vram).clone(),
                },
            );
            definition.model = name;
        } else {
            assets.models.remove("override-player");
            assets.model_sources.remove("override-player");
        }
        if weapon_parts.is_none() {
            assets
                .weapons
                .retain(|name, _| !assets.weapon_imports.contains_key(name));
            assets.weapon_imports.clear();
        }
        let parts = &assets.models[&definition.model].parts;
        let motion = rig_changed.then(|| Motion::new(parts)).transpose()?;
        let point = if rig_changed {
            Some(
                world.instances[owner]
                    .rig
                    .root_position(&world.actors[owner].actor, &mut world.scene)?,
            )
        } else {
            None
        };
        world.effects.entries.retain(|effect| !matches!(effect,
            tenchu_core::tenchu2::effects::Effect::Ribbon(ribbon)
                if matches!(ribbon.source, tenchu_core::tenchu2::effects::ribbon::Source::Weapon { actor, .. } if actor == owner)));
        let actor = &mut world.actors[owner].actor;
        let rig = &mut world.instances[owner].rig;
        actor.set_weapon(
            rig,
            owner,
            &mut world.collision,
            &assets.resources,
            WeaponMode::Remove,
        )?;
        // Reclaim removed contacts even when browsing repeatedly while paused.
        world.collision.compute(|model| world.scene.world(model))?;
        if let Some(point) = point {
            actor.commit_root(point);
            actor.velocity = Default::default();
        }
        actor.attack = Default::default();
        actor.flags &= !0x200000;
        let drawn = actor.weapon_drawn();
        let crouching = actor.state == State::Crouched;
        actor.flags &= !0x40;
        if rig_changed {
            // Reuse joint slots across swaps; no orphaned collision bodies or growing rigs.
            while rig.joints.len() < parts.len() {
                rig.joints
                    .push(world.scene.add(Some(rig.root), Matrix::default()));
            }
            for (index, part) in parts.iter().enumerate() {
                let parent = parts
                    .iter()
                    .position(|p| p.child == part.parent)
                    .map_or(rig.root, |p| rig.joints[p]);
                let joint = &mut world.scene.models[rig.joints[index]];
                joint.parent = Some(parent);
                joint.flags = 0;
                joint.local = Matrix {
                    translation: part.position.map(i32::from).into(),
                    ..Default::default()
                };
            }
        }
        let active = body.is_some() || moveset.is_some() || weapon.is_some();
        if active {
            assets.resources.registry.actors.retain(|a| a.kind != -1);
            assets.resources.registry.actors.push(definition.clone());
            actor.moveset = Some(Box::new(actor::Override {
                definition: -1,
                attacks: imported,
                no_idle_taunts: selection.options.no_idle_taunts,
                retain_combo_pose: moveset.is_some_and(|m| m.imported_moveset.is_none()),
            }));
        } else {
            actor.moveset = None;
        }
        if let Some(parts) = weapon_parts {
            rig.weapons = [None; 5];
            while self.weapon_pool.len() < parts.len() {
                self.weapon_pool.push(Vec::new());
            }
            for (weapon, pool) in parts.iter().zip(&mut self.weapon_pool) {
                let Some(name) = &weapon.model else { continue };
                let geometry = &assets.weapons[name];
                while pool.len() < geometry.len() + 1 {
                    pool.push(world.scene.add(None, Matrix::default()));
                }
                let root = pool[0];
                world.scene.models[root].parent = Some(rig.joints[weapon.bone]);
                world.scene.update(root, Matrix::default());
                let pose = Pose::new(geometry)?;
                // Imported weapon geometry has one rigid part.
                world.scene.models[pool[1]].parent = Some(root);
                pose.publish(
                    &mut world.scene,
                    &pool[1..],
                    &assets.resources.math,
                    u32::MAX,
                    geometry.len(),
                );
                rig.weapons[weapon.slot] = Some(root);
            }
        } else {
            rig.weapons = original.rig.weapons;
            if original.drawn {
                if matches!(original.definition.weapon & 0xf00, 0x100 | 0x200) {
                    rig.weapons.swap(0, 3);
                }
                if original.definition.weapon & 0xf00 == 0x200 {
                    rig.weapons.swap(1, 4);
                }
            }
        }
        if let Some(mut motion) = motion {
            let action = if crouching {
                0x200
            } else if drawn {
                0x100
            } else {
                0
            };
            motion
                .select(
                    action,
                    actor.definition_kind(),
                    &assets.resources.registry,
                    &assets.resources.clips,
                )?
                .ok_or("Missing override idle")?;
            actor.motion = motion;
            actor.state = State::from_action(action)?;
            actor.turn = if moveset.is_some() {
                definition.turn
            } else {
                original.turn
            };
        }
        if drawn {
            actor.set_weapon(
                rig,
                owner,
                &mut world.collision,
                &assets.resources,
                WeaponMode::Draw,
            )?;
        }
        if rig_changed {
            actor.set_body_collision(rig.joints[0], owner, &mut world.collision, true)?;
        }
        world.instances[owner].model_height = actor::model_bounds(parts)?[1];
        world.publish(owner, assets);
        world.scene.finish_tick();
        self.voice = voice;
        self.weapon_voice = weapon_voice;
        self.custom = custom;
        self.applied = selection;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub struct Gallery {
    pub library: Library,
    pub game: Game,
    pub bodies: Arc<Pack>,
    pub picker: crate::visuals::appearance::picker::Picker,
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
impl Gallery {
    pub fn remembered(directory: &str) -> Result<Self> {
        let saved = crate::storage::UserFile::LastDisc.path()?;
        let path = crate::startup::remembered_path(&saved)
            .ok_or("Open Gaisen once with play to connect its disc to the cross-game gallery.")?;
        let assets = crate::startup::open(&path)?;
        let library = Library::load(&assets)?;
        let game = donor(&assets)?;
        Ok(Self {
            library,
            game,
            bodies: Pack::bodies(directory)?,
            picker: Default::default(),
        })
    }
    pub fn state(
        &mut self,
        original_model: Option<String>,
    ) -> crate::visuals::appearance::picker::State<'_> {
        crate::visuals::appearance::picker::State {
            picker: &mut self.picker,
            library: Some(&self.library),
            bodies: &self.bodies,
            game: &self.game,
            original_model,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn donor(assets: &tenchu_assets::game::GameAssets<crate::source::DiscSource>) -> Result<Game> {
    let mut game = crate::session::Options::default().load(assets)?;
    for _ in 0..30 {
        game.step(0)?;
    }
    Ok(game)
}

pub fn compatible_moveset(body: &Body) -> bool {
    body.moveset_character().is_some_and(|c| {
        !matches!(
            c.weapon,
            kind::YUMI | kind::TEPPO | kind::GUN | kind::KATAYUMI
        )
    })
}
