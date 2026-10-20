use super::{catalog, pack::Pack};
use crate::{
    lighting::{Light, Source},
    render_mesh,
};
use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::{Result, model::Geometry};
use tenchu_core::{game::Game, math::Matrix};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rule {
    pub name_contains: String,
    pub model: Option<String>,
    pub enabled: bool,
    pub threshold: f32,
    pub emission: f32,
    pub radius_scale: f32,
    pub color: Option<[f32; 3]>,
    pub offset: [f32; 3],
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            name_contains: String::new(),
            model: None,
            enabled: true,
            threshold: 0.65,
            emission: 2.0,
            radius_scale: 1.0,
            color: None,
            offset: [0.0; 3],
        }
    }
}

impl Rule {
    pub fn validate(&self) -> Result<()> {
        if (self.name_contains.is_empty() && self.model.is_none())
            || !self.threshold.is_finite()
            || !(0.05..=1.0).contains(&self.threshold)
            || !self.emission.is_finite()
            || !(0.0..=10.0).contains(&self.emission)
            || !self.radius_scale.is_finite()
            || !(0.1..=4.0).contains(&self.radius_scale)
            || self
                .offset
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 100_000.0)
            || self
                .color
                .is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=4.0).contains(v)))
        {
            return Err("Invalid light rule selector, brightness, radius, color or offset".into());
        }
        Ok(())
    }
}

pub fn defaults() -> Vec<Rule> {
    serde_json::from_str(include_str!("../../data/lights.json")).unwrap()
}

struct Emitter {
    name: String,
    position: Vec3,
    color: [f32; 3],
    radius_scale: f32,
}

#[derive(Default)]
pub struct Emitters {
    models: BTreeMap<usize, Emitter>,
    placed: Vec<Source>,
    pub emissions: BTreeMap<catalog::Region, [f32; 4]>,
}

impl Emitters {
    pub fn build(game: &Game, pack: &Pack, artwork: bool) -> Self {
        let rules = artwork
            .then(|| pack.lights.clone())
            .flatten()
            .unwrap_or_else(defaults);
        let mut out = Self::default();
        for model in catalog::models(&game.mission) {
            let Some(rule) = rules.iter().rev().find(|r| {
                r.model.as_ref().map_or_else(
                    || {
                        model
                            .label
                            .to_ascii_uppercase()
                            .contains(&r.name_contains.to_ascii_uppercase())
                    },
                    |id| id == &model.id,
                )
            }) else {
                continue;
            };
            if !rule.enabled || model.geometry.is_empty() {
                continue;
            }
            if let Some(emitter) = detect(&model, rule, &game.mission.vram, &mut out.emissions) {
                out.models.insert(model.geometry.as_ptr() as usize, emitter);
            }
        }
        render_mesh::visit_stage(&game.mission, &game.math, |geometry, matrix| {
            if let Some(source) = out.source(geometry, matrix) {
                out.placed.push(source);
            }
        });
        out
    }

    fn source(&self, geometry: &[Geometry], matrix: Matrix) -> Option<Source> {
        if geometry.is_empty() {
            return None;
        }
        let emitter = self.models.get(&(geometry.as_ptr() as usize))?;
        let rotation = Mat3::from_cols_array(&std::array::from_fn(|i| {
            f32::from(matrix.rotation[i % 3][i / 3]) / 4096.0
        }));
        Some(Source {
            name: emitter.name.clone(),
            light: Light {
                position: rotation * emitter.position
                    + Vec3::from(matrix.translation.map(|v| v as f32)),
                color: emitter.color,
                radius: 12_000.0 * emitter.radius_scale,
            },
        })
    }

    pub fn sources(&self, game: &Game) -> Vec<Source> {
        let mut out = self.placed.clone();
        render_mesh::visit_inspection_characters(game, |geometry, matrix| {
            if let Some(source) = self.source(geometry, matrix) {
                out.push(source);
            }
        });
        render_mesh::visit_hinged(game, |geometry, matrix| {
            if let Some(source) = self.source(geometry, matrix) {
                out.push(source);
            }
        });
        out
    }
}

