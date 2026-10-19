use crate::Result;
use tenchu_assets::terrain::{AreaMap, Node};

pub const NONE: i32 = i32::MIN;
pub const STEP_DOWN: u16 = 1;
pub const RETURN_DELTA: u16 = 2;
pub const ALLOW_DEEP: u16 = 4;
pub const FIRST_HIT: u16 = 8;
pub const REUSE_CACHED: u16 = 16;

#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub row: usize,
    pub area: usize,
    pub last_y: i32,
    pub attribute: u16,
}

impl Cursor {
    pub fn new(map: &AreaMap) -> Self {
        Self {
            row: 0,
            area: map.rows.first().map_or(0, |r| r.lookup.unwrap_or(r.list)),
            last_y: NONE,
            attribute: 0x81,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Probe {
    pub level: i32,
    pub height: i32,
    pub attribute: u16,
    pub vector: u8,
    pub angle_low: u8,
    pub angle_high: u8,
    pub area: usize,
    pub row: usize,
}

pub fn node_level(node: Node, x: i32, z: i32) -> Result<i32> {
    let [x1, z1, x2, z2] = node.bounds.map(|v| i32::from(v as u16));
    let dx = x.wrapping_sub(x1) as i16 as i32;
    let dz = z.wrapping_sub(z1) as i16 as i32;
    let xs = (x2 - x1 + 1) as i16 as i32;
    let zs = (z2 - z1 + 1) as i16 as i32;
    if xs == 0 || zs == 0 {
        return Err("Zero terrain span".into());
    }
    let shift = (dz * 4 / zs) * 4 + dx * 4 / xs;
    if u32::from(node.division) & 1u32.wrapping_shl(shift as u32) == 0 {
        return Ok(NONE);
    }
    let slope = match node.attributes & 0xc000 {
        0x4000 => dx * i32::from(node.dy) / xs,
        0x8000 => dz * i32::from(node.dy) / zs,
        _ => 0,
    };
    Ok((i32::from(node.y) + slope) as i16 as i32)
}

pub fn level(map: &AreaMap, cursor: &mut Cursor, position: [i32; 3], mode: u16) -> Result<i32> {
    let [wx, wy, wz] = position;
    let (x, z) = (wx / 10, wz / 10);
    let y = (wy / 10).wrapping_sub(if mode & STEP_DOWN != 0 { 150 } else { 0 });
    cursor.attribute = 0x81;
    let mut height = NONE;
    if mode & REUSE_CACHED != 0 && y == cursor.last_y {
        let node = map.node(cursor.area)?;
        if node.contains(x, z) {
            height = node_level(node, x, z)?;
        }
    }
    cursor.last_y = y;
    let mut row = cursor.row;
    if row > map.rows.len() {
        return Err("Invalid terrain row cursor".into());
    }
    while row > 0 && map.rows.get(row).is_some_and(|r| y < i32::from(r.height)) {
        row -= 1;
    }
    while map.rows.get(row).is_some_and(|r| i32::from(r.height) < y) {
        row += 1;
    }
    while height == NONE {
        let Some(bounds) = map.rows.get(row) else {
            break;
        };
        let [x1, z1, x2, z2] = bounds.bounds.map(i32::from);
        if x >= x1 && x <= x2 && z >= z1 && z <= z2 {
            for i in map.first_node(bounds, x, z)?..bounds.count {
                let at = bounds.list + i * 16;
                let node = map.node(at)?;
                if z < i32::from(node.bounds[1]) {
                    break;
                }
                if !node.contains(x, z) {
                    continue;
                }
                cursor.row = row;
                cursor.area = at;
                if mode & FIRST_HIT != 0 {
                    height = if node.division == u16::MAX {
                        i32::from(node.y)
                    } else {
                        node_level(node, x, z)?
                    };
                    cursor.attribute = node.attributes;
                    break;
                }
                let current = node_level(node, x, z)?;
                if (height == NONE || current < height) && y <= current {
                    height = current;
                    cursor.attribute = node.attributes;
                    if cursor.attribute & 0x2000 != 0 {
                        break;
                    }
                }
            }
        }
        row += 1;
    }
    if height == NONE || cursor.attribute & 2 != 0 {
        return Ok(NONE);
    }
    let world_y = height.wrapping_mul(10);
    let delta = world_y.wrapping_sub(wy);
    if delta < -1000 && mode & ALLOW_DEEP == 0 {
        return Ok(NONE);
    }
    Ok(if mode & RETURN_DELTA != 0 {
        delta
    } else {
        world_y
    })
}

pub fn vector(
    map: &AreaMap,
    cursor: &mut Cursor,
    position: [i32; 3],
    width: i32,
    mode: u16,
) -> Result<Probe> {
    let center = level(map, cursor, position, mode & !REUSE_CACHED)?;
    let mut result = Probe {
        level: center,
        height: if center == NONE {
            0
        } else {
            center.wrapping_sub(position[1])
        },
        attribute: cursor.attribute,
        row: cursor.row,
        area: cursor.area,
        ..Probe::default()
    };
    if center == NONE && mode & ALLOW_DEEP == 0 {
        result.attribute = 2;
        result.vector = 15;
        result.angle_low = 15;
        result.angle_high = 15;
        return Ok(result);
    }
    for (i, [dx, dz]) in [[0, -1], [1, 0], [0, 1], [-1, 0]].into_iter().enumerate() {
        let other = level(
            map,
            cursor,
            [
                position[0].wrapping_add(width.wrapping_mul(dx)),
                position[1],
                position[2].wrapping_add(width.wrapping_mul(dz)),
            ],
            mode,
        )?;
        if other == NONE
            || (other.wrapping_sub(position[1]) < -500
                && mode & ALLOW_DEEP == 0
                && (result.attribute | cursor.attribute) & 0xc000 == 0)
        {
            result.vector |= 1 << i;
        } else if center < other {
            result.angle_low |= 1 << i;
        } else if other < center {
            result.angle_high |= 1 << i;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_spans_holes_and_slope_narrowing_follow_compute_area_level() {
        let mut node = Node {
            y: 32760,
            dy: 100,
            bounds: [-100, -100, 99, 99],
            attributes: 0x4000,
            division: 0xffff,
        };
        assert_eq!(node_level(node, 0, 0).unwrap(), -32726);
        node.attributes |= 0x8000;
        assert_eq!(node_level(node, 0, 0).unwrap(), 32760);
        node.division &= !(1 << 10);
        assert_eq!(node_level(node, 0, 0).unwrap(), NONE);
        node.bounds = [-20000, -100, 20000, 99];
        node.division = u16::MAX;
        node.attributes = 0x4000;
        assert_eq!(node_level(node, -20000, -100).unwrap(), 32760);
    }
}
