use super::{
    catalog::ModelSource,
    interpolation::{Frame, Part},
    models::{Meshes, rotation},
    pack::Pack,
};
use crate::{render_mesh, settings::Graphics};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap};
#[cfg(not(target_arch = "wasm32"))]
use tenchu_assets::stage::MissionAssets;
use tenchu_assets::{Result, model};
use tenchu_core::{game::Game, math::Matrix};

pub mod library;
pub mod picker;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Body {
    pub name: String,
    pub parts: Vec<BodyPart>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BodyPart {
    pub file: Option<String>,
    pub parent: Option<usize>,
    pub position: [i16; 3],
}

pub fn parents(parts: &[model::Part]) -> Vec<Option<usize>> {
    parts
        .iter()
        .map(|p| {
            (p.parent >= 0)
                .then(|| parts.iter().position(|v| v.child == p.parent))
                .flatten()
        })
        .collect()
}

impl Body {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.name.len() > 100 || self.parts.len() != 15 {
            return Err("A player body needs a name and fifteen rig parts".into());
        }
        for (index, part) in self.parts.iter().enumerate() {
            let mut parent = part.parent;
            for depth in 0..=self.parts.len() {
                let Some(id) = parent else { break };
                if id >= self.parts.len() || id == index || depth == self.parts.len() {
                    return Err(format!("Invalid parent hierarchy in {}", self.name));
                }
                parent = self.parts[id].parent;
            }
        }
        Ok(())
    }
}

pub fn choices(
    game: &Game,
    pack: &Pack,
    library: Option<&library::Library>,
) -> Vec<(String, String)> {
    let rig = parents(&game.mission.models[&game.player().kind]);
    let mut choices = vec![(String::new(), "Original (follow equipment/disguise)".into())];
    if let Some(library) = library {
        choices.extend(
            library
                .bodies
                .iter()
                .map(|b| (b.id.clone(), b.name.clone())),
        );
    } else {
        for &kind in game.mission.models.keys() {
            let name = &game
                .mission
                .characters
                .iter()
                .find(|c| c.kind == kind)
                .unwrap()
                .model;
            choices.push((format!("disc:{kind}"), format!("{name} ({kind:04x})")));
        }
    }
    for (id, body) in &pack.player_models {
        if body.parts.iter().map(|p| p.parent).eq(rig.iter().copied()) {
            choices.push((format!("pack:{id}"), format!("{} — folder", body.name)));
        }
    }
    choices
}

pub(super) struct Appearance {
    parts: Vec<model::Part>,
    parents: Vec<Option<usize>>,
    pub meshes: Meshes,
}

fn rendered_player(game: &Game, inspection: bool) -> Cow<'_, tenchu_core::actor::Actor> {
    if inspection || game.drawing.step != tenchu_core::drawing::Step::Normal {
        let mut player = game.player().clone();
        player.initialize_rendered(&game.scene, &game.math);
        Cow::Owned(player)
    } else {
        Cow::Borrowed(game.player())
    }
}

impl Appearance {
    pub fn build(
        game: &Game,
        selection: &str,
        pack: &Pack,
        bodies: &Pack,
        graphics: &Graphics,
        library: Option<&library::Library>,
    ) -> Result<Option<Self>> {
        if selection.is_empty() {
            return Ok(None);
        }
        let body = library.and_then(|l| l.get(selection));
        let own_pack = body
            .filter(|_| graphics.asset_pack)
            .map(|body| Pack::for_models(&body.vram, &body.sources()))
            .transpose()?;
        let vram = body.map_or(&game.mission.vram, |b| &b.vram);
        let (parts, ids, pack, custom) = if let Some(body) = body {
            (
                body.parts.clone(),
                body.sources().into_iter().map(|s| s.id).collect(),
                own_pack.as_ref().unwrap_or(pack),
                false,
            )
        } else if let Some(kind) = selection.strip_prefix("disc:") {
            let kind: i16 = kind.parse().map_err(|_| "Invalid disc body")?;
            let parts = game
                .mission
                .models
                .get(&kind)
                .ok_or("Body is unavailable in this mission; choose Original or a model folder")?
                .clone();
            let ids = (0..parts.len())
                .map(|i| format!("actors/{kind:04x}/{i}"))
                .collect::<Vec<_>>();
            (parts, ids, pack, false)
        } else if let Some(id) = selection.strip_prefix("pack:") {
            let body = bodies
                .player_models
                .get(id)
                .ok_or("Selected body is missing from the model folder")?;
            let parts = body
                .parts
                .iter()
                .enumerate()
                .map(|(i, p)| model::Part {
                    parent: p.parent.map_or(-1, |v| v as i16),
                    child: i as i16,
                    position: p.position,
                    geometry: vec![model::Geometry {
                        vertices: Vec::new(),
                        normals: Vec::new(),
                        faces: Vec::new(),
                    }],
                })
                .collect::<Vec<_>>();
            let ids = (0..parts.len())
                .map(|i| format!("player/{id}/{i}"))
                .collect();
            (parts, ids, bodies, true)
        } else {
            return Err("Unknown player model selection".into());
        };
        let parent = parents(&parts);
        tenchu_core::motion::Pose::new(&parts)?;
        let sources = parts
            .iter()
            .zip(ids)
            .map(|(p, id)| ModelSource {
                id,
                label: String::new(),
                geometry: &p.geometry,
            })
            .collect();
        let mut options = graphics.clone();
        if custom {
            options.asset_pack = true;
        }
        let mut meshes = Meshes::build(vram, pack, &options, sources, true)?;
        if body.is_some() {
            meshes.detach_textures();
            if graphics.asset_pack {
                meshes.replace_textures(pack);
            }
        }
        Ok(Some(Self {
            parts,
            parents: parent,
            meshes,
        }))
    }

