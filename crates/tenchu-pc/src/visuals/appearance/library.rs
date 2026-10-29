use crate::visuals::catalog;
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    ReadAt, Result,
    executable::{Character, Executable, MotionRegistration},
    game::GameAssets,
    model::{self, Part},
    motion::Clip,
    stage::MissionAssets,
    texture::Vram,
};

pub struct Body {
    pub id: String,
    pub name: String,
    pub resource: String,
    pub parts: Vec<Part>,
    pub vram: Arc<Vram>,
    pub character: Option<Character>,
    pub clips: Arc<Vec<Clip>>,
    pub idle: Option<usize>,
    pub think: i16,
    pub voice: Option<tenchu_assets::stage::imports::Voice>,
}

#[derive(Default)]
pub struct Library {
    pub bodies: Vec<Body>,
    aliases: BTreeMap<String, usize>,
}

impl Library {
    pub fn get(&self, id: &str) -> Option<&Body> {
        self.aliases.get(id).map(|&index| &self.bodies[index])
    }

    pub fn load(assets: &GameAssets<impl ReadAt>) -> Result<Self> {
        let definitions = Executable::new(&assets.executable)?.characters()?;
        let mut found = BTreeMap::new();
        let mut thinking = BTreeMap::new();
        let mut voices = BTreeMap::new();
        let mut textures = Vec::new();
        for stage in assets.stages()? {
            let mission = MissionAssets::load(assets, stage.id, 0, 0)?;
            for spawn in mission.enemies.iter().chain(
                mission
                    .random_layouts
                    .iter()
                    .flatten()
                    .flat_map(|l| &l.enemies),
            ) {
                if spawn.think != 0 {
                    thinking.entry(spawn.kind).or_insert(spawn.think);
                }
            }
            for spawn in &mission.story {
                if spawn.think != 0 {
                    thinking.entry(spawn.kind).or_insert(spawn.think);
                }
            }
            for (program, kind) in Executable::new(&assets.executable)?
                .appearance(stage.id)?
                .into_iter()
                .enumerate()
            {
                voices
                    .entry(kind)
                    .or_insert(tenchu_assets::stage::imports::Voice {
                        stage: stage.id,
                        character: 0,
                        program: program + 6,
                    });
            }
            let vram = Arc::new(mission.vram);
            let clips = Arc::new(mission.clips);
            for (kind, parts) in mission.models {
                let character = definitions.iter().find(|c| c.kind == kind).unwrap();
                found.entry(character.model.clone()).or_insert_with(|| {
                    (
                        parts,
                        vram.clone(),
                        clips.clone(),
                        mission.common_motions.clone(),
                    )
                });
            }
            textures.push(vram);
        }
        let mut library = Self::default();
        for path in assets.archive.files.keys() {
            let Some(name) = path
                .strip_prefix("K:/WORK/CDIMAGE/HUMAN/")
                .and_then(|s| s.strip_suffix(".MAD"))
            else {
                continue;
            };
            let resource = format!("HUMAN/{name}.MAD");
            let family = match name {
                "RIKIMAUA" => "RIKIMARU",
                "AYAMEA" | "AYAMES" => "AYAME",
                "KOGUMA" => "KUMA",
                "JOCHU2" => "JOCHU",
                _ => name,
            };
            let character = definitions.iter().find(|c| c.model == family).cloned();
            let (parts, mut vram, clips, common) = if let Some(body) = found.get(name) {
                body.clone()
            } else {
                let parts = model::archive(&assets.asset(&resource)?)?;
                if let Some((_, vram, clips, common)) = found.get(family) {
                    (parts, vram.clone(), clips.clone(), common.clone())
                } else {
                    let vram = textures
                        .iter()
                        .max_by_key(|vram| coverage(&parts, vram))
                        .unwrap()
                        .clone();
                    let player = if family == "AYAME" { 1 } else { 0 };
                    let mission = MissionAssets::load(assets, 0, player, 1)?;
                    (parts, vram, Arc::new(mission.clips), mission.common_motions)
                }
            };
            if matches!(family, "RIKIMARU" | "AYAME") {
                let mut own = (*vram).clone();
                own.upload_tpd(&assets.asset(&format!("HUMAN/{family}.TPD"))?)?;
                vram = Arc::new(own);
            }
            tenchu_core::motion::Pose::new(&parts)?;
            let idle = character
                .as_ref()
                .and_then(|c| idle(&c.motions, &common, &clips));
            let id = definitions.iter().find(|c| c.model == name).map_or_else(
                || format!("resource:{resource}"),
                |c| format!("disc:{}", c.kind),
            );
            let think = character
                .as_ref()
                .and_then(|c| thinking.get(&c.kind))
                .copied()
                .unwrap_or(0x4445);
            let voice = character.as_ref().and_then(|c| {
                if c.kind < 2 {
                    Some(tenchu_assets::stage::imports::Voice {
                        stage: 0,
                        character: c.kind,
                        program: 5,
                    })
                } else {
                    voices.get(&c.kind).copied().or_else(|| {
                        definitions
                            .iter()
                            .filter(|d| d.model == c.model)
                            .find_map(|d| voices.get(&d.kind).copied())
                    })
                }
            });
            library.bodies.push(Body {
                id,
                name: label(name),
                resource,
                parts,
                vram,
                character,
                clips,
                idle,
                think,
                voice,
            });
        }
        let mut extra_clips = BTreeMap::new();
        for path in assets
            .archive
            .files
            .keys()
            .filter(|p| p.starts_with("K:/WORK/CDIMAGE/HUMAN/MOTION/") && p.ends_with(".AMD"))
        {
            for clip in tenchu_assets::motion::parse(&assets.asset(path)?)? {
                extra_clips.entry(clip.id).or_insert(clip);
            }
        }
        for body in &mut library.bodies {
            let idle_id = body.idle.map(|i| body.clips[i].id);
            let needed: std::collections::BTreeSet<_> = body
                .character
                .iter()
                .flat_map(|c| c.motions.iter().map(|m| m.clip))
                .chain(idle_id)
                .collect();
            body.clips = Arc::new(
                needed
                    .into_iter()
                    .filter_map(|id| {
                        body.clips
                            .iter()
                            .find(|c| c.id == id)
                            .or_else(|| extra_clips.get(&id))
                            .cloned()
                    })
                    .collect(),
            );
            body.idle = body
                .character
                .as_ref()
                .and_then(|c| idle(&c.motions, &[], &body.clips))
                .or_else(|| idle_id.and_then(|id| body.clips.iter().position(|c| c.id == id)));
        }
        library.bodies.sort_by(|a, b| a.name.cmp(&b.name));
        for (index, body) in library.bodies.iter().enumerate() {
            library.aliases.insert(body.id.clone(), index);
            library
                .aliases
                .insert(format!("resource:{}", body.resource), index);
            for character in &definitions {
                if body.resource == format!("HUMAN/{}.MAD", character.model) {
                    library
                        .aliases
                        .insert(format!("disc:{}", character.kind), index);
                }
            }
        }
        Ok(library)
    }
}

