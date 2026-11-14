use super::{catalog, pack::Pack};
use crate::{
    lighting::{Light, Source},
    render_mesh,
};
use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::{Result, model::Geometry};
use tenchu_core::math::Matrix;

mod gaisen;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rule {
    pub name_contains: String,
    pub model: Option<String>,
    pub texture: Option<String>,
    pub label: Option<String>,
    pub enabled: bool,
    pub threshold: f32,
    pub emission: f32,
    pub emit_caps: bool,
    pub flame: bool,
    pub radius_scale: f32,
    pub color: Option<[f32; 3]>,
    pub offset: [f32; 3],
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            name_contains: String::new(),
            model: None,
            texture: None,
            label: None,
            enabled: true,
            threshold: 0.65,
            emission: 1.0,
            emit_caps: false,
            flame: true,
            radius_scale: 1.0,
            color: None,
            offset: [0.0; 3],
        }
    }
}

impl Rule {
    pub fn emits_surface(&self, points: [Vec3; 3]) -> bool {
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        self.emit_caps || normal.y.abs() < 0.5
    }

    pub(crate) fn matches(
        &self,
        model: &catalog::ModelSource<'_>,
        textures: &catalog::Textures,
    ) -> bool {
        self.model.as_ref().map_or_else(
            || {
                model
                    .label
                    .to_ascii_uppercase()
                    .contains(&self.name_contains.to_ascii_uppercase())
            },
            |id| id == &model.id,
        ) && self
            .texture
            .as_ref()
            .is_none_or(|id| textures.contains(model, id))
    }

    pub fn validate(&self) -> Result<()> {
        if (self.name_contains.is_empty() && self.model.is_none() && self.texture.is_none())
            || self
                .texture
                .as_ref()
                .is_some_and(|id| id.len() != 32 || !id.bytes().all(|c| c.is_ascii_hexdigit()))
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

pub fn rules(pack: &Pack, artwork: bool) -> Vec<Rule> {
    artwork
        .then(|| pack.lights.clone())
        .flatten()
        .unwrap_or_else(defaults)
}

struct Emitter {
    flame: bool,
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
    pub material_emissions: BTreeMap<String, [f32; 4]>,
}

impl Emitters {
    pub fn new<'a>(
        vram: &tenchu_assets::texture::Vram,
        models: impl IntoIterator<Item = catalog::ModelSource<'a>>,
        rules: &[Rule],
    ) -> Self {
        let models: Vec<_> = models.into_iter().collect();
        let textures = catalog::Textures::new(vram, &models);
        Self::with_textures(vram, models, rules, &textures)
    }

    fn with_textures<'a>(
        vram: &tenchu_assets::texture::Vram,
        models: impl IntoIterator<Item = catalog::ModelSource<'a>>,
        rules: &[Rule],
        textures: &catalog::Textures,
    ) -> Self {
        let mut out = Self::default();
        for model in models {
            let Some(rule) = rules.iter().rev().find(|r| r.matches(&model, textures)) else {
                continue;
            };
            if !rule.enabled || model.geometry.is_empty() {
                continue;
            }
            if let Some(emitter) = detect(&model, rule, vram, textures, &mut out.emissions) {
                out.models.insert(model.geometry.as_ptr() as usize, emitter);
            }
        }
        out.material_emissions = out
            .emissions
            .iter()
            .map(|(region, emission)| (textures.regions[region].clone(), *emission))
            .collect();
        out
    }

    pub fn source(&self, geometry: &[Geometry], matrix: impl FnOnce() -> Matrix) -> Option<Source> {
        if geometry.is_empty() {
            return None;
        }
        let emitter = self.models.get(&(geometry.as_ptr() as usize))?;
        let matrix = matrix();
        let rotation = Mat3::from_cols_array(&std::array::from_fn(|i| {
            f32::from(matrix.rotation[i % 3][i / 3]) / 4096.0
        }));
        Some(Source {
            name: emitter.name.clone(),
            light: Light {
                flame: emitter.flame,
                emission_radius: 0.0,
                position: rotation * emitter.position
                    + Vec3::from(matrix.translation.map(|v| v as f32).to_array()),
                color: emitter.color,
                radius: 12_000.0 * emitter.radius_scale,
            },
        })
    }
}

fn detect(
    model: &catalog::ModelSource<'_>,
    rule: &Rule,
    vram: &tenchu_assets::texture::Vram,
    textures: &catalog::Textures,
    emissions: &mut BTreeMap<catalog::Region, [f32; 4]>,
) -> Option<Emitter> {
    let mut position = Vec3::ZERO;
    let mut color = Vec3::ZERO;
    let mut weight = 0.0;
    for geometry in model.geometry {
        for face in render_mesh::draw_tmd_faces(geometry) {
            let Some(region) = textures.face(face) else {
                continue;
            };
            let id = &textures.regions[&region];
            if rule.texture.as_ref().is_some_and(|texture| texture != id) {
                continue;
            }
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
                if area == 0.0 || !rule.emits_surface(points) {
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
        flame: rule.flame,
        name: rule.label.clone().unwrap_or_else(|| model.label.clone()),
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
            let mut history = super::super::interpolation::History::default();
            history.reset(&game);
            for _ in 0..40 {
                game.step(0).unwrap();
                history.tick(&game);
            }
            let before = crate::session::tests::state(&game);
            let emitters = Emitters::build(&game, &Pack::default(), true);
            let sources = emitters.sources(&game);
            let frame = history.sample(game.tick, 0.5).unwrap();
            let mut expected = emitters.placed.clone();
            let mut append = |key, geometry: &[Geometry], matrix| {
                let matrix =
                    super::super::interpolation::Frame::transform(Some(&frame), key, matrix);
                if let Some(source) = emitters.source(geometry, || matrix) {
                    expected.push(source);
                }
            };
            render_mesh::visit_character_parts(&game, true, &mut append);
            render_mesh::visit_hinged_parts(&game, append);
            let fields = |sources: Vec<Source>| {
                sources
                    .into_iter()
                    .map(|source| {
                        (
                            source.name,
                            source.light.position,
                            source.light.color,
                            source.light.radius,
                            source.light.emission_radius,
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let textures =
                catalog::Textures::new(&game.mission.vram, &catalog::models(&game.mission));
            let shared = Emitters::build_with_textures(&game, &Pack::default(), true, &textures);
            assert_eq!(shared.emissions, emitters.emissions);
            assert_eq!(shared.material_emissions, emitters.material_emissions);
            assert_eq!(fields(shared.sources(&game)), fields(sources.clone()));
            assert_eq!(
                fields(shared.sources_presented(&game, Some(&frame))),
                fields(expected.clone())
            );
            assert_eq!(
                fields(emitters.sources_presented(&game, Some(&frame))),
                fields(expected)
            );
            let selected = crate::lighting::sources(
                &game,
                &crate::settings::Lighting {
                    enabled: true,
                    fires: false,
                    count: 1,
                    ..Default::default()
                },
                None,
                None,
                sources.clone(),
            );
            assert_eq!(
                selected.len(),
                sources.len(),
                "Mission {mission}: local budgets must retain distant lamps"
            );
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
                let player = Vec3::from(game.player().position.map(|v| v as f32).to_array());
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
