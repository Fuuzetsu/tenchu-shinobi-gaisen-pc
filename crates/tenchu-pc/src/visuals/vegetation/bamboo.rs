use super::{Material, Vertex, render_mesh};
use glam::Vec3;

pub fn stalks(source: &[Vertex], materials: &[Material], budget: &mut usize) -> Vec<Vertex> {
    let Some(material) = source[0]
        .material
        .checked_sub(1)
        .and_then(|i| materials.get(i as usize))
    else {
        return source.to_vec();
    };
    let image = &material.image;
    let rect = material
        .region
        .map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
    let p: [Vec3; 3] = std::array::from_fn(|i| source[i].position.into());
    let normal = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
    if normal.y.abs() > 0.7 {
        return source.to_vec();
    }
    let across = Vec3::NEG_Y.cross(normal).normalize();
    let (min, max) = source.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(min, max), v| {
            let p = Vec3::from(v.position);
            let p = Vec3::new(p.dot(across), p.y, p.dot(normal));
            (min.min(p), max.max(p))
        },
    );
    let mut runs = Vec::new();
    let mut start = None;
    for x in 0..=image.size[0] {
        let solid = x < image.size[0]
            && (0..image.size[1])
                .filter(|&y| image.rgba[((y * image.size[0] + x) * 4 + 3) as usize] >= 200)
                .count()
                * 3
                > image.size[1] as usize;
        if solid {
            start.get_or_insert(x);
        } else if let Some(a) = start.take() {
            runs.push((a, x));
        }
    }
    let segments = ((max.y - min.y) / 800.0).ceil().clamp(1.0, 48.0) as usize;
    let cost = runs.len() * segments * 8 * 6;
    if runs.is_empty() || cost > *budget {
        return source.to_vec();
    }
    *budget -= cost;
    let mut out = Vec::with_capacity(cost);
    for (left, right) in runs {
        let center = (left + right) as f32 * 0.5 / image.size[0] as f32;
        let center = across * (min.x + (max.x - min.x) * center) + normal * ((min.z + max.z) * 0.5);
        let radius = ((right - left) as f32 / image.size[0] as f32 * (max.x - min.x) * 0.45)
            .clamp(10.0, 120.0);
        for y in 0..segments {
            for side in 0..8 {
                let quad = std::array::from_fn::<_, 4, _>(|i| {
                    let angle = (side + i % 2) as f32 * std::f32::consts::TAU / 8.0;
                    let facing = across * angle.cos() + normal * angle.sin();
                    let height = max.y - (max.y - min.y) * (y + i / 2) as f32 / segments as f32;
                    let u = (left as f32 + (right - left) as f32 * (0.5 + angle.cos() * 0.35))
                        / image.size[0] as f32;
                    Vertex {
                        position: (center + Vec3::Y * height + facing * radius).to_array(),
                        normal: facing.to_array(),
                        uv: [
                            rect[0] + u * rect[2],
                            rect[1] + (i / 2) as f32 * (rect[3] - 1.0),
                        ],
                        flags: source[0].flags | render_mesh::FOLIAGE | render_mesh::DOUBLE_SIDED,
                        ..source[0]
                    }
                });
                out.extend([0, 1, 2, 1, 3, 2].map(|i| quad[i]));
            }
        }
    }
    out
}