fn idle(
    registry: &[MotionRegistration],
    common: &[MotionRegistration],
    clips: &[Clip],
) -> Option<usize> {
    registry
        .iter()
        .chain(common)
        .filter(|r| r.action == tenchu_core::ids::MOT_NORMAL)
        .find_map(|r| clips.iter().position(|c| c.id == r.clip))
}

fn coverage(parts: &[Part], vram: &Vram) -> usize {
    parts
        .iter()
        .flat_map(|p| &p.geometry)
        .flat_map(|g| &g.faces)
        .filter_map(|f| catalog::face_region(vram, f))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|r| r.pixels(vram).chunks_exact(4).filter(|p| p[3] != 0).count())
        .sum()
}

fn label(name: &str) -> String {
    let friendly = match name {
        "NINJA" => "Onikage",
        "RIKIMARU" => "Rikimaru",
        "RIKIMAUA" => "Rikimaru (armour)",
        "AYAME" => "Ayame",
        "AYAMEA" => "Ayame (armour)",
        "AYAMES" => "Ayame (alternate)",
        "CAT" => "Cat",
        "DOG" => "Dog",
        "RAT" => "Rat",
        "KARASU" => "Crow",
        "KUMA" => "Bear",
        "KOGUMA" => "Bear cub",
        "WOLF" => "Wolf",
        "FIREDOG" => "Fire dog",
        "NINKEN" => "Ninja dog",
        "MEIOU" => "Mei-Oh",
        "ECHIGOYA" => "Echigoya",
        "KATAOKA" => "Kataoka",
        _ => return name.into(),
    };
    format!("{friendly} · {name}")
}

