use crate::math::Position;
use crate::{
    Result,
    camera::Camera,
    math::{Math, Matrix},
    packets,
    scene::{ModelId, Scene},
};
use tenchu_assets::{model::Geometry, stage::MissionAssets};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub enum Mesh {
    Object(usize),
    Map(usize),
}

#[derive(Clone, Debug)]
pub struct Object {
    pub model: ModelId,
    pub mesh: Mesh,
    pub size: i16,
    pub shift_y: i16,
    pub subdivisions: u8,
}

#[derive(Debug)]
pub struct Construction {
    pub objects: Vec<Object>,
    cells: Vec<Vec<usize>>,
}

fn cell(value: i32) -> i32 {
    value / 16000 - i32::from(value < 0)
}

fn index(Position { x, y, z }: Position) -> usize {
    ((x & 7) * 64 + (y & 7) * 8 + (z & 7)) as usize
}

pub fn visible(
    position: Position,
    radius: i32,
    camera: Position,
    rotation: [[i16; 3]; 3],
) -> Option<Position> {
    let delta: Position = position.wrapping_sub(camera);
    if delta.iter().any(|v| v.wrapping_abs() > 30000) {
        return None;
    }
    let point = Matrix {
        rotation,
        translation: Position::ZERO,
    }
    .transform(delta.map(|v| v as i16).to_array());
    let far = point[2].wrapping_add(radius);
    if far <= 150 || point[2].wrapping_sub(radius) > 17000 {
        return None;
    }
    let project = |v: i32| v.wrapping_mul(300) / far;
    let size = project(radius);
    (project(point[0]).wrapping_abs() <= size.wrapping_add(160)
        && project(point[1]).wrapping_abs() <= size.wrapping_add(120))
    .then_some(point)
}

impl Construction {
    pub fn new(mission: &MissionAssets, scene: &mut Scene, math: &Math) -> Result<Self> {
        let mut out = Self {
            objects: Vec::new(),
            cells: vec![Vec::new(); 512],
        };
        let root = scene.add(Some(0), Matrix::default());
        let originals: Vec<_> = mission
            .objects
            .iter()
            .map(|part| {
                scene.add(
                    Some(root),
                    Matrix {
                        translation: part.position.map(i32::from).into(),
                        ..Matrix::default()
                    },
                )
            })
            .collect();
        for (i, part) in mission.objects.iter().enumerate() {
            if part.parent >= 0
                && let Some(parent) = mission.objects.iter().position(|p| p.child == part.parent)
            {
                scene.models[originals[i]].parent = Some(originals[parent]);
            }
        }
        for placement in &mission.placements {
            let mesh = mission.objects[placement.part]
                .geometry
                .first()
                .ok_or("Empty construction object")?;
            let mut low = [0i16; 3];
            let mut high = [0i16; 3];
            for v in &mesh.vertices {
                for i in 0..3 {
                    low[i] = low[i].min(v[i]);
                    high[i] = high[i].max(v[i]);
                }
            }
            let size = (0..3).map(|i| high[i].wrapping_sub(low[i])).max().unwrap() / 2 / 2;
            let shift_y = ((i32::from(high[1]) + i32::from(low[1])) / 2) as i16;
            let matrix = Matrix {
                rotation: math.rotate_yxz([0, placement.facing as i16, 0]),
                translation: placement.position.into(),
            };
            let model = if placement.cloned {
                scene.add(Some(0), matrix)
            } else {
                let id = originals[placement.part];
                scene.update(id, matrix);
                id
            };
            out.insert(
                scene,
                Object {
                    model,
                    mesh: Mesh::Object(placement.part),
                    size,
                    shift_y,
                    subdivisions: if placement.cloned { 0 } else { 2 },
                },
            );
        }
        let root = scene.add(Some(0), Matrix::default());
        for (i, part) in mission.map.iter().enumerate() {
            let model = scene.add(
                Some(root),
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Matrix::default()
                },
            );
            out.insert(
                scene,
                Object {
                    model,
                    mesh: Mesh::Map(i),
                    size: (-i32::from(part.parent) * 14) as i16,
                    shift_y: 0,
                    subdivisions: 2,
                },
            );
        }
        Ok(out)
    }

    fn insert(&mut self, scene: &Scene, object: Object) {
        self.cells[index(scene.models[object.model].local.translation.map(cell))]
            .push(self.objects.len());
        self.objects.push(object);
    }

    pub fn geometry<'a>(
        &self,
        object: &Object,
        mission: &'a MissionAssets,
    ) -> Result<&'a Geometry> {
        let part = match object.mesh {
            Mesh::Object(i) => &mission.objects[i],
            Mesh::Map(i) => &mission.map[i],
        };
        part.geometry
            .first()
            .ok_or_else(|| "Empty construction geometry".into())
    }

    pub fn select(&self, scene: &Scene, camera: &Camera) -> Result<Vec<Vec<usize>>> {
        let center = camera.position.map(cell);
        if center.iter().any(|v| !(-32765..32765).contains(v)) {
            return Err("Construction scan overflows its source iterator".into());
        }
        let mut buckets = vec![Vec::new(); 153];
        let mut count = 0;
        for x in center[0] - 2..=center[0] + 2 {
            for y in center[1] - 1..=center[1] + 1 {
                for z in center[2] - 2..=center[2] + 2 {
                    if visible(
                        [x, y, z].map(|v| v * 16000 + 8000).into(),
                        0x2bc1,
                        camera.position,
                        camera.view.rotation,
                    )
                    .is_none()
                    {
                        continue;
                    }
                    for &id in self.cells[index(Position::new(x, y, z))].iter().rev() {
                        let object = &self.objects[id];
                        let mut position = scene.models[object.model].local.translation;
                        position[1] = position[1].wrapping_add(i32::from(object.shift_y));
                        if let Some(view) = visible(
                            position,
                            i32::from(object.size),
                            camera.position,
                            camera.view.rotation,
                        ) {
                            let bucket = ((view[2].wrapping_sub(i32::from(object.size)) >> 8) - 11)
                                .max(0) as usize;
                            if bucket >= buckets.len() || count == 100 {
                                return Err("Construction drawing list overflow".into());
                            }
                            buckets[bucket].push(id);
                            count += 1;
                        }
                    }
                }
            }
        }
        for bucket in &mut buckets {
            bucket.reverse();
        }
        Ok(buckets)
    }

    pub fn packets(
        &self,
        mission: &MissionAssets,
        scene: &mut Scene,
        camera: &Camera,
        subdivision: &packets::Subdivision,
    ) -> Result<u32> {
        let buckets = self.select(scene, camera)?;
        let mut bytes = 0;
        for (bucket, objects) in buckets.iter().enumerate() {
            for &id in objects {
                let object = &self.objects[id];
                let matrix = camera.view.compose(&scene.world(object.model)?);
                let mesh = self.geometry(object, mission)?;
                bytes += if bucket != 0 {
                    packets::draw_tmd(mesh, &matrix, true)
                } else {
                    if object.subdivisions != 0 {
                        subdivision.count(mesh, &matrix, object.subdivisions)?
                    } else {
                        packets::sdk_plain(mesh, &matrix)?
                    }
                };
                if bucket != 0 && bytes > 25600 {
                    return Ok(bytes);
                }
            }
        }
        Ok(bytes)
    }
}
