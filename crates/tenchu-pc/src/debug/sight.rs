use glam::Vec3;
use tenchu_assets::terrain::AreaMap;
use tenchu_core::{math::Position, terrain};

pub(super) fn endpoint(map: &AreaMap, origin: Vec3, target: Vec3) -> Vec3 {
    let start: Position = origin.to_array().map(|v| v.round() as i32).into();
    let mut delta: Position = (target - origin)
        .to_array()
        .map(|v| v.round() as i32)
        .into();
    let mut count = 1;
    while delta.iter().any(|v| v.abs() > 100) {
        delta = delta.map(|v| v >> 1);
        count *= 2;
    }
    let mut cursor = terrain::Cursor::new(map);
    match terrain::passage(
        map,
        &mut cursor,
        start,
        delta.map(|v| v as i16).to_array(),
        count + 1,
    ) {
        Ok(Some(hit)) => {
            let hit = Vec3::from(hit.map(|v| v as f32).to_array());
            let ray = target - origin;
            origin + ray * ((hit - origin).dot(ray) / ray.length_squared().max(1.0)).clamp(0.0, 1.0)
        }
        Ok(None) => target,
        Err(_) => origin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sight_outline_stops_at_collision_boundary_and_keeps_clear_rays() {
        let mut bytes = vec![0u8; 48];
        bytes[0..2].copy_from_slice(&300i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
        for (i, value) in [-200i16, -200, 199, 199].into_iter().enumerate() {
            bytes[8 + i * 2..10 + i * 2].copy_from_slice(&value.to_le_bytes());
        }
        bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
        for (offset, value) in [
            (0, 300i16),
            (2, 0),
            (4, -200),
            (6, -200),
            (8, 199),
            (10, 199),
        ] {
            bytes[32 + offset..34 + offset].copy_from_slice(&value.to_le_bytes());
        }
        bytes[46..48].copy_from_slice(&0xffffu16.to_le_bytes());
        let map = AreaMap::parse(bytes).unwrap();
        let start = Vec3::ZERO;
        assert_eq!(endpoint(&map, start, Vec3::X * 1000.0), Vec3::X * 1000.0);
        let blocked = endpoint(&map, start, Vec3::X * 5000.0);
        assert!(blocked.x > 1800.0 && blocked.x < 2000.0, "{blocked}");
    }
}
