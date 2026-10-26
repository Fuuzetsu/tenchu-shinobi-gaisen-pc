use crate::Result;
use crate::math::Position;
use tenchu_assets::terrain::{AreaMap, Node};

pub const NONE: i32 = i32::MIN;
pub const STEP_DOWN: u16 = 1;
pub const RETURN_DELTA: u16 = 2;
pub const ALLOW_DEEP: u16 = 4;
pub const FIRST_HIT: u16 = 8;
pub const REUSE_CACHED: u16 = 16;

pub fn passage(
    map: &AreaMap,
    cursor: &mut Cursor,
    mut position: Position,
    step: [i16; 3],
    count: i16,
) -> Result<Option<Position>> {
    let mut remaining = if count <= 0 { 100 } else { count };
    loop {
        let floor = level(map, cursor, position, 0)?;
        if floor == NONE {
            return Ok(Some(Position::from_fn(|i| {
                position[i].wrapping_sub(i32::from(step[i]))
            })));
        }
        let node = map.node(cursor.area)?;
        let [x1, z1, x2, z2] = node.bounds.map(|v| i32::from(v) * 10);
        let ceiling = if cursor.row == 0 {
            -1000000
        } else {
            i32::from(
                map.rows
                    .get(cursor.row - 1)
                    .ok_or("Invalid passage row")?
                    .height,
            ) * 10
        };
        loop {
            for i in 0..3 {
                position[i] = position[i].wrapping_add(i32::from(step[i]));
            }
            remaining = remaining.wrapping_sub(1);
            if remaining == 0 {
                return Ok(None);
            }
            if !(x1 <= position.x
                && position.x <= x2
                && floor <= position.y
                && position.y <= ceiling
                && z1 <= position.z
                && position.z <= z2)
            {
                break;
            }
        }
    }
}

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
    let dx = i32::from(x.wrapping_sub(x1) as i16);
    let dz = i32::from(z.wrapping_sub(z1) as i16);
    let xs = i32::from((x2 - x1 + 1) as i16);
    let zs = i32::from((z2 - z1 + 1) as i16);
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
    Ok(i32::from((i32::from(node.y) + slope) as i16))
}

