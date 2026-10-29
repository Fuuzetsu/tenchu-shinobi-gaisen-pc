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
        let mut textures = Vec::new();
        for stage in assets.stages()? {
            let mission = MissionAssets::load(assets, stage.id, 0, 0)?;
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
            library.bodies.push(Body {
                id,
                name: label(name),
                resource,
                parts,
                vram,
                character,
                clips,
                idle,
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
            game.set_player_moveset(Some(source), &onikage.clips)
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
                game.set_player_moveset(None, &[]).unwrap();
                assert_eq!(game.player().definition, original);
                game.set_player_moveset(Some(source), &onikage.clips)
                    .unwrap();
            }
            assert_eq!(game.scene.models.len(), nodes);
            assert_eq!(game.mission.clips.len(), clips);
            assert_eq!(game.mission.characters.len(), definitions);
            game.set_player_moveset(None, &[]).unwrap();
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
                game.set_player_moveset(Some(source), &body.clips).unwrap();
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
                game.set_player_moveset(None, &[]).unwrap();
                assert_eq!(game.player().definition, original);
                for _ in 0..30 {
                    game.step(UP).unwrap();
                }
            }
        }
    }
}