    fn offsets(
        &self,
        game: &Game,
        inspection: bool,
        frame: Option<&Frame>,
    ) -> BTreeMap<usize, Vec3> {
        let player = rendered_player(game, inspection);
        let original = &game.mission.models[&player.rendered_mesh_kind];
        let matrices: Vec<_> = player
            .rendered
            .iter()
            .enumerate()
            .map(|(i, &matrix)| {
                Frame::transform(
                    frame,
                    Part::Actor {
                        root: player.root_model,
                        model: player.parts[i],
                        kind: player.rendered_mesh_kind,
                        inspection,
                    },
                    matrix,
                )
            })
            .collect();
        let mut offsets = BTreeMap::new();
        for _ in 0..self.parts.len() {
            for (i, parent) in self.parents.iter().copied().enumerate() {
                if offsets.contains_key(&player.parts[i]) {
                    continue;
                }
                let (base, orientation) = if let Some(p) = parent {
                    let Some(&offset) = offsets.get(&player.parts[p]) else {
                        continue;
                    };
                    (offset, rotation(matrices[p]))
                } else {
                    let local = game.scene.models[player.parts[i]].local;
                    (
                        Vec3::ZERO,
                        rotation(matrices[i]) * rotation(local).inverse(),
                    )
                };
                let mut delta = Vec3::from(self.parts[i].position.map(f32::from))
                    - Vec3::from(original[i].position.map(f32::from));
                if parent.is_none() && original[i].position[1] != 0 {
                    delta.x = 0.0;
                    delta.z = 0.0;
                    delta.y *= game.scene.models[player.parts[i]].local.translation[1] as f32
                        / f32::from(original[i].position[1]);
                }
                offsets.insert(player.parts[i], base + orientation * delta);
            }
        }
        for weapon in player.weapons.iter().flatten() {
            if let Some(parent) = game.scene.models[weapon.model].parent
                && let Some(&offset) = offsets.get(&parent)
            {
                offsets.insert(weapon.model, offset);
            }
        }
        offsets
    }

    fn transforms(&self, game: &Game, inspection: bool, frame: Option<&Frame>) -> Vec<Matrix> {
        let player = rendered_player(game, inspection);
        let source = &game.mission.models[&player.rendered_mesh_kind];
        let source_parents = parents(source);
        let matrices: Vec<_> = player
            .rendered
            .iter()
            .enumerate()
            .map(|(i, &matrix)| {
                Frame::transform(
                    frame,
                    Part::Actor {
                        root: player.root_model,
                        model: player.parts[i],
                        kind: player.rendered_mesh_kind,
                        inspection,
                    },
                    matrix,
                )
            })
            .collect();
        let root =
            rotation(matrices[0]) * rotation(game.scene.models[player.parts[0]].local).inverse();
        let origin = Vec3::from(matrices[0].translation.map(|n| n as f32).to_array())
            - root
                * Vec3::from(
                    game.scene.models[player.parts[0]]
                        .local
                        .translation
                        .map(|n| n as f32)
                        .to_array(),
                );
        let mut result = vec![None; self.parts.len()];
        for _ in 0..self.parts.len() {
            for (i, part) in self.parts.iter().enumerate() {
                if result[i].is_some() {
                    continue;
                }
                let (parent_rotation, position) = if let Some(parent) = self.parents[i] {
                    let Some((r, p)) = result[parent] else {
                        continue;
                    };
                    (r, p)
                } else {
                    (root, origin)
                };
                let local = matrices.get(i).map_or(glam::Mat3::IDENTITY, |&matrix| {
                    source_parents[i]
                        .map_or(root, |p| rotation(matrices[p]))
                        .inverse()
                        * rotation(matrix)
                });
                let mut offset = Vec3::from(part.position.map(f32::from));
                if i == 0 && source[0].position[1] != 0 {
                    let translation = game.scene.models[player.parts[0]].local.translation;
                    offset.x = translation.x as f32;
                    offset.y *= translation.y as f32 / f32::from(source[0].position[1]);
                    offset.z = translation.z as f32;
                }
                result[i] = Some((parent_rotation * local, position + parent_rotation * offset));
            }
        }
        result
            .into_iter()
            .map(|value| {
                let (rotation, position) = value.expect("validated model hierarchy");
                Matrix {
                    rotation: std::array::from_fn(|r| {
                        std::array::from_fn(|c| (rotation.col(c)[r] * 4096.0).round() as i16)
                    }),
                    translation: position.to_array().map(|n| n.round() as i32).into(),
                }
            })
            .collect()
    }
}

