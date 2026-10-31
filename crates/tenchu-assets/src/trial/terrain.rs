use super::{Mission, Tables, Theme};
use crate::{
    Result,
    model::{Corner, Face, Geometry, Part, Texture},
    stage::Placement,
    terrain::{AreaMap, Node},
};
use std::collections::BTreeMap;

const FINAL: u16 = 0x2000;
const WATER: u16 = 4;
const DEATH: u16 = 0x200;
const COLLAPSING: u16 = 0x1000;
const NEIGHBORS: [(i16, i16); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

pub struct Terrain {
    pub objects: Vec<Part>,
    pub names: Vec<String>,
    pub placements: Vec<Placement>,
    pub collision: AreaMap,
    pub ornaments: Vec<(super::Point, usize)>,
}

struct Mesh {
    geometry: Geometry,
    colors: Vec<u8>,
    lookup: BTreeMap<[i16; 3], u16>,
}
impl Mesh {
    fn new() -> Self {
        Self {
            geometry: Geometry {
                vertices: vec![],
                normals: vec![],
                faces: vec![],
            },
            colors: vec![],
            lookup: BTreeMap::new(),
        }
    }
    fn vertex(&mut self, mut point: [i16; 3], scale: i16, random: &mut impl FnMut() -> u16) -> u16 {
        if point[1] == -6 {
            point[1] = -8;
        }
        let y = point[1];
        point = point.map(|v| v * scale);
        if let Some(index) = self.lookup.get(&point) {
            return *index;
        }
        let index = self.geometry.vertices.len() as u16;
        self.lookup.insert(point, index);
        self.geometry.vertices.push(point);
        let mut color = (random() & 31) as i16 + 80 - y.abs() * 8;
        if random().is_multiple_of(100) {
            color -= 16;
        }
        self.colors.push(color as u8);
        index
    }
}
impl Theme {
    pub fn validate(&self, mission: &Mission) -> Result<()> {
        mission.validate()?;
        for cell in &mission.map {
            for (encoded, ornament) in [(cell.ground, false), (cell.ornament, true)] {
                if encoded != 255
                    && self
                        .tile(encoded)
                        .is_none_or(|tile| tile.is_ornament() != ornament)
                {
                    return Err("A map tile is unavailable in this theme or layer".into());
                }
            }
        }
        Ok(())
    }

    pub fn build(
        &self,
        mission: &Mission,
        tables: &Tables,
        mut random: impl FnMut() -> u16,
    ) -> Result<Terrain> {
        self.validate(mission)?;
        let mut out = Terrain {
            objects: vec![],
            names: vec![],
            placements: vec![],
            collision: self.collision(mission)?,
            ornaments: vec![],
        };
        for cz in 0..8 {
            for cx in 0..8 {
                let center = (cx * 4 + 2, cz * 4 + 2);
                let mut mesh = Mesh::new();
                for z in cz * 4..cz * 4 + 4 {
                    for x in cx * 4..cx * 4 + 4 {
                        let cell = mission.cell(x, z).unwrap();
                        let Some(tile) = self.tile(cell.ground) else {
                            continue;
                        };
                        let height = self.height(mission, x, z, false);
                        if height.abs() == 99 || tile.kind() == 9 {
                            continue;
                        }
                        for corner in 0..4 {
                            let v = tables.vertices[corner];
                            let (a, b) = (NEIGHBORS[corner], NEIGHBORS[(corner + 1) % 4]);
                            let mut flags = 0;
                            for (nx, nz) in [
                                (x, z),
                                (x + a.1, z - a.0),
                                (x + b.1, z - b.0),
                                (x + v[2], z + v[0]),
                            ] {
                                if mission.cell(nx, nz).is_none_or(|c| c.ground == 255) {
                                    flags |= 1;
                                } else if self
                                    .tile(mission.cell(nx, nz).unwrap().ornament)
                                    .is_some_and(|t| t.flags & 0x4000 != 0)
                                {
                                    flags |= 2;
                                } else if self.height(mission, nx, nz, true) < height {
                                    flags |= 4;
                                }
                            }
                            if flags != 0 {
                                let color = if flags & 1 != 0 {
                                    16
                                } else if flags & 2 != 0 {
                                    (random() as u8 & 15) | 192
                                } else {
                                    ((random() & 15) as i16 + 48 - height.abs() * 8) as u8
                                };
                                let index = mesh.vertex(
                                    [(x - center.0) * 2 + v[2], height, (center.1 - z) * 2 - v[0]],
                                    1000,
                                    &mut random,
                                );
                                mesh.colors[index as usize] = color;
                            }
                        }
                    }
                }
                for z in cz * 4..cz * 4 + 4 {
                    for x in cx * 4..cx * 4 + 4 {
                        let cell = mission.cell(x, z).unwrap();
                        let Some(tile) = self.tile(cell.ground) else {
                            continue;
                        };
                        let height = self.height(mission, x, z, false);
                        let rotation = usize::from(cell.ground >> 6);
                        let mut neighbors = [0; 4];
                        for (i, (dx, dz)) in NEIGHBORS.into_iter().enumerate() {
                            neighbors[i] = self.height(mission, x + dx, z + dz, false);
                            let Some(other) =
                                mission.cell(x + dx, z + dz).filter(|c| c.ground != 255)
                            else {
                                if tile.kind() == 8 {
                                    neighbors[i] = 99;
                                }
                                continue;
                            };
                            let other_kind = self.tile(other.ground).unwrap().kind();
                            let other_rotation = usize::from(other.ground >> 6);
                            if (5..=6).contains(&tile.kind()) {
                                if (5..=6).contains(&other_kind)
                                    && (tile.kind() != other_kind
                                        || rotation != other_rotation
                                        || ((dx == 0 || rotation & 1 != 0)
                                            && (dz == 0 || rotation & 1 == 0)))
                                    && (rotation != i || i != (other_rotation + 2) % 4)
                                {
                                    if rotation != other_rotation || tile.kind() == other_kind {
                                        neighbors[i] = 0;
                                    } else {
                                        neighbors[i] += 1;
                                    }
                                }
                            } else if (5..=6).contains(&other_kind) && (other_rotation + 2) % 4 != i
                            {
                                neighbors[i] = 0;
                            }
                        }
                        for face in &tables.faces[tile.kind()] {
                            let edge = (face.edge + rotation) % 4;
                            if height == 99 {
                                if neighbors[edge] == 99 {
                                    continue;
                                }
                            } else if face.edge != 4
                                && (height >= neighbors[edge]
                                    || tables.vertices[face.vertices[0]][1] >= neighbors[edge])
                            {
                                continue;
                            }
                            let mut indices = [0; 4];
                            for (i, vertex) in face.vertices.into_iter().enumerate() {
                                let v =
                                    tables.vertices[vertex / 4 * 4 + (vertex % 4 + rotation) % 4];
                                let point =
                                    [v[0] + (x - center.0) * 2, v[1], v[2] + (center.1 - z) * 2];
                                indices[i] = if tile.kind() == 9 {
                                    mesh.vertex(
                                        [point[0] * 10, point[1] * 10 + 1, point[2] * 10],
                                        100,
                                        &mut random,
                                    )
                                } else {
                                    mesh.vertex(point, 1000, &mut random)
                                };
                            }
                            let texture = tile.textures[face.texture];
                            if texture == -1 {
                                continue;
                            }
                            let image = self.images[texture as usize];
                            let h = if face.half == 0 { 63 } else { 31 };
                            let [u, v] = image.uv;
                            let v = v.wrapping_add(if face.half == 2 { 32 } else { 0 });
                            let uv = [
                                [u, v],
                                [u.wrapping_add(63), v],
                                [u, v.wrapping_add(h)],
                                [u.wrapping_add(63), v.wrapping_add(h)],
                            ];
                            mesh.geometry.faces.push(Face {
                                mode: 0x3c,
                                flags: 1,
                                corners: indices
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, vertex)| Corner {
                                        vertex,
                                        normal: None,
                                        color: [mesh.colors[vertex as usize]; 3],
                                        uv: uv[i],
                                    })
                                    .collect(),
                                texture: Some(Texture {
                                    page: image.page,
                                    palette: image.palette,
                                }),
                                double_sided: false,
                                translucent: false,
                            });
                        }
                    }
                }
                if !mesh.geometry.faces.is_empty() {
                    out.add(
                        vec![mesh.geometry],
                        format!("TRIAL_CHUNK_{cx}_{cz}"),
                        [
                            (i32::from(center.0) * 2 + 1) * 1000,
                            0,
                            (-i32::from(center.1) * 2 - 1) * 1000,
                        ],
                        0,
                    );
                }
                for z in cz * 4..cz * 4 + 4 {
                    for x in cx * 4..cx * 4 + 4 {
                        if out.ornaments.len() == 64 {
                            break;
                        }
                        let cell = mission.cell(x, z).unwrap();
                        let Some(tile) = self.tile(cell.ornament) else {
                            continue;
                        };
                        let mut height = self.height(mission, x, z, false);
                        if height.abs() == 99 {
                            height = 0;
                        }
                        let point = super::Point { x, z };
                        let mut position = point.world();
                        position[1] = i32::from(height) * 1000;
                        let index = out.placements.len();
                        out.add(
                            self.ornaments[tile.ornament.unwrap()].clone(),
                            tile.name.clone(),
                            position,
                            i32::from(cell.ornament >> 6) * 1024,
                        );
                        out.ornaments.push((point, index));
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn collision(&self, mission: &Mission) -> Result<AreaMap> {
        self.validate(mission)?;
        let mut nodes = Vec::new();
        for z in (0..32).rev() {
            for x in 0..32 {
                let cell = mission.cell(x, z).unwrap();
                let Some(tile) = self.tile(cell.ground) else {
                    continue;
                };
                let height = tile.height();
                if height == -99 {
                    continue;
                }
                let mut node = Node {
                    y: height * 100,
                    dy: 0,
                    bounds: [x * 200, -z * 200 - 200, x * 200 + 200, -z * 200],
                    attributes: FINAL,
                    division: 0xffff,
                };
                if height == 99 {
                    node.y = 3000;
                    node.attributes |= DEATH;
                } else if (5..=6).contains(&tile.kind()) {
                    let rotation = cell.ground >> 6;
                    node.y = 0;
                    node.dy = if tile.kind() == 6 { 200 } else { 100 };
                    node.attributes |= 1 | if rotation & 1 != 0 { 0x4000 } else { 0x8000 };
                    if rotation < 2 {
                        node.dy = -node.dy;
                    } else {
                        node.y -= node.dy;
                    }
                } else if tile.kind() == 9 {
                    node.attributes |= WATER;
                } else {
                    node.attributes |= 1;
                }
                if let Some(ornament) = self.tile(cell.ornament) {
                    if ornament.flags & 0x2000 != 0 {
                        nodes.push(node);
                        node.division = 0xffff;
                        node.y = 0;
                        node.attributes = FINAL | COLLAPSING | 1;
                    } else if ornament.flags & 0x800 != 0 {
                        let mut mask = 0x0660u16;
                        for (i, (dx, dz)) in NEIGHBORS.into_iter().enumerate() {
                            let neighbor = mission
                                .cell(x + dx, z + dz)
                                .and_then(|c| self.tile(c.ornament));
                            if (neighbor.is_none()
                                || self.height(mission, x + dx, z + dz, false) * 100 < node.y
                                || neighbor.is_some_and(|t| t.flags & 0x800 == 0))
                                && self.height(mission, x + dx, z + dz, true) * 100 < node.y
                            {
                                mask |= [0xf000, 0x8888, 0xf, 0x1111][i];
                            }
                        }
                        if mask != 0xffff {
                            node.division = !mask;
                            nodes.push(node);
                            node.division = mask;
                            node.attributes = FINAL | 1;
                        }
                        node.y = self.height(mission, x, z, true);
                    } else {
                        node.y = self.height(mission, x, z, true);
                        node.attributes = FINAL | 1;
                    }
                    if node.y.abs() == 99 {
                        continue;
                    }
                    node.y *= 100;
                }
                nodes.push(node);
            }
        }
        merge(&mut nodes, true);
        merge(&mut nodes, false);
        let mut levels = Vec::new();
        for height in (-8..=0).chain([30]) {
            let mut level = Vec::new();
            for node in &mut nodes {
                if node.division != 0
                    && (node.y == height * 100 && node.dy == 0 || height == 0 && node.y <= 0)
                {
                    let mut copied = *node;
                    if copied.attributes & WATER != 0 {
                        copied.y += 10;
                    }
                    level.push(copied);
                    node.division = 0;
                }
            }
            levels.push((height * 100, level));
        }
        AreaMap::from_nodes(levels)
    }
}
impl Terrain {
    fn add(&mut self, geometry: Vec<Geometry>, name: String, position: [i32; 3], facing: i32) {
        let part = self.objects.len();
        self.objects.push(Part {
            parent: -1,
            child: part as i16,
            position: [0; 3],
            geometry,
        });
        self.names.push(name);
        self.placements.push(Placement {
            part,
            cloned: false,
            position,
            facing,
        });
    }
}
fn merge(nodes: &mut [Node], equal: bool) {
    let eligible = |n: &Node| n.division == 0xffff && n.attributes & COLLAPSING == 0 && n.dy == 0;
    let mut axis = false;
    loop {
        axis = !axis;
        let mut changed = false;
        for i in 0..nodes.len() {
            if !eligible(&nodes[i]) {
                continue;
            }
            for j in i + 1..nodes.len() {
                let (a, b) = (nodes[i], nodes[j]);
                if !eligible(&b) || a.y != b.y || a.attributes != b.attributes {
                    continue;
                }
                let width = a.bounds[2] - a.bounds[0] == b.bounds[2] - b.bounds[0];
                let depth = a.bounds[3] - a.bounds[1] == b.bounds[3] - b.bounds[1];
                if equal && (!width || !depth) {
                    continue;
                }
                if (!equal || axis)
                    && depth
                    && a.bounds[1] == b.bounds[1]
                    && a.bounds[2] == b.bounds[0]
                {
                    nodes[i].bounds[2] = b.bounds[2];
                } else if (!equal || !axis)
                    && width
                    && a.bounds[0] == b.bounds[0]
                    && a.bounds[3] == b.bounds[1]
                {
                    nodes[i].bounds[3] = b.bounds[3];
                } else {
                    continue;
                }
                nodes[j].division = 0;
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
}
