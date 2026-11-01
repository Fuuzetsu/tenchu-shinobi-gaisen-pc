use super::{
    appearance,
    interpolation::Part,
    models::Models,
    surfaces::{Stamp, Surfaces},
};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
    settings::{Weather, WeatherMode},
};
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tenchu_assets::model;
use tenchu_core::game::Game;

const MARK_LIMIT: usize = 512;
const VERTEX_LIMIT: usize = 65_536;

pub fn snow_materials(pack: &super::pack::Pack, artwork: bool) -> BTreeSet<String> {
    let mut materials: BTreeSet<String> =
        serde_json::from_str(include_str!("../../data/snow.json")).unwrap();
    if artwork {
        for (id, surface) in &pack.surfaces {
            match surface.snow {
                Some(true) => {
                    materials.insert(id.clone());
                }
                Some(false) => {
                    materials.remove(id);
                }
                None => {}
            }
        }
    }
    materials
}

struct Mark {
    vertices: Vec<Vertex>,
    position: Vec3,
    age: f32,
    burial: f32,
    exposed: bool,
    retirement: Option<f32>,
}

impl Mark {
    fn opacity(&self, seconds: u16) -> f32 {
        (1.0 - self.burial / f32::from(seconds))
            .clamp(0.0, 1.0)
            .powi(2)
            * (1.0 - self.age / 600.0).clamp(0.0, 1.0)
            * self.retirement.unwrap_or(1.0)
    }
}

#[derive(Default)]
pub struct Footprints {
    tick: Option<u64>,
    direction: Option<Vec3>,
    rigs: BTreeMap<i16, Vec<(usize, [i16; 3])>>,
    last: BTreeMap<Part, Vec3>,
    marks: VecDeque<Mark>,
}

fn feet(parts: &[model::Part]) -> Vec<(usize, [i16; 3])> {
    if parts.len() != 15 {
        return Vec::new();
    }
    let parents = appearance::parents(parts);
    let mut candidates = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if parents.contains(&Some(index)) {
            continue;
        }
        let mut height = i32::from(part.position[1]);
        let mut parent = parents[index];
        for _ in 0..parts.len() {
            let Some(p) = parent else { break };
            height += i32::from(parts[p].position[1]);
            parent = parents[p];
        }
        let points: Vec<_> = part.geometry.iter().flat_map(|g| &g.vertices).collect();
        let Some(y) = points.iter().map(|p| p[1]).max() else {
            continue;
        };
        let center = |axis| {
            let min = points.iter().map(|p| p[axis]).min().unwrap();
            let max = points.iter().map(|p| p[axis]).max().unwrap();
            ((i32::from(min) + i32::from(max)) / 2) as i16
        };
        candidates.push((height + i32::from(y), index, [center(0), y, center(2)]));
    }
    candidates.sort_by_key(|&(height, _, _)| std::cmp::Reverse(height));
    candidates
        .into_iter()
        .take(2)
        .map(|(_, index, sole)| (index, sole))
        .collect()
}

impl Footprints {
    fn update_shelter(&mut self, surfaces: &Surfaces, direction: Vec3) {
        if self.direction == Some(direction) {
            return;
        }
        self.direction = Some(direction);
        for mark in &mut self.marks {
            mark.exposed = surfaces.exposed(mark.position - Vec3::Y * 5.0, direction);
        }
    }

    pub fn clear(&mut self) {
        self.marks.clear();
        self.last.clear();
    }