pub fn visit_parts(
    game: &Game,
    inspection: bool,
    models: Option<&super::models::Models>,
    frame: Option<&Frame>,
    mut visit: impl FnMut(Part, &[model::Geometry], Matrix),
) {
    let appearance = models.and_then(|m| m.player.as_ref());
    let compatible = appearance.is_none_or(|a| {
        a.parents == parents(&game.mission.models[&game.player().rendered_mesh_kind])
    });
    let offsets = appearance
        .filter(|_| compatible)
        .map(|a| a.offsets(game, inspection, frame));
    let transforms = appearance
        .filter(|_| !compatible)
        .map(|a| a.transforms(game, inspection, frame));
    let posed_player = transforms
        .as_ref()
        .map(|_| rendered_player(game, inspection));
    render_mesh::visit_character_parts(game, inspection, |key, geometry, matrix| {
        let mut matrix = Frame::transform(frame, key, matrix);
        if let Part::Actor { root, model, .. } = key
            && root == game.player().root_model
            && let Some(appearance) = appearance
        {
            if let Some(offset) = offsets.as_ref().and_then(|o| o.get(&model)) {
                for (value, offset) in matrix.translation.iter_mut().zip(offset.to_array()) {
                    *value += offset.round() as i32;
                }
            }
            if let Some(index) = game.player().parts.iter().position(|&id| id == model) {
                if let Some(part) = appearance.parts.get(index) {
                    visit(
                        key,
                        &part.geometry,
                        transforms.as_ref().map_or(matrix, |t| t[index]),
                    );
                }
                return;
            }
            if let Some(transforms) = &transforms {
                let player = posed_player.as_deref().unwrap();
                let mut parent = game.scene.models[model].parent;
                while let Some(id) = parent {
                    if let Some(index) = player.parts.iter().position(|&p| p == id)
                        && let Some(target) = transforms.get(index)
                    {
                        let source = Frame::transform(
                            frame,
                            Part::Actor {
                                root,
                                model: id,
                                kind: player.rendered_mesh_kind,
                                inspection,
                            },
                            player.rendered[index],
                        );
                        let relative = rotation(*target) * rotation(source).inverse();
                        let position = Vec3::from(target.translation.map(|n| n as f32).to_array())
                            + relative
                                * (Vec3::from(matrix.translation.map(|n| n as f32).to_array())
                                    - Vec3::from(source.translation.map(|n| n as f32).to_array()));
                        let r = relative * rotation(matrix);
                        matrix = Matrix {
                            rotation: std::array::from_fn(|row| {
                                std::array::from_fn(|col| (r.col(col)[row] * 4096.0).round() as i16)
                            }),
                            translation: position.to_array().map(|n| n.round() as i32).into(),
                        };
                        break;
                    }
                    parent = game.scene.models[id].parent;
                }
            }
        }
        visit(key, geometry, matrix);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn exported(mission: &MissionAssets) -> BTreeMap<String, Body> {
    mission
        .models
        .iter()
        .filter(|(_, parts)| parts.len() == 15)
        .map(|(&kind, parts)| {
            let name = mission
                .characters
                .iter()
                .find(|c| c.kind == kind)
                .unwrap()
                .model
                .clone();
            let body = Body {
                name,
                parts: parts
                    .iter()
                    .zip(parents(parts))
                    .enumerate()
                    .map(|(i, (p, parent))| BodyPart {
                        file: p
                            .geometry
                            .iter()
                            .any(|g| render_mesh::draw_tmd_faces(g).next().is_some())
                            .then(|| format!("models/actors/{kind:04x}/{i}.obj")),
                        parent,
                        position: p.position,
                    })
                    .collect(),
            };
            (format!("{kind:04x}"), body)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visuals::{interpolation::History, models::Models, pack::Manifest};

    #[test]
    #[ignore = "requires TENCHU_DISC; checks body swaps, custom rig offsets and moving weapon attachments without changing game state"]
    fn bodies_keep_weapons_animation_and_gameplay_for_both_players() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let library = library::Library::load(&assets).unwrap();
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            let root = std::env::temp_dir().join(format!(
                "tenchu-player-model-{}-{character}",
                std::process::id()
            ));
            super::super::export(&game.mission, &root).unwrap();
            let mut manifest: Manifest =
                serde_json::from_slice(&std::fs::read(root.join("pack.json")).unwrap()).unwrap();
            let id = format!("{character:04x}");
            let body = manifest.player_models.get_mut(&id).unwrap();
            body.name = "Long sleeves".into();
            body.parts[13].position[0] += 100;
            body.parts[14].position[0] -= 100;
            std::fs::write(
                root.join("pack.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            let bodies = Pack::load(&root).unwrap();
            let mut choices: Vec<_> = choices(&game, &bodies, None)
                .into_iter()
                .filter(|(id, _)| {
                    id.strip_prefix("disc:").is_none_or(|kind| {
                        game.mission.models[&kind.parse::<i16>().unwrap()].len() == 15
                    })
                })
                .collect();
            choices.push(("disc:135".into(), "Onikage".into()));
            assert!(choices.len() > 3);
            eprintln!(
                "character {character}: {} compatible bodies",
                choices.len() - 1
            );
            let pack = Pack::default();
            let graphics = Graphics::default();
            let mut history = History::default();
            history.reset(&game);
            for tick in 0..110 {
                game.step(match tick {
                    35 => tenchu_core::input::SQUARE,
                    55..=70 => tenchu_core::input::UP,
                    75 => tenchu_core::input::CROSS,
                    _ => 0,
                })
                .unwrap();
                history.tick(&game);
                if ![5, 36, 45, 60, 85, 100].contains(&tick) {
                    continue;
                }
                let state = crate::session::tests::state(&game);
                for selection in choices
                    .iter()
                    .map(|(id, _)| id)
                    .filter(|id| id.starts_with("disc:") || **id == format!("pack:{character:04x}"))
                {
                    let models = Models::build(&game.mission, &pack, &graphics)
                        .unwrap()
                        .with_player(&game, selection, &pack, &bodies, &graphics, Some(&library))
                        .unwrap();
                    let appearance = models.player.as_ref().unwrap();
                    for alpha in [0.0, 0.5, 1.0] {
                        let frame = history.sample(game.tick, alpha);
                        for inspection in [false, true] {
                            let offsets = appearance.offsets(&game, inspection, frame.as_ref());
                            let mut presented = Vec::new();
                            visit_parts(
                                &game,
                                inspection,
                                Some(&models),
                                frame.as_ref(),
                                |_, geometry, matrix| {
                                    presented.push((geometry.as_ptr(), matrix));
                                },
                            );
                            render_mesh::visit_character_parts(
                                &game,
                                inspection,
                                |key, geometry, matrix| {
                                    let Part::Actor { root, model, .. } = key else {
                                        unreachable!()
                                    };
                                    let mut expected =
                                        Frame::transform(frame.as_ref(), key, matrix);
                                    if root == game.player().root_model {
                                        if game.player().parts.contains(&model) {
                                            return;
                                        }
                                        let parent = game.scene.models[model].parent.unwrap();
                                        assert_eq!(offsets[&model], offsets[&parent]);
                                        for (v, delta) in expected
                                            .translation
                                            .iter_mut()
                                            .zip(offsets[&model].to_array())
                                        {
                                            *v += delta.round() as i32;
                                        }
                                    }
                                    assert!(
                                        presented.contains(&(geometry.as_ptr(), expected)),
                                        "Weapon or NPC changed unexpectedly"
                                    );
                                },
                            );
                        }
                        assert!(
                            !render_mesh::characters_presented(
                                &game,
                                Some(&models),
                                true,
                                frame.as_ref()
                            )
                            .is_empty()
                        );
                        assert!(
                            !render_mesh::shadow_models_presented(
                                &game,
                                Some(&models),
                                frame.as_ref()
                            )
                            .is_empty()
                        );
                        let _ = crate::render_effects::build_presented(
                            &game,
                            false,
                            None,
                            true,
                            Some(&models),
                            frame.as_ref(),
                            crate::render_effects::Style::default(),
                        );
                    }
                }
                assert_eq!(state, crate::session::tests::state(&game));
            }
            manifest.player_models.get_mut(&id).unwrap().parts[0].parent = Some(0);
            std::fs::write(
                root.join("pack.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            assert!(Pack::load(&root).err().unwrap().contains("hierarchy"));
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
