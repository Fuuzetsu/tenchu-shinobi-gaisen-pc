use crate::{
    render_mesh::{self, Vertex},
    renderer::Renderer,
    session::Options,
    source::DiscSource,
    visuals::{catalog, pack::Pack},
};
use glam::{Mat4, Vec3};
use std::{collections::BTreeMap, fs, path::Path};
use tenchu_assets::{Result, game::GameAssets, model::Texture, texture::Vram};

fn points(triangle: &[Vertex]) -> [Vec3; 3] {
    std::array::from_fn(|i| Vec3::from(triangle[i].position))
}

fn hit(vram: &Vram, triangle: &[Vertex], origin: Vec3, direction: Vec3) -> Option<f32> {
    let [a, b, c] = points(triangle);
    let distance = crate::debug::terrain::triangle(origin, direction, [a, b, c])?;
    if triangle[0].flags & render_mesh::TEXTURED != 0 {
        let e1 = b - a;
        let e2 = c - a;
        let point = origin + direction * distance - a;
        let d00 = e1.dot(e1);
        let d01 = e1.dot(e2);
        let d11 = e2.dot(e2);
        let denominator = d00 * d11 - d01 * d01;
        let u = (d11 * point.dot(e1) - d01 * point.dot(e2)) / denominator;
        let v = (d00 * point.dot(e2) - d01 * point.dot(e1)) / denominator;
        let uv = std::array::from_fn(|i| {
            (triangle[0].uv[i] * (1.0 - u - v) + triangle[1].uv[i] * u + triangle[2].uv[i] * v)
                .floor() as u16
        });
        if vram.texel(triangle[0].page as u16, triangle[0].palette as u16, uv) == 0 {
            return None;
        }
    }
    Some(distance)
}

fn visible(vram: &Vram, vertices: &[Vertex], eye: Vec3, center: Vec3) -> bool {
    let ray = center - eye;
    !vertices.chunks_exact(3).any(|triangle| {
        hit(vram, triangle, eye, ray.normalize())
            .is_some_and(|distance| distance < ray.length() - 10.0)
    })
}

fn camera_position(vram: &Vram, vertices: &[Vertex], center: Vec3, offset: Vec3) -> Vec3 {
    let direction = offset.normalize();
    let distance = vertices
        .chunks_exact(3)
        .filter_map(|triangle| hit(vram, triangle, center + direction * 20.0, direction))
        .fold(offset.length(), |nearest, hit| nearest.min(hit * 0.9));
    center + direction * distance
}