pub fn level(map: &AreaMap, cursor: &mut Cursor, position: Position, mode: u16) -> Result<i32> {
    let Position {
        x: wx,
        y: wy,
        z: wz,
    } = position;
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
    position: Position,
    width: i32,
    mode: u16,
) -> Result<Probe> {
    let center = level(map, cursor, position, mode & !REUSE_CACHED)?;
    let mut result = Probe {
        level: center,
        height: if center == NONE {
            0
        } else {
            center.wrapping_sub(position.y)
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
    // Bit order matches CONFLICT.C's direction table and RefrectVector/RefrectMove.
    for (i, [dx, dz]) in [[-1, -1], [1, -1], [-1, 1], [1, 1]].into_iter().enumerate() {
        let other = level(
            map,
            cursor,
            Position::new(
                position.x.wrapping_add(width.wrapping_mul(dx)),
                position.y,
                position.z.wrapping_add(width.wrapping_mul(dz)),
            ),
            mode,
        )?;
        if other == NONE
            || (other.wrapping_sub(position.y) < -500
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

pub fn particle_floor(
    map: &AreaMap,
    cursor: &mut Cursor,
    position: Position,
    hint: &mut Option<Node>,
    mode: u16,
) -> Result<i32> {
    let Position { x, y, z } = position.map(|v| v / 10);
    // A particle can retain a node from Balma's map after the active map swaps.
    if let Some(node) = *hint
        && i32::from(node.y) - 200 <= y
        && y <= i32::from(node.y)
        && node.contains(x, z)
    {
        return if node.dy == 0 {
            Ok(i32::from(node.y) * 10)
        } else {
            let value = node_level(node, x, z)?;
            Ok(if value == NONE {
                NONE
            } else {
                value.wrapping_mul(10)
            })
        };
    }
    let query = Position::new(position.x, position.y.wrapping_sub(300), position.z);
    let floor = level(map, cursor, query, mode)?;
    if position.y <= floor && map.node(cursor.area)?.division == u16::MAX {
        *hint = Some(map.node(cursor.area)?);
    }
    Ok(floor)
}

pub fn ground_ray(
    map: &AreaMap,
    cursor: &mut Cursor,
    math: &crate::math::Math,
    from: Position,
    to: Position,
    mode: u16,
) -> Result<(i32, Position)> {
    let delta = to.wrapping_sub(from);
    let step = (500i32 << 12)
        .checked_div(math.vector_distance(delta))
        .filter(|v| *v > 0)
        .ok_or("Ground ray has a zero or negative step")?;
    let mut last = from;
    let mut hint = None;
    let mut time = step;
    while time < 4096 {
        let point = Position::from_fn(|i| from[i].wrapping_add(delta[i].wrapping_mul(time) / 4096));
        if particle_floor(map, cursor, point, &mut hint, mode)? < point.y {
            break;
        }
        last = point;
        time = time.wrapping_add(step);
    }
    Ok((time, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    mod vector;

    fn flat_map() -> AreaMap {
        let mut bytes = vec![0u8; 48];
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
        bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
        for base in [8, 36] {
            for (i, v) in [-1000i16, -1000, 1000, 1000].into_iter().enumerate() {
                bytes[base + i * 2..base + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
            }
        }
        bytes[44..46].copy_from_slice(&0x2000u16.to_le_bytes());
        bytes[46..48].copy_from_slice(&u16::MAX.to_le_bytes());
        AreaMap::parse(bytes).unwrap()
    }

    #[test]
    fn passage_excludes_the_final_step_and_returns_before_an_invalid_start() {
        let map = flat_map();
        let mut cursor = Cursor::new(&map);
        let start = [9990, -100, 0];
        let step = [100, 0, 0];
        assert_eq!(
            passage(&map, &mut cursor, start.into(), step, 1).unwrap(),
            None
        );
        assert_eq!(
            passage(&map, &mut cursor, start.into(), step, 2).unwrap(),
            Some(start.into())
        );
        assert_eq!(
            passage(&map, &mut cursor, Position::new(10090, -100, 0), step, 1).unwrap(),
            Some(start.into())
        );
        assert_eq!(
            passage(&map, &mut cursor, Position::new(0, -100, 0), [0; 3], 0).unwrap(),
            None
        );
    }

    #[test]
    fn particle_hint_uses_division_field_and_preserves_global_cursor_on_a_hit() {
        let map = flat_map();
        let mut cursor = Cursor::new(&map);
        let mut hint = None;
        assert_eq!(
            particle_floor(&map, &mut cursor, Position::new(0, -100, 0), &mut hint, 0).unwrap(),
            0
        );
        assert_eq!(hint, Some(map.node(32).unwrap()));
        cursor.last_y = 12345;
        assert_eq!(
            particle_floor(&map, &mut cursor, Position::new(10, -200, 10), &mut hint, 0).unwrap(),
            0
        );
        assert_eq!(cursor.last_y, 12345);
    }

    #[test]
    fn particle_hint_survives_switching_to_a_different_area_map() {
        let map = flat_map();
        let mut cursor = Cursor::new(&map);
        cursor.last_y = 987;
        let mut hint = Some(Node {
            y: -50,
            ..map.node(32).unwrap()
        });
        assert_eq!(
            particle_floor(&map, &mut cursor, Position::new(0, -600, 0), &mut hint, 0).unwrap(),
            -500
        );
        assert_eq!(cursor.last_y, 987);
        assert_eq!(
            particle_floor(&map, &mut cursor, Position::new(0, -200, 0), &mut hint, 0).unwrap(),
            0
        );
        assert_eq!(hint, Some(map.node(32).unwrap()));
    }

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