impl Body {
    pub fn sources(&self) -> Vec<catalog::ModelSource<'_>> {
        self.parts
            .iter()
            .enumerate()
            .map(|(i, part)| catalog::ModelSource {
                id: self.character.as_ref().map_or_else(
                    || format!("{}/{i}", self.resource),
                    |c| format!("actors/{:04x}/{i}", c.kind),
                ),
                label: self.name.clone(),
                geometry: &part.geometry,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        settings::Graphics,
        visuals::{appearance, models::Models, pack::Pack},
    };

    #[test]
    #[ignore = "requires TENCHU_DISC; checks all-resource swaps, texture isolation, aliases and both player rigs"]
    fn all_disc_bodies_render_in_both_player_rigs_without_mutating_gameplay() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let start = std::time::Instant::now();
        let library = Library::load(&assets).unwrap();
        eprintln!(
            "{} bodies loaded in {:?}",
            library.bodies.len(),
            start.elapsed()
        );
        assert_eq!(
            library.bodies.len(),
            assets
                .archive
                .files
                .keys()
                .filter(|p| p.starts_with("K:/WORK/CDIMAGE/HUMAN/") && p.ends_with(".MAD"))
                .count()
        );
        assert!(std::ptr::eq(
            library.get("disc:135").unwrap(),
            library.get("disc:138").unwrap()
        ));
        assert!(library.get("disc:161").unwrap().parts.len() < 15);
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let state = crate::session::tests::state(&game);
            for body in &library.bodies {
                let models = Models::default()
                    .with_player(
                        &game,
                        &body.id,
                        &Pack::default(),
                        &Pack::default(),
                        &Graphics::default(),
                        Some(&library),
                    )
                    .unwrap();
                let appearance = models.player.as_ref().unwrap();
                assert!(
                    appearance
                        .meshes
                        .materials()
                        .iter()
                        .all(|m| m.region.is_none()),
                    "foreign VRAM must not alias active mission slots"
                );
                let mut count = 0;
                appearance::visit_parts(&game, true, Some(&models), None, |_, geometry, matrix| {
                    if appearance
                        .parts
                        .iter()
                        .any(|p| std::ptr::eq(p.geometry.as_slice(), geometry))
                    {
                        count += 1;
                        assert!(matrix.translation.iter().all(|n| n.abs() < 1_000_000));
                    }
                });
                assert_eq!(
                    count,
                    body.parts.len(),
                    "{} on player {character}",
                    body.name
                );
                assert!(
                    !crate::render_mesh::characters_presented(&game, Some(&models), true, None)
                        .is_empty()
                );
            }
            assert_eq!(state, crate::session::tests::state(&game));
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; exercises actor movesets through player input, fallback actions and reversible weapon setup"]
    fn native_movesets_keep_campaign_identity_and_reuse_weapon_nodes() {
        use tenchu_core::{ids::*, input::*};
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let library = Library::load(&assets).unwrap();
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let original = game.player().definition;
            let inventory = game.player().inventory;
            let random = game.random;
            let tick = game.tick;
            let onikage = library.get("disc:135").unwrap();
            let source = onikage.character.as_ref().unwrap();
            game.set_player_moveset(Some(source), &onikage.clips, Default::default())
                .unwrap();
            assert_eq!(game.player().kind, character);
            assert_eq!(game.player().inventory, inventory);
            assert_eq!(game.tick, tick);
            assert_eq!(game.random, random);
            assert_ne!(game.player().definition, original);
            assert_eq!(
                game.mission.characters[game.player().definition].weapon,
                source.weapon
            );
            let registry = &game.mission.characters[game.player().definition].motions;
            for action in [MOT_NORMAL, MOT_CHASE, MOT_ATTACK, MOT_ATTACK_SLASH2] {
                assert_eq!(
                    registry.iter().find(|r| r.action == action).unwrap().clip,
                    source
                        .motions
                        .iter()
                        .find(|r| r.action == action)
                        .unwrap()
                        .clip
                );
            }
            assert!(registry.iter().any(|r| r.action == MOT_KAGI));
            let nodes = game.scene.models.len();
            let clips = game.mission.clips.len();
            let definitions = game.mission.characters.len();
            for _ in 0..20 {
                game.set_player_moveset(None, &[], Default::default())
                    .unwrap();
                assert_eq!(game.player().definition, original);
                game.set_player_moveset(Some(source), &onikage.clips, Default::default())
                    .unwrap();
            }
            assert_eq!(game.scene.models.len(), nodes);
            assert_eq!(game.mission.clips.len(), clips);
            assert_eq!(game.mission.characters.len(), definitions);
            game.set_player_moveset(None, &[], Default::default())
                .unwrap();
            for body in &library.bodies {
                let Some(source) = &body.character else {
                    continue;
                };
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                game.set_player_moveset(Some(source), &body.clips, Default::default())
                    .unwrap();
                let mut attack = false;
                for tick in 0..240 {
                    let input = match tick {
                        1..=20 => UP,
                        30 | 65 | 100 | 115 | 130 => SQUARE,
                        160 => CROSS,
                        180..=200 => R1,
                        _ => 0,
                    };
                    game.step(input).unwrap_or_else(|e| {
                        panic!("{} on {character}, tick {tick}: {e}", body.name)
                    });
                    attack |= game.player().status == 7;
                }
                if source.kind == 0x87 {
                    assert!(attack, "Onikage attacks must respond to player input");
                }
                assert_eq!(game.player().kind, character);
                assert!(game.player().health > 0);
                game.set_player_moveset(None, &[], Default::default())
                    .unwrap();
                assert_eq!(game.player().definition, original);
                for _ in 0..30 {
                    game.step(UP).unwrap();
                }
            }
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; Onikage keeps his attacks while both heroes regain movement, cancel taunts and restore options"]
    fn hybrid_movesets_restore_player_locomotion_and_cancel_idle_taunts() {
        use tenchu_core::{
            game::moveset::Options,
            ids::*,
            input::*,
            motion::{Change, Motion},
        };
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        let library = Library::load(&assets).unwrap();
        let onikage = library.get("disc:135").unwrap();
        let source = onikage.character.as_ref().unwrap();
        let motion = |game: &tenchu_core::game::Game, action| {
            let mut motion = Motion::new(&game.mission.models[&game.player().mesh_kind]).unwrap();
            let definition = &game.mission.characters[game.player().definition];
            if motion.change(
                action,
                &definition.motions,
                &game.mission.common_motions,
                &game.mission.clips,
            ) == Change::Missing
            {
                return None;
            }
            let clip = &game.mission.clips[motion.clip.unwrap()];
            Some((clip.id, clip.forward_speed, clip.side_speed, clip.duration))
        };
        for character in 0..2 {
            let load = || {
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                game.intelligence.ignore_player = true;
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                game
            };
            let mut original = load();
            let mut game = load();
            let definition = game.player().definition;
            let health = game.player().health;
            let inventory = game.player().inventory;
            game.set_player_moveset(Some(source), &onikage.clips, Default::default())
                .unwrap();
            assert_ne!(motion(&game, MOT_CHASE), motion(&original, MOT_CHASE));
            let options = Options {
                player_movement: true,
                no_idle_taunts: false,
            };
            game.set_player_moveset(Some(source), &onikage.clips, options)
                .unwrap();
            for action in [
                MOT_MOVE,
                MOT_MOVE_BACK,
                MOT_MOVE_DASH_FWD,
                MOT_CHASE,
                MOT_CHASE_BACK,
                MOT_CHASE_DASH_FWD,
                MOT_JUMP,
                MOT_JUMP_FORWARD,
                MOT_SQUAT,
                MOT_SQUAT_WALK_F,
                MOT_SQUAT_ROLL_R,
                MOT_HANG,
                MOT_HANG_SHIMMY_LEFT,
                MOT_STICKON_SLIDE_L,
                MOT_SWIM,
                MOT_KAGI_PULL,
                MOT_STATE_CLIMB,
                MOT_STATE_FALL,
                MOT_STATE_LAND,
            ] {
                assert_eq!(
                    motion(&game, action),
                    motion(&original, action),
                    "action {action:#x}, player {character}"
                );
            }
            assert_eq!(
                game.mission.characters[game.player().definition].weapon,
                source.weapon
            );
            for action in [MOT_ATTACK, MOT_ATTACK_SLASH2] {
                assert_eq!(
                    motion(&game, action).unwrap().0,
                    source
                        .motions
                        .iter()
                        .find(|r| r.action == action)
                        .unwrap()
                        .clip
                );
            }
            for _ in 0..30 {
                game.step(UP).unwrap();
                original.step(UP).unwrap();
                assert_eq!(
                    game.player().position,
                    original.player().position,
                    "movement on player {character}"
                );
                assert_eq!(game.player().velocity, original.player().velocity);
            }
            let state = crate::session::tests::state(&game);
            game.set_player_moveset(Some(source), &onikage.clips, options)
                .unwrap();
            assert_eq!(
                crate::session::tests::state(&game),
                state,
                "unchanged options must not interrupt movement"
            );

            let registry = &game.mission.characters[game.player().definition].motions;
            let actor = game.roster.get_mut(game.player_id);
            assert_ne!(
                actor.motion.change(
                    MOT_ATTACK_TAUNT,
                    registry,
                    &game.mission.common_motions,
                    &game.mission.clips
                ),
                Change::Missing
            );
            actor.status = 7;
            actor.motion.count = 5;
            let options = Options {
                no_idle_taunts: true,
                ..options
            };
            game.set_player_moveset(Some(source), &onikage.clips, options)
                .unwrap();
            assert!(matches!(
                game.player().motion.action,
                MOT_NORMAL | MOT_ENGAGE_STANCE
            ));
            assert_eq!(game.moveset.options, options);
            for _ in 0..1800 {
                game.step(0).unwrap();
                assert!(!matches!(
                    game.player().motion.action,
                    MOT_ATTACK_TAUNT | MOT_ACTION_FIDGET_A | MOT_ACTION_FIDGET_B
                ));
            }
            game.step(UP).unwrap();
            assert_eq!(game.player().motion.action, MOT_CHASE);
            assert_ne!(game.player().velocity, [0; 3]);

            let nodes = game.scene.models.len();
            let clips = game.mission.clips.len();
            for _ in 0..4 {
                game.set_player_moveset(Some(source), &onikage.clips, Default::default())
                    .unwrap();
                assert_eq!(
                    motion(&game, MOT_CHASE).unwrap().0,
                    source
                        .motions
                        .iter()
                        .find(|r| r.action == MOT_CHASE)
                        .unwrap()
                        .clip
                );
                game.set_player_moveset(Some(source), &onikage.clips, options)
                    .unwrap();
            }
            assert_eq!(game.scene.models.len(), nodes);
            assert_eq!(game.mission.clips.len(), clips);
            game.set_player_moveset(None, &[], options).unwrap();
            assert_eq!(game.player().definition, definition);
            assert_eq!(game.moveset.options, Options::default());
            assert_eq!(game.player().kind, character);
            assert_eq!(game.player().health, health);
            assert_eq!(game.player().inventory, inventory);
        }
    }
}
