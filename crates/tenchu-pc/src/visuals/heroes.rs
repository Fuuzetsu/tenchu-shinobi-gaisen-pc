use crate::render_mesh::{self, Vertex};
use glam::Vec3;

pub fn part(id: &str) -> Option<usize> {
    let (body, part) = id.rsplit_once('/')?;
    if !matches!(body, "actors/0000" | "actors/0001")
        && !body
            .strip_prefix("tenchu2/actors/")
            .is_some_and(|name| tenchu_assets::tenchu2::characters::hero(name).is_some())
    {
        return None;
    }
    part.parse().ok().filter(|&part| part < 15)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_part(id: &str) -> Option<usize> {
    let bone = part(id)?;
    tenchu_assets::tenchu2::BONES
        .iter()
        .position(|&b| b == bone)
}

pub fn prepare(
    vertices: &mut Vec<Vertex>,
    part: usize,
    budget: &mut usize,
    skin: &mut Vec<super::appearance::skin::Influence>,
) {
    let cost = vertices.len().saturating_mul(64);
    if vertices.is_empty() || cost > *budget {
        return;
    }
    let (min, max) = vertices.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(min, max), v| {
            let p = Vec3::from(v.position);
            (min.min(p), max.max(p))
        },
    );
    let size = max - min;
    // Retail limbs are separate rigid meshes; shortening their Y axis opens joints.
    super::refine::preserve_height(
        vertices,
        if part == 2 { 0.012 } else { 0.035 },
        budget,
        skin,
    );
    if matches!(part, 13 | 14) && size.y > 0.0 && size.z > 0.0 {
        let sign = if part == 14 { -1.0 } else { 1.0 };
        for v in vertices.iter_mut() {
            if v.flags & render_mesh::TEXTURED != 0 {
                continue;
            }
            let p = Vec3::from(v.position);
            let y = ((p.y - min.y) / size.y).clamp(0.0, 1.0);
            let z = ((p.z - min.z) / size.z).clamp(0.0, 1.0);
            let envelope = ((y - 0.4) / 0.3).clamp(0.0, 1.0)
                * ((1.0 - y) / 0.15).clamp(0.0, 1.0)
                * (z * std::f32::consts::PI).sin();
            let grooves = (z * std::f32::consts::TAU * 4.0).cos() * 0.5 + 0.5;
            let depth = envelope * grooves.powi(4) * size.x * 0.045;
            let outward = (v.normal[0] * sign).max(0.0);
            v.position[0] -= sign * depth * outward;
            v.color = v
                .color
                .map(|n| n * (1.0 - envelope * grooves.powi(4) * outward * 0.2));
            v.normal = [0.0; 3];
        }
        super::smooth::prepare(vertices, true, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        settings::Graphics,
        visuals::{models::Models, pack::Pack},
    };
    use tenchu_core::{input, math::Matrix};

    #[test]
    #[ignore = "requires TENCHU_DISC; high-detail hero/outfit geometry must stay finite and bounded, preserve attachments and leave gameplay unchanged"]
    fn hero_outfits_keep_rigs_textures_and_finite_animation() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for (character, outfit) in [(0, 0), (0, 1), (1, 0), (1, 1), (1, 255)] {
            let options = crate::session::Options {
                character,
                ..Default::default()
            };
            let mut inventory = [0; 20];
            inventory[19] = outfit;
            let mut game = options.load_equipped(&assets, 1, inventory).unwrap();
            game.events = Default::default();
            let before = postcard::to_allocvec(&game).unwrap();
            let pack = Pack::from_env(&game.mission).unwrap();
            let mut graphics = Graphics {
                asset_pack: true,
                ..Default::default()
            };
            graphics.models.heroes = true;
            let models = Models::build(&game.mission, &pack, &graphics).unwrap();
            let mut count = 0;
            for (i, p) in game.mission.models[&character].iter().enumerate() {
                let mut old = Vec::new();
                let mut new = Vec::new();
                render_mesh::append_geometry(&mut old, &p.geometry, Matrix::default(), None);
                render_mesh::append_geometry(
                    &mut new,
                    &p.geometry,
                    Matrix::default(),
                    Some(&models),
                );
                assert!(new.len() >= old.len() * 16, "{character}/{outfit}/{i}");
                let min = old.iter().fold(Vec3::splat(f32::INFINITY), |p, v| {
                    p.min(Vec3::from(v.position))
                });
                let max = old.iter().fold(Vec3::splat(f32::NEG_INFINITY), |p, v| {
                    p.max(Vec3::from(v.position))
                });
                let margin = Vec3::splat(0.01);
                let refined_min_y = new
                    .iter()
                    .map(|v| v.position[1])
                    .fold(f32::INFINITY, f32::min);
                let refined_max_y = new
                    .iter()
                    .map(|v| v.position[1])
                    .fold(f32::NEG_INFINITY, f32::max);
                assert!(
                    (refined_min_y - min.y).abs() < 0.01 && (refined_max_y - max.y).abs() < 0.01,
                    "joint length changed: {character}/{outfit}/{i}"
                );
                assert!(
                    new.iter().all(|v| {
                        let p = Vec3::from(v.position);
                        p.is_finite()
                            && p.cmpge(min - margin).all()
                            && p.cmple(max + margin).all()
                            && Vec3::from(v.normal).is_finite()
                            && v.uv.iter().all(|n| n.is_finite())
                    }),
                    "bounds or invalid vertex {character}/{outfit}/{i}"
                );
                count += new.len() / 3;
            }
            assert!(count < 60000, "unbounded hero mesh: {count}");
            assert_eq!(before, postcard::to_allocvec(&game).unwrap());
            for tick in 0..180 {
                game.step(match tick % 60 {
                    0..20 => input::UP,
                    25 => input::SQUARE,
                    45 => input::CROSS,
                    _ => 0,
                })
                .unwrap();
                let before = postcard::to_allocvec(&game).unwrap();
                let mut drawn = Vec::new();
                render_mesh::update_characters(&mut drawn, &game, Some(&models), true, None);
                assert!(drawn.iter().all(|v| Vec3::from(v.position).is_finite()));
                assert_eq!(before, postcard::to_allocvec(&game).unwrap());
            }
            eprintln!("hero {character} outfit {outfit}: {count} triangles");
        }
    }
}
