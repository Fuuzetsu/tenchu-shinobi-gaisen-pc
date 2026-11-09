use glam::Vec3;
use tenchu_assets::{tenchu2::collision::Collision, terrain::Node};
#[cfg(not(target_arch = "wasm32"))]
use tenchu_core::{math::Position, tenchu2::terrain};

pub(super) fn surfaces(map: &Collision) -> impl Iterator<Item = (Node, [Vec3; 4])> + '_ {
    map.cells
        .iter()
        .flat_map(|cell| &cell.nodes)
        .flat_map(|&node| {
            let full = node.division == u16::MAX;
            let count = if full { 1 } else { 16 };
            (0..count).filter_map(move |index| {
                if node.division & (1 << index) == 0 {
                    return None;
                }
                let [x1, z1, x2, z2] = node.bounds.map(f32::from);
                let (width, depth) = (x2 - x1, z2 - z1);
                if width <= 0.0 || depth <= 0.0 {
                    return None;
                }
                let (x, z, scale) = if full {
                    (0, 0, 1.0)
                } else {
                    (index % 4, index / 4, 0.25)
                };
                let point = |x, z| {
                    let (dx, dz) = (width * x as f32 * scale, depth * z as f32 * scale);
                    let slope = match node.attributes & 0xc000 {
                        0x8000 => dx * f32::from(node.dy) / width,
                        0xc000 => dz * f32::from(node.dy) / depth,
                        _ => 0.0,
                    };
                    Vec3::new(x1 + dx, f32::from(node.y) + slope, z1 + dz) * 10.0
                };
                Some((
                    node,
                    [
                        point(x, z),
                        point(x + 1, z),
                        point(x, z + 1),
                        point(x + 1, z + 1),
                    ],
                ))
            })
        })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn pick(map: &Collision, origin: Vec3, direction: Vec3) -> Option<Position> {
    surfaces(map)
        .filter(|(node, _)| node.attributes & 0xf00 != 0x100)
        .flat_map(|(_, corners)| [[0, 1, 2], [1, 3, 2]].map(|indices| indices.map(|i| corners[i])))
        .filter_map(|triangle| {
            let distance = crate::debug::terrain::triangle(origin, direction, triangle)?;
            let mut p = Position::from(
                (origin + direction * distance)
                    .to_array()
                    .map(|v| v.round() as i32),
            );
            let y = terrain::level(
                map,
                &mut Default::default(),
                p.wrapping_sub(Position::new(0, 10, 0)),
                terrain::Query::Surface,
            )
            .ok()??;
            if (p.y - y).abs() > 20 {
                return None;
            }
            p.y = y;
            Some((distance, p))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, p)| p)
}
