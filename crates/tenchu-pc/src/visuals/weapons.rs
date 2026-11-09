use crate::render_mesh::{self, Vertex};
use tenchu_assets::texture::Vram;

pub fn metal_weapon(id: &str) -> bool {
    use tenchu_assets::weapon::kind::{CLAW, FIST, JAW};
    id.starts_with("tenchu2/weapons/")
        || id
            .strip_prefix("weapons/")
            .and_then(|id| i16::from_str_radix(id, 16).ok())
            .is_some_and(|kind| !matches!(kind, CLAW | FIST | JAW))
}

pub fn mark_metal(vertices: &mut [Vertex], vram: &Vram, materials: &[super::models::Material]) {
    for triangle in vertices.chunks_exact_mut(3) {
        if triangle[0].flags & render_mesh::TRANSLUCENT != 0 {
            continue;
        }
        let samples = std::array::from_fn::<_, 3, _>(|i| {
            let v = &triangle[i];
            let base = if let Some(material) = v
                .material
                .checked_sub(1)
                .and_then(|i| materials.get(i as usize))
            {
                let rect = material
                    .region
                    .map_or([0.0, 0.0, 1.0, 1.0], |r| r.rect.map(f32::from));
                let [x, y] = std::array::from_fn(|a| {
                    (((v.uv[a] + 0.5 - rect[a]) / rect[a + 2]).clamp(0.0, 1.0)
                        * (material.image.size[a] - 1) as f32)
                        .round() as usize
                });
                let at = (y * material.image.size[0] as usize + x) * 4;
                std::array::from_fn(|i| f32::from(material.image.rgba[at + i]) / 255.0)
            } else if v.flags & render_mesh::TEXTURED != 0 {
                let pixel = vram.texel(
                    v.page as u16,
                    v.palette as u16,
                    v.uv.map(|n| n.max(0.0) as u16),
                );
                [pixel & 31, (pixel >> 5) & 31, (pixel >> 10) & 31].map(|n| f32::from(n) / 31.0)
            } else {
                [1.0; 3]
            };
            std::array::from_fn::<_, 3, _>(|i| base[i] * v.color[i])
        });
        let silver = samples
            .iter()
            .filter(|c| {
                let lo = c.iter().copied().fold(f32::INFINITY, f32::min);
                let hi = c.iter().copied().fold(0.0, f32::max);
                hi > 0.22 && hi - lo < hi * 0.25
            })
            .count();
        if silver >= 2 {
            for v in triangle {
                v.flags |= render_mesh::WEAPON_METAL;
            }
        }
    }
}