    pub fn tick(
        &mut self,
        game: &Game,
        models: &Models,
        surfaces: &Surfaces,
        settings: &Weather,
        snow: f32,
        eye: Vec3,
    ) {
        let coating = settings.mode != WeatherMode::Original && snow > 0.01;
        let baked = settings.baked_snow && surfaces.contains(render_mesh::SNOW);
        if !settings.footprints || !coating && !baked {
            self.clear();
            self.tick = Some(game.tick);
            return;
        }
        if self.tick == Some(game.tick) {
            return;
        }
        if self.tick.is_some_and(|tick| tick + 1 != game.tick) {
            self.clear();
        }
        self.tick = Some(game.tick);
        self.update_shelter(surfaces, settings.direction());
        let falling = if settings.mode.snow() {
            f32::from(settings.amount) / 100.0
        } else {
            0.0
        };
        for mark in &mut self.marks {
            mark.age += 1.0 / 30.0;
            if let Some(fade) = &mut mark.retirement {
                *fade = (*fade - 1.0 / 45.0).max(0.0);
            }
            if mark.exposed {
                mark.burial += falling / 30.0;
            }
        }
        self.marks
            .retain(|mark| mark.opacity(settings.footprint_seconds) > 0.005);
        let mut soles = BTreeMap::new();
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            let position = Vec3::from(actor.position.map(|n| n as f32).to_array());
            if actor.health <= 0
                || actor.map.height.abs() > 160
                || position.distance_squared(eye) > 30000.0_f32.powi(2)
            {
                continue;
            }
            let Some(parts) = game.mission.models.get(&actor.rendered_mesh_kind) else {
                continue;
            };
            let rig = self
                .rigs
                .entry(actor.rendered_mesh_kind)
                .or_insert_with(|| feet(parts));
            for &(index, sole) in rig.iter() {
                if let Some(&model) = actor.parts.get(index) {
                    soles.insert((actor.root_model, model), sole);
                }
            }
        }
        let mut contacts = Vec::new();
        appearance::visit_parts(game, true, Some(models), None, |key, _, matrix| {
            if let Part::Actor { root, model, .. } = key
                && let Some(&sole) = soles.get(&(root, model))
            {
                let position = Vec3::from(matrix.transform(sole).map(|n| n as f32).to_array());
                let forward = super::models::rotation(matrix) * Vec3::Z;
                contacts.push((key.canonical(), position, forward));
            }
        });
        self.last
            .retain(|key, _| contacts.iter().any(|(part, _, _)| part == key));
        let mut vertex_count: usize = self.marks.iter().map(|m| m.vertices.len()).sum();
        for (key, position, forward) in contacts {
            if self
                .last
                .get(&key)
                .is_some_and(|old| old.distance_squared(position) < 400.0_f32.powi(2))
            {
                continue;
            }
            let Some(hit) = surfaces.cast(position - Vec3::Y * 80.0, position + Vec3::Y * 180.0)
            else {
                continue;
            };
            if hit.normal.y > -0.55 || !coating && hit.flags & render_mesh::SNOW == 0 {
                continue;
            }
            let mut vertices = surfaces.project(&Stamp {
                center: hit.position,
                normal: hit.normal,
                radius: 190.0,
                depth: 30.0,
                seed: 0,
                shape: Some((forward, 0.48)),
            });
            if vertices.is_empty() || vertices.len() > 3072 {
                continue;
            }
            for v in &mut vertices {
                v.normal = hit.normal.to_array();
                v.flags = (v.flags & !render_mesh::STAIN) | render_mesh::FOOTPRINT;
                if !settings.baked_snow || v.flags & render_mesh::FOLIAGE != 0 {
                    v.flags &= !render_mesh::SNOW;
                }
            }
            if self.marks.len() == MARK_LIMIT || vertex_count + vertices.len() > VERTEX_LIMIT {
                continue;
            }
            vertex_count += vertices.len();
            self.last.insert(key, position);
            self.marks.push_back(Mark {
                vertices,
                position: hit.position,
                age: 0.0,
                burial: 0.0,
                retirement: None,
                exposed: surfaces.exposed(hit.position - Vec3::Y * 5.0, settings.direction()),
            });
        }
        self.retire_to_budget();
    }

    fn retire_to_budget(&mut self) {
        let mut count = 0;
        let mut vertices = 0;
        for mark in self.marks.iter().filter(|m| m.retirement.is_none()) {
            count += 1;
            vertices += mark.vertices.len();
        }
        for mark in &mut self.marks {
            if count <= MARK_LIMIT * 3 / 4 && vertices <= VERTEX_LIMIT * 3 / 4 {
                break;
            }
            if mark.retirement.is_none() {
                mark.retirement = Some(1.0);
                count -= 1;
                vertices -= mark.vertices.len();
            }
        }
    }

    pub fn geometry(&self, settings: &Weather) -> Geometry {
        let vertices: Vec<_> = self
            .marks
            .iter()
            .flat_map(|mark| {
                let opacity = mark.opacity(settings.footprint_seconds);
                mark.vertices.iter().map(move |v| Vertex {
                    opacity: v.opacity * opacity,
                    ..*v
                })
            })
            .collect();
        let count = vertices.len() as u32;
        Geometry {
            vertices,
            batches: vec![Batch {
                blend: 5,
                range: 0..count,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_snow_buries_tracks_without_an_abrupt_expiry() {
        let mut mark = Mark {
            vertices: Vec::new(),
            position: Vec3::ZERO,
            age: 0.0,
            burial: 0.0,
            exposed: true,
            retirement: None,
        };
        assert_eq!(mark.opacity(60), 1.0);
        mark.age = 30.0;
        mark.burial = 30.0;
        assert!(mark.opacity(60) > 0.15 && mark.opacity(60) < 0.3);
        mark.burial = 60.0;
        assert_eq!(mark.opacity(60), 0.0);
    }

    #[test]
    fn crowded_tracks_retire_without_deleting_visible_marks() {
        let mut prints = Footprints::default();
        for _ in 0..MARK_LIMIT {
            prints.marks.push_back(Mark {
                vertices: Vec::new(),
                position: Vec3::ZERO,
                age: 0.0,
                burial: 0.0,
                exposed: true,
                retirement: None,
            });
        }
        prints.retire_to_budget();
        assert_eq!(prints.marks.len(), MARK_LIMIT);
        assert_eq!(prints.marks.front().unwrap().opacity(60), 1.0);
        assert_eq!(
            prints
                .marks
                .iter()
                .filter(|m| m.retirement.is_some())
                .count(),
            MARK_LIMIT / 4
        );
        prints.marks.front_mut().unwrap().retirement = Some(0.5);
        assert_eq!(prints.marks.front().unwrap().opacity(60), 0.5);
        assert_eq!(prints.marks.back().unwrap().opacity(60), 1.0);
    }

    #[test]
    fn turning_the_wind_rechecks_shelter_for_existing_tracks() {
        use bytemuck::Zeroable;
        let points = [
            [-500.0, -1000.0, -500.0],
            [500.0, -1000.0, -500.0],
            [-500.0, -1000.0, 500.0],
            [500.0, -1000.0, 500.0],
        ];
        let surfaces = Surfaces::new(&[0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: points[i],
            ..Vertex::zeroed()
        }));
        let mut prints = Footprints::default();
        prints.marks.push_back(Mark {
            vertices: Vec::new(),
            position: Vec3::ZERO,
            age: 1.0,
            burial: 0.5,
            exposed: true,
            retirement: None,
        });
        prints.update_shelter(&surfaces, Vec3::Y);
        assert!(!prints.marks[0].exposed);
        prints.update_shelter(&surfaces, Vec3::new(2.0, 1.0, 0.0).normalize());
        assert!(prints.marks[0].exposed);
        assert_eq!(prints.marks[0].burial, 0.5);
        assert_eq!(prints.marks[0].age, 1.0);
        prints.update_shelter(&surfaces, Vec3::Y);
        assert!(!prints.marks[0].exposed);
    }
}

#[cfg(test)]
mod retail_tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; existing snow takes tracks with Original weather, while bare stages and the Original preset do not"]
    fn existing_snow_takes_tracks_without_a_weather_coating() {
        use tenchu_core::{math::Position, terrain};
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for mission in [1, 5, 10] {
            for character in 0..2 {
                let mut game = crate::session::Options {
                    mission,
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                game.intelligence.ignore_player = true;
                game.roster.order.retain(|&id| id == game.player_id);
                let mut graphics = crate::settings::Graphics::default();
                graphics.weather.baked_snow = true;
                let mut models =
                    Models::build(&game.mission, &Default::default(), &graphics).unwrap();
                let mut review = std::env::var_os("TENCHU_WEATHER_REVIEW").map(|_| {
                    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
                    review.renderer.bind_models(&mut models);
                    review
                });
                let stage = render_mesh::stage(&game.mission, &game.math, Some(&models));
                let surfaces = Surfaces::new(&stage);
                assert_eq!(surfaces.contains(render_mesh::SNOW), mission != 1);
                if mission != 1 {
                    let mut candidates: Vec<_> = stage
                        .chunks_exact(3)
                        .filter_map(|t| {
                            if t[0].flags & render_mesh::SNOW == 0 {
                                return None;
                            }
                            let [a, b, c] = std::array::from_fn(|i| Vec3::from(t[i].position));
                            let normal = (b - a).cross(c - a);
                            if normal.length() < 500_000.0 || normal.normalize().y.abs() < 0.9 {
                                return None;
                            }
                            let uv = std::array::from_fn(|i| {
                                (t.iter().map(|v| v.uv[i]).sum::<f32>() / 3.0) as u16
                            });
                            let rgba = tenchu_assets::texture::rgba8(game.mission.vram.texel(
                                t[0].page as u16,
                                t[0].palette as u16,
                                uv,
                            ));
                            if rgba[..3].iter().any(|&v| v < 185) {
                                return None;
                            }
                            Some((a + b + c) / 3.0)
                        })
                        .collect();
                    let origin = Vec3::from(game.player().position.to_array().map(|v| v as f32));
                    let mut cursor = terrain::Cursor::new(&game.mission.terrain);
                    candidates.sort_by(|a, b| {
                        a.distance_squared(origin)
                            .total_cmp(&b.distance_squared(origin))
                    });
                    let start = candidates
                        .into_iter()
                        .find(|p| {
                            let floor = terrain::level(
                                &game.mission.terrain,
                                &mut cursor,
                                Position::from(p.to_array().map(|v| v as i32))
                                    .wrapping_sub(Position::new(0, 100, 0)),
                                0,
                            )
                            .unwrap();
                            (floor as f32 - p.y).abs() < 100.0
                        })
                        .expect("snowy walking surface");
                    game.teleport_player(Position::from(start.to_array().map(|v| v as i32)), true)
                        .unwrap();
                    println!("m{mission} c{character}: {:?}", game.player().position);
                }
                let mut prints = Footprints::default();
                for tick in 0..120 {
                    game.step(if tick < 20 { 0 } else { tenchu_core::input::UP })
                        .unwrap();
                    let state = crate::session::tests::state(&game);
                    let eye = Vec3::from(game.player().position.to_array().map(|v| v as f32));
                    prints.tick(&game, &models, &surfaces, &graphics.weather, 0.0, eye);
                    assert_eq!(state, crate::session::tests::state(&game));
                }
                assert_eq!(prints.marks.is_empty(), mission == 1);
                if mission != 1
                    && let Some(output) = std::env::var_os("TENCHU_WEATHER_REVIEW")
                {
                    let output = std::path::PathBuf::from(output);
                    std::fs::create_dir_all(&output).unwrap();
                    let review = review.as_mut().unwrap();
                    graphics.fog = false;
                    review.configure(graphics.clone());
                    let center = prints.marks[prints.marks.len() / 2].position;
                    let camera = glam::Mat4::perspective_rh(
                        55.0_f32.to_radians(),
                        1280.0 / 720.0,
                        10.0,
                        200000.0,
                    ) * glam::Mat4::look_at_rh(
                        center + Vec3::new(1600.0, -4800.0, -2600.0),
                        center,
                        Vec3::NEG_Y,
                    );
                    let stage = review.renderer.upload(&stage);
                    let actors = review
                        .renderer
                        .upload(&render_mesh::characters(&game, Some(&models)));
                    let effects = review
                        .renderer
                        .upload_effects(prints.geometry(&graphics.weather));
                    for (name, effects) in [("before", None), ("after", Some(&effects))] {
                        review
                            .capture(
                                camera,
                                &[&stage, &actors],
                                effects,
                                true,
                                &output.join(format!("snow-m{mission}-c{character}-{name}.png")),
                            )
                            .unwrap();
                    }
                }
                prints.tick(
                    &game,
                    &models,
                    &surfaces,
                    &Weather::default(),
                    0.0,
                    Vec3::ZERO,
                );
                assert!(prints.marks.is_empty());
            }
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; animated feet leave grounded, bounded tracks without changing gameplay"]
    fn both_players_leave_tracks_on_actual_surfaces() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for character in [0, 1] {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            let graphics = crate::settings::Graphics::default();
            let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
            let stage = render_mesh::stage(&game.mission, &game.math, Some(&models));
            let surfaces = Surfaces::new(&stage);
            let settings = Weather {
                mode: WeatherMode::Snow,
                ..Default::default()
            };
            let mut prints = Footprints::default();
            for tick in 0..300 {
                game.step(if tick < 30 { 0 } else { tenchu_core::input::UP })
                    .unwrap();
                let state = crate::session::tests::state(&game);
                let eye = Vec3::from(game.player().position.map(|n| n as f32).to_array());
                prints.tick(&game, &models, &surfaces, &settings, 1.0, eye);
                assert_eq!(state, crate::session::tests::state(&game));
                let vertices = prints.geometry(&settings).vertices;
                assert!(vertices.len() <= VERTEX_LIMIT);
                assert!(vertices.iter().all(|v| Vec3::from(v.position).is_finite()
                    && Vec3::from(v.normal).length_squared() > 0.9));
            }
            assert!(
                prints.marks.len() >= 6,
                "character {character}: {} tracks, rigs {:?}",
                prints.marks.len(),
                prints.rigs
            );
            assert!(
                prints.marks.front().unwrap().age > 7.0,
                "Geometry pressure retired footprints before they could fade"
            );
            let count = prints.marks.len();
            prints.tick(&game, &models, &surfaces, &settings, 1.0, Vec3::ZERO);
            assert_eq!(
                prints.marks.len(),
                count,
                "presentation cadence must not create tracks"
            );
            if let Some(output) = std::env::var_os("TENCHU_WEATHER_REVIEW") {
                let output = std::path::PathBuf::from(output);
                std::fs::create_dir_all(&output).unwrap();
                let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
                let center = Vec3::from(game.player().position.map(|n| n as f32).to_array());
                let camera = glam::Mat4::perspective_rh(
                    55.0_f32.to_radians(),
                    1280.0 / 720.0,
                    10.0,
                    200000.0,
                ) * glam::Mat4::look_at_rh(
                    center + Vec3::new(3000.0, -6500.0, -4500.0),
                    center,
                    Vec3::NEG_Y,
                );
                let stage = review.renderer.upload(&stage);
                let actors = review
                    .renderer
                    .upload(&render_mesh::characters(&game, Some(&models)));
                let effects = review.renderer.upload_effects(prints.geometry(&settings));
                let mut graphics = graphics.clone();
                graphics.weather = settings;
                graphics.fog = false;
                review.configure(graphics);
                review.renderer.weather.snow = 1.0;
                review
                    .capture(
                        camera,
                        &[&stage, &actors],
                        Some(&effects),
                        true,
                        &output.join(format!("footprints-{character}.png")),
                    )
                    .unwrap();
            }
        }
    }
}