fn detect(
    model: &catalog::ModelSource<'_>,
    rule: &Rule,
    vram: &tenchu_assets::texture::Vram,
    emissions: &mut BTreeMap<catalog::Region, [f32; 4]>,
) -> Option<Emitter> {
    let mut position = Vec3::ZERO;
    let mut color = Vec3::ZERO;
    let mut weight = 0.0;
    for geometry in model.geometry {
        for face in render_mesh::draw_tmd_faces(geometry) {
            let Some(region) = catalog::face_region(vram, face) else {
                continue;
            };
            let texture = face.texture.unwrap();
            let order = if face.corners.len() == 4 {
                &[0, 1, 2, 1, 3, 2][..]
            } else {
                &[0, 1, 2][..]
            };
            let mut luminous = false;
            for triangle in order.chunks_exact(3) {
                let corners = [triangle[0], triangle[1], triangle[2]].map(|i| face.corners[i]);
                let points = corners
                    .map(|c| Vec3::from(geometry.vertices[c.vertex as usize].map(f32::from)));
                let area = (points[1] - points[0])
                    .cross(points[2] - points[0])
                    .length()
                    * 0.5;
                if area == 0.0 {
                    continue;
                }
                for a in 0..8 {
                    for b in 0..8 - a {
                        let weights = [(a as f32 + 0.333) / 8.0, (b as f32 + 0.333) / 8.0];
                        let weights = [weights[0], weights[1], 1.0 - weights[0] - weights[1]];
                        let uv = std::array::from_fn(|axis| {
                            (0..3)
                                .map(|i| f32::from(corners[i].uv[axis]) * weights[i])
                                .sum::<f32>()
                                .round() as u16
                        });
                        let pixel = vram.texel(texture.page, texture.palette, uv);
                        if pixel == 0 {
                            continue;
                        }
                        let rgb = Vec3::from(
                            [0, 5, 10].map(|shift| f32::from((pixel >> shift) & 31) / 31.0),
                        );
                        let brightness = rgb.dot(Vec3::new(0.2126, 0.7152, 0.0722));
                        if brightness < rule.threshold {
                            continue;
                        }
                        let w = area * brightness;
                        position += (points[0] * weights[0]
                            + points[1] * weights[1]
                            + points[2] * weights[2])
                            * w;
                        color += rgb * w;
                        weight += w;
                        luminous = true;
                    }
                }
            }
            if luminous {
                emissions.insert(
                    region,
                    [rule.emission, rule.emission, rule.emission, rule.threshold],
                );
            }
        }
    }
    (weight > 0.0).then(|| Emitter {
        name: model.label.clone(),
        position: position / weight + Vec3::from(rule.offset),
        color: rule
            .color
            .unwrap_or_else(|| (color / color.max_element().max(0.0001)).to_array()),
        radius_scale: rule.radius_scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; named disc props and carried lanterns become presentation-only emitters"]
    fn disc_lanterns_follow_named_meshes_without_advancing_simulation() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut carried = false;
        for mission in 1..=10 {
            let mut game = crate::session::Options {
                mission,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            for _ in 0..40 {
                game.step(0).unwrap();
            }
            let before = crate::session::tests::state(&game);
            let emitters = Emitters::build(&game, &Pack::default(), true);
            let sources = emitters.sources(&game);
            let names: std::collections::BTreeSet<_> =
                emitters.models.values().map(|s| s.name.as_str()).collect();
            eprintln!(
                "Mission {mission}: {} placed, {} live, {names:?}",
                emitters.placed.len(),
                sources.len()
            );
            for source in &sources {
                assert!(source.light.position.is_finite());
                assert!(source.light.color.iter().all(|c| (0.0..=1.0).contains(c)));
            }
            if mission == 1 {
                assert!(sources.len() > 10);
                assert!(names.iter().any(|name| name.contains("AKARI")));
                let player = Vec3::from(game.player().position.map(|v| v as f32));
                let nearest = sources
                    .iter()
                    .min_by(|a, b| {
                        player
                            .distance_squared(a.light.position)
                            .total_cmp(&player.distance_squared(b.light.position))
                    })
                    .unwrap();
                eprintln!(
                    "Nearest {} {:?}, player {:?}",
                    nearest.name, nearest.light.position, player
                );
                let empty = Pack {
                    lights: Some(Vec::new()),
                    ..Default::default()
                };
                assert!(
                    Emitters::build(&game, &empty, true)
                        .sources(&game)
                        .is_empty()
                );
                assert_eq!(
                    Emitters::build(&game, &empty, false).sources(&game).len(),
                    sources.len()
                );
            }
            if !carried
                && let Some((&kind, _)) = game.mission.weapons.names.iter().find(|(kind, name)| {
                    name.as_str() == "ANDON"
                        && emitters
                            .models
                            .contains_key(&(game.mission.weapons.models[kind].as_ptr() as usize))
                })
            {
                let original = game.player().clone();
                game.roster.get_mut(game.player_id).weapons[0]
                    .as_mut()
                    .unwrap()
                    .kind = kind;
                let attached = emitters
                    .sources(&game)
                    .into_iter()
                    .find(|s| s.name == "ANDON")
                    .unwrap();
                game.roster.get_mut(game.player_id).position[0] += 500;
                let moved = emitters
                    .sources(&game)
                    .into_iter()
                    .find(|s| s.name == "ANDON")
                    .unwrap();
                assert!(
                    (moved.light.position - attached.light.position - Vec3::X * 500.0).length()
                        < 0.01
                );
                *game.roster.get_mut(game.player_id) = original;
                carried = true;
            }
            assert_eq!(before, crate::session::tests::state(&game));
        }
        assert!(carried, "Expected a carried lantern model in the campaign");
    }
}
