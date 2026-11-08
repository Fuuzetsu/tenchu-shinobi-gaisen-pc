use crate::{
    Result,
    math::{Math, Position, Vector3},
    tenchu2::terrain::{self, Cursor, Query},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Aim {
    pub direction: Position,
    pub reach: Option<i32>,
    pub target: Position,
    pub distance: i32,
    pub phase: i32,
}

#[derive(Deserialize)]
pub struct Input {
    pub eye: Position,
    pub look: Position,
    pub root: Position,
    pub camera: i16,
    pub grapple: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reticle {
    pub scale: i16,
    pub ready: bool,
}

pub fn target<'a>(
    origin: Position,
    direction: Position,
    owner: usize,
    actors: impl Iterator<Item = (usize, &'a crate::tenchu2::actor::Actor)>,
    math: &Math,
) -> Option<usize> {
    use crate::tenchu2::actor::State;
    let mut closest = i32::MAX;
    let mut target = None;
    for (index, actor) in actors {
        if index == owner
            || actor.flags & 8 != 0
            || matches!(actor.state, State::Hurt | State::Dead)
        {
            continue;
        }
        let delta = actor.position.wrapping_sub(origin);
        if delta.x.wrapping_abs() > 16000 || delta.z.wrapping_abs() > 16000 {
            continue;
        }
        let normal = math.normalize_vector(delta);
        let facing = normal
            .x
            .wrapping_mul(direction.x)
            .wrapping_add(normal.z.wrapping_mul(direction.z));
        // VectorNormal returns the squared length of its signed-halfword inputs.
        let distance = delta.into_iter().fold(0i32, |sum, v| {
            let v = i32::from(v as i16);
            sum.wrapping_add(v * v)
        });
        if facing > 0xddb3d7 && distance < closest {
            closest = distance;
            target = Some(index);
        }
    }
    target
}

impl Aim {
    pub fn update(
        &mut self,
        input: Input,
        map: &Collision,
        cursor: &mut Cursor,
        math: &Math,
    ) -> Result<Option<Reticle>> {
        let delta = input.look.wrapping_sub(input.eye);
        if input.camera != 20 {
            if input.camera == 6 {
                self.reach = None;
                self.direction = delta;
            } else {
                self.phase = 0;
            }
            return Ok(None);
        }
        self.phase = self.phase.wrapping_add(1);
        let normal: Position = math
            .normalize_short(delta.map(|v| v as i16).to_array())
            .map(i32::from)
            .into();
        let target = wall(map, cursor, input.eye, normal.map(|v| v as i16), -1, math)?
            .unwrap_or_else(|| input.eye.wrapping_add(normal.map(|v| v * 4)));
        let offset = target.wrapping_sub(input.root);
        self.direction = normal;
        let scale = if dot(offset, normal) <= 0 {
            self.distance = math.length(normal);
            self.target = input.root.wrapping_add(normal.map(|v| v * 4));
            self.reach = None;
            2048
        } else {
            self.target = target.wrapping_add(normal.map(|v| v >> 3));
            self.direction = offset;
            let squared_input = offset.map(|v| i32::from(v as i16));
            self.reach = Some(math.length(Position::new(squared_input.x, 0, squared_input.z)));
            self.distance = math.length(squared_input);
            if self.distance > 12288 {
                2048
            } else if self.distance > 4096 {
                4096 - ((self.distance - 4096) >> 2) as i16
            } else {
                4096
            }
        };
        let ready = if input.grapple {
            let target = ledge(
                map,
                cursor,
                input.eye,
                input.root,
                normal.map(|v| v as i16),
                -1,
                math,
            )?
            .filter(|&target| dot(target.wrapping_sub(input.root), normal) > 0);
            if let Some(target) = target {
                self.target = target;
                self.direction = target.wrapping_sub(input.root);
                let squared_input = self.direction.map(|v| i32::from(v as i16));
                self.reach = Some(math.length(Position::new(squared_input.x, 0, squared_input.z)));
                self.distance = math.length(squared_input);
                true
            } else {
                self.reach = None;
                false
            }
        } else {
            true
        };
        Ok(Some(Reticle { scale, ready }))
    }
}

fn dot(a: Position, b: Position) -> i32 {
    a.into_iter()
        .zip(b)
        .fold(0i32, |sum, (a, b)| sum.wrapping_add(a.wrapping_mul(b)))
}

fn step(vector: Vector3<i16>, count: i16, limit: i16, math: &Math) -> Option<(Position, i16)> {
    if count > 0 {
        return Some((vector.map(i32::from), count));
    }
    let length = math.length(vector.map(i32::from));
    if length == 0 {
        return None;
    }
    Some((
        math.normalize_short(vector.to_array())
            .map(|v| i32::from(v >> 4))
            .into(),
        if count == 0 {
            (length >> 8) as i16
        } else {
            limit
        },
    ))
}

pub fn wall(
    map: &Collision,
    cursor: &mut Cursor,
    mut position: Position,
    direction: Vector3<i16>,
    count: i16,
    math: &Math,
) -> Result<Option<Position>> {
    let Some((step, mut count)) = step(direction, count, 62, math) else {
        return Ok(None);
    };
    if step.x == 0 && step.z == 0 {
        return Ok(None);
    }
    loop {
        let Some(height) = terrain::level(map, cursor, position, Query::Surface)? else {
            return Ok(Some(if cursor.current.is_some() {
                position.wrapping_sub(step)
            } else {
                position
            }));
        };
        let id = cursor.current.ok_or("Missing Tenchu 2 aim surface")?;
        let node = &map.cells[id.cell].nodes[id.node];
        let skip = node.division == 0;
        let distance = if skip {
            let edge = |axis: usize, near: i16, far: i16| {
                if step[axis] == 0 {
                    32767
                } else {
                    (i32::from(if step[axis] < 0 { near } else { far }) * 10)
                        .wrapping_sub(position[axis])
                        .wrapping_div(step[axis])
                }
            };
            edge(0, node.bounds[0], node.bounds[2])
                .min(edge(2, node.bounds[1], node.bounds[3]))
                .wrapping_add(1)
                .max(1)
        } else {
            1
        };
        count = count.wrapping_sub(distance as i16);
        position = position.wrapping_add(step.map(|v| v.wrapping_mul(distance)));
        if skip && position.y.wrapping_sub(height).wrapping_abs() > 499 {
            cursor.current = None;
        }
        if count <= 0 {
            return Ok(None);
        }
    }
}

pub fn ledge(
    map: &Collision,
    cursor: &mut Cursor,
    mut position: Position,
    root: Position,
    direction: Vector3<i16>,
    count: i16,
    math: &Math,
) -> Result<Option<Position>> {
    let Some((step, mut count)) = step(direction, count, 46, math) else {
        return Ok(None);
    };
    if step.x == 0 && step.z == 0 {
        return Ok(None);
    }
    let mut previous = 0;
    let mut last_height = position.y;
    loop {
        let height = terrain::level(map, cursor, position, Query::Uncached)?;
        let height = if let Some(height) = height {
            height
        } else {
            if cursor.material & 0xf00 == 0x100 {
                return Ok(None);
            }
            let mut higher = position;
            higher.y = higher.y.wrapping_sub(1000);
            let Some(height) = terrain::level(map, cursor, higher, Query::Uncached)? else {
                return Ok(None);
            };
            height
        };
        let material = cursor.material & 0x3000;
        if matches!(
            (previous, material),
            (0x1000, 0x3000) | (0x2000, 0x1000) | (0x3000, 0x1000 | 0x2000)
        ) {
            return Ok(None);
        }
        let steep = cursor.current.is_some_and(|id| {
            let node = &map.cells[id.cell].nodes[id.node];
            let extent = match node.attributes & 0xc000 {
                0x8000 => i32::from(node.bounds[2]) - i32::from(node.bounds[0]),
                0xc000 => i32::from(node.bounds[3]) - i32::from(node.bounds[1]),
                _ => return false,
            };
            extent < i32::from(node.dy)
        });
        if !steep
            && height < position.y.wrapping_add(1000)
            && height < root.y.wrapping_sub(1000)
            && height < last_height.wrapping_sub(256)
        {
            return Ok((cursor.material & 0xf00 == 0).then(|| {
                Position::new(
                    position.x.wrapping_sub(step.x >> 1),
                    height.wrapping_add(30),
                    position.z.wrapping_sub(step.z >> 1),
                )
            }));
        }
        previous = material;
        position = position.wrapping_add(step);
        let old_count = count;
        count = count.wrapping_sub(1);
        last_height = height;
        if old_count <= 0 {
            return Ok(None);
        }
    }
}
