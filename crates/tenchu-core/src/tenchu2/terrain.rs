use crate::{
    Result,
    math::{Math, Position, Vector3},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::{tenchu2::collision::Collision, terrain::Node};

mod gaisen;
mod probe;
pub use probe::{Options, Probe};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    pub cell: usize,
    pub node: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cursor {
    pub current: Option<Surface>,
    pub last: Option<Surface>,
    pub material: u16,
    pub index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Base,
    Surface,
    Uncached,
    Relative,
}

pub fn level(
    map: &Collision,
    cursor: &mut Cursor,
    position: Position,
    query: Query,
) -> Result<Option<i32>> {
    if let Some(source) = &map.gaisen {
        return gaisen::level(map, source, cursor, position, query);
    }
    let x = position.x / 10;
    let y = (position.y / 10) as i16;
    let z = position.z / 10;
    cursor.material = 0;
    let cached = cursor.current.filter(|&id| {
        map.cells
            .get(id.cell)
            .and_then(|cell| cell.nodes.get(id.node))
            .is_some_and(|node| {
                query != Query::Uncached && node.division == u16::MAX && contains(node, x, y, z)
            })
    });
    let (cell_index, start, end) = if let Some(id) = cached {
        (id.cell, id.node, id.node + 1)
    } else {
        let size = i32::from(map.cell_size);
        if size <= 0 {
            return Err("Invalid Tenchu 2 collision cell size".into());
        }
        let column = ((x - i32::from(map.origin[0])) / size) as i16;
        let row = ((z - i32::from(map.origin[1])) / size) as i16;
        let index = i32::from(row) * map.dimensions[0] as i32 + i32::from(column);
        let Some(cell) = usize::try_from(index).ok().and_then(|i| map.cells.get(i)) else {
            return Ok(None);
        };
        if cell.nodes.is_empty() || y > cell.maximum {
            return Ok(None);
        }
        let mut bucket = 3;
        let span = i32::from(cell.maximum) - i32::from(cell.minimum);
        while bucket > 0 && i32::from(cell.minimum) + span * bucket as i32 / 4 > i32::from(y) {
            bucket -= 1;
        }
        (index as usize, cell.starts[bucket], cell.nodes.len())
    };
    let cell = &map.cells[cell_index];
    let mut height = None;
    for i in start..end {
        let node = cell.nodes.get(i).ok_or("Invalid Tenchu 2 surface index")?;
        if !contains(node, x, y, z) {
            continue;
        }
        let value = if query == Query::Base {
            i32::from(node.y)
        } else {
            let dx = i32::from((x - i32::from(node.bounds[0])) as i16);
            let dz = i32::from((z - i32::from(node.bounds[1])) as i16);
            let width = i32::from(node.bounds[2].wrapping_sub(node.bounds[0]));
            let depth = i32::from(node.bounds[3].wrapping_sub(node.bounds[1]));
            if width == 0 || depth == 0 {
                return Err("Empty Tenchu 2 collision surface".into());
            }
            let bit = ((dz * 4 / depth) * 4 + dx * 4 / width) & 31;
            let slope = node.attributes & 0xc000;
            if (i32::from(node.division as i16) >> bit) & 1 == 0 {
                if slope == 0 {
                    break;
                }
                continue;
            }
            let value = i32::from(node.y)
                + match slope {
                    0x8000 => dx * i32::from(node.dy) / width,
                    0xc000 => dz * i32::from(node.dy) / depth,
                    _ => 0,
                };
            if value < i32::from(y) {
                continue;
            }
            cursor.index = if cached.is_some() { 0 } else { i };
            value
        };
        cursor.current = Some(Surface {
            cell: cell_index,
            node: i,
        });
        cursor.material = node.attributes;
        height = Some(value);
        break;
    }
    cursor.last = cursor.current;
    if let Some(height) = height.filter(|_| cursor.material & 0xf00 != 0x100) {
        Ok(Some(
            (height
                - if query == Query::Relative {
                    i32::from(y)
                } else {
                    0
                })
                * 10,
        ))
    } else {
        cursor.current = None;
        Ok(None)
    }
}

fn contains(node: &Node, x: i32, y: i16, z: i32) -> bool {
    let [x1, z1, x2, z2] = node.bounds.map(i32::from);
    i32::from(y) <= i32::from(node.y) + i32::from(node.dy).max(0)
        && x1 <= x
        && x < x2
        && z1 <= z
        && z < z2
}

pub fn segment(
    map: &Collision,
    cursor: &mut Cursor,
    position: Position,
    delta: Position,
    math: &Math,
) -> Result<Option<Position>> {
    let mut step = delta.map(|v| v as i16);
    let mut count = 1i16;
    while step.into_iter().any(|v| i32::from(v).abs() > 100) {
        step = step.map(|v| v >> 1);
        count *= 2;
    }
    passage(map, cursor, position, step, count, math)
}

pub fn passage(
    map: &Collision,
    cursor: &mut Cursor,
    mut position: Position,
    step: Vector3<i16>,
    count: i16,
    math: &Math,
) -> Result<Option<Position>> {
    let Some((step, mut count)) = ray_step(step, count, math) else {
        return Ok(None);
    };
    if level(map, cursor, position, Query::Surface)?.is_none() {
        return Ok(Some(position));
    }
    let material = cursor.material;
    let query = ray_query(step);
    loop {
        if level(map, cursor, position, query)?.is_none()
            || material_changed(material, cursor.material)
        {
            return Ok(Some(position.wrapping_sub(step.map(i32::from))));
        }
        position = position.wrapping_add(step.map(i32::from));
        let old_count = count;
        count = count.wrapping_sub(1);
        if old_count <= 0 {
            return Ok(None);
        }
    }
}

pub fn traverse(
    map: &Collision,
    cursor: &mut Cursor,
    mut position: Position,
    step: Vector3<i16>,
    count: i16,
    math: &Math,
) -> Result<Option<Position>> {
    let Some((step, mut count)) = ray_step(step, count, math) else {
        return Ok(None);
    };
    if step.x == 0 && step.z == 0 {
        return Ok(None);
    }
    if level(map, cursor, position, Query::Surface)?.is_none() {
        return Ok(Some(position));
    }
    let material = cursor.material;
    let query = ray_query(step);
    loop {
        let height = level(map, cursor, position, query)?;
        if height.is_none() || material_changed(material, cursor.material) {
            return Ok(Some(if cursor.current.is_some() {
                position.wrapping_sub(step.map(i32::from))
            } else {
                position
            }));
        }
        let id = cursor.current.ok_or("Missing Tenchu 2 traversal surface")?;
        let node = &map.cells[id.cell].nodes[id.node];
        let skip = node.division == 0;
        let steps = if skip {
            let boundary = |coordinate: i32, delta: i16, near: i16, far: i16| -> Result<i32> {
                if delta == 0 {
                    return Ok(32767);
                }
                (i32::from(if delta < 0 { near } else { far }) * 10)
                    .wrapping_sub(coordinate)
                    .checked_div(i32::from(delta))
                    .ok_or_else(|| "Tenchu 2 traversal division overflow".into())
            };
            boundary(position.x, step.x, node.bounds[0], node.bounds[2])?
                .min(boundary(
                    position.z,
                    step.z,
                    node.bounds[1],
                    node.bounds[3],
                )?)
                .wrapping_add(1)
                .max(1)
        } else {
            1
        };
        count = count.wrapping_sub(steps as i16);
        position = position.wrapping_add(step.map(|v| i32::from(v).wrapping_mul(steps)));
        if skip && position.y.wrapping_sub(height.unwrap()).wrapping_abs() > 499 {
            cursor.current = None;
        }
        if count <= 0 {
            return Ok(None);
        }
    }
}

fn ray_step(step: Vector3<i16>, count: i16, math: &Math) -> Option<(Vector3<i16>, i16)> {
    if count > 0 {
        return Some((step, count));
    }
    let squared = step
        .into_iter()
        .fold(0i32, |sum, v| sum.wrapping_add(i32::from(v).pow(2)));
    let length = math.square_root(squared);
    if length == 0 {
        return None;
    }
    Some((
        math.normalize_short(step.to_array()).map(|v| v >> 4).into(),
        if count == 0 { (length >> 8) as i16 } else { 62 },
    ))
}

fn ray_query(step: Vector3<i16>) -> Query {
    if i32::from(step.x).abs() < 128 && i32::from(step.z).abs() < 128 {
        Query::Uncached
    } else {
        Query::Surface
    }
}

fn material_changed(previous: u16, current: u16) -> bool {
    if previous & 0xf00 == 0x300 {
        current & 0x200 == 0
    } else {
        current & 0xf00 == 0x300
    }
}