pub fn render(
    assets: &GameAssets<DiscSource>,
    output: &Path,
    ids: &[&str],
    options: Options,
    ticks: u32,
) -> Result<()> {
    if ids.is_empty()
        || ids
            .iter()
            .any(|id| id.len() != 32 || !id.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("Supply comma-separated texture identities from export-assets".into());
    }
    if output.exists()
        && fs::read_dir(output)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("Preview output must be an empty directory".into());
    }
    let mut game = options.load(assets)?;
    for _ in 0..ticks {
        game.step(0)?;
    }
    let pack = Pack::from_env()?;
    let vertices = render_mesh::stage(&game.mission, &game.math, None);
    let player = Vec3::from(game.player().position.to_array().map(|n| n as f32));
    let mut regions = BTreeMap::new();
    let mut surfaces: BTreeMap<String, Vec<(f32, Vec3, Vec3)>> = BTreeMap::new();
    for triangle in vertices.chunks_exact(3) {
        if triangle[0].flags & render_mesh::TEXTURED == 0 {
            continue;
        }
        let mut bounds = [255, 255, 0, 0];
        for vertex in triangle {
            for axis in 0..2 {
                bounds[axis] = bounds[axis].min(vertex.uv[axis] as u16);
                bounds[axis + 2] = bounds[axis + 2].max(vertex.uv[axis] as u16);
            }
        }
        let region = catalog::Region::find(
            &game.mission.vram,
            Texture {
                page: triangle[0].page as u16,
                palette: triangle[0].palette as u16,
            },
            bounds,
        );
        let id = regions.entry(region).or_insert_with(|| {
            catalog::image_id(
                [region.rect[2], region.rect[3]],
                &region.original_pixels(&game.mission.vram),
            )
        });
        if !ids.contains(&id.as_str()) {
            continue;
        }
        let [a, b, c] = points(triangle);
        let normal = -(b - a).cross(c - a).normalize_or_zero();
        if normal == Vec3::ZERO {
            continue;
        }
        let center = (a + b + c) / 3.0;
        surfaces.entry(id.clone()).or_default().push((
            center.distance_squared(player),
            center,
            normal,
        ));
    }
    for id in ids {
        if !surfaces.contains_key(*id) {
            return Err(format!(
                "Texture {id} is not used by this mission's scenery"
            ));
        }
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .ok_or("No graphics adapter")?;
    let size = [1280, 720];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, &game.mission.vram))?;
    renderer.set_asset_pack(&pack);
    let mesh = renderer.upload(&vertices);
    let actors = renderer.upload(&render_mesh::characters(&game, None));
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Material review"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut graphics = if std::env::var_os("TENCHU_SETTINGS").is_some() {
        crate::settings::Store::new()?.load()?.graphics
    } else {
        let mut graphics = crate::settings::Graphics {
            fog: false,
            ..Default::default()
        };
        graphics.set_filtering(4);
        graphics
    };
    let mut views = Vec::new();
    for (id, candidates) in &mut surfaces {
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (_, center, normal) = candidates
            .iter()
            .copied()
            .find(|(_, center, normal)| {
                visible(
                    &game.mission.vram,
                    &vertices,
                    *center + *normal * 2800.0,
                    *center,
                )
            })
            .or_else(|| {
                candidates.iter().copied().find(|(_, center, normal)| {
                    let eye =
                        camera_position(&game.mission.vram, &vertices, *center, *normal * 2800.0);
                    eye.distance(*center) > 40.0
                        && visible(&game.mission.vram, &vertices, eye, *center)
                })
            })
            .ok_or_else(|| format!("No unobstructed view of {id}"))?;
        let up = if normal.y.abs() > 0.95 {
            Vec3::Z
        } else {
            Vec3::NEG_Y
        };
        let right = normal.cross(up).normalize();
        for (name, distance, offset, vertical) in [
            ("front", 2800.0, 0.0, 0.0),
            ("angle", 3500.0, 0.65, 0.0),
            ("wide", 9000.0, 0.3, 0.0),
            ("context", 6500.0, 0.5, -1.0),
        ] {
            let eye = camera_position(
                &game.mission.vram,
                &vertices,
                center,
                (normal + right * offset + up * vertical) * distance,
            );
            let camera = Mat4::perspective_rh(
                55.0_f32.to_radians(),
                size[0] as f32 / size[1] as f32,
                20.0,
                250_000.0,
            ) * Mat4::look_at_rh(eye, center, up);
            for enabled in [false, true] {
                graphics.asset_pack = enabled;
                renderer.configure(&graphics);
                renderer.draw_lit(
                    &texture.create_view(&Default::default()),
                    camera,
                    &[&mesh, &actors],
                    None,
                    &[],
                    &[&mesh, &actors],
                )?;
                let suffix = if enabled { "pack" } else { "original" };
                super::read_texture(
                    &renderer,
                    &texture,
                    &output.join(format!("{id}-{name}-{suffix}.png")),
                )?;
            }
            views.push(serde_json::json!({"texture": id, "view": name, "tick": ticks, "eye": eye.to_array(), "target": center.to_array(), "unobstructed": visible(&game.mission.vram, &vertices, eye, center)}));
        }
    }
    fs::write(
        output.join("views.json"),
        serde_json::to_vec_pretty(&views).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn preview_rays_pass_through_cutouts_but_stop_at_opaque_and_semitransparent_texels() {
        let mut vram = Vram::default();
        let mut triangle = [Vertex::zeroed(); 3];
        for (vertex, point) in triangle.iter_mut().zip([
            Vec3::new(-100.0, -100.0, 0.0),
            Vec3::new(100.0, -100.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        ]) {
            vertex.position = point.to_array();
            vertex.page = 0x100;
            vertex.flags = render_mesh::TEXTURED;
        }
        let eye = Vec3::new(0.0, 0.0, 100.0);
        let target = Vec3::new(0.0, 0.0, -100.0);
        assert!(visible(&vram, &triangle, eye, target));
        for word in [1, 0x8000] {
            vram.words[0] = word;
            assert!(!visible(&vram, &triangle, eye, target));
        }
        vram.words[0] = 0;
        for vertex in &mut triangle {
            vertex.flags = 0;
        }
        assert!(!visible(&vram, &triangle, eye, target));
    }
}
