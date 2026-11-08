use super::{model, volume::Volume};
use crate::{
    ReadAt, Result, disc::Disc, model::Part, short3, slice, texture::Vram, u16_le, u32_le,
};

#[derive(Clone)]
pub struct Level {
    pub id: usize,
    pub map: Vec<Part>,
    pub objects: Vec<Part>,
    pub placements: Vec<Object>,
    pub vram: Vram,
    pub collision: super::collision::Collision,
    pub omitted_effects: usize,
}

#[derive(Clone)]
pub struct Object {
    pub part: usize,
    pub position: [i32; 3],
    pub rotation: [i16; 3],
}

impl Level {
    pub fn draw_distance(disc: &Disc<impl ReadAt>, id: usize) -> Result<i32> {
        if id >= 24 {
            return Err("Tenchu 2 map index must be 0–23".into());
        }
        let (edition, bytes) = super::executable(disc)?;
        let executable = crate::executable::Executable::new(&bytes)?;
        let bytes = executable.bytes(edition.addresses().stages + id as u32 * 20, 4)?;
        let distance = u32_le(bytes, 0)? as i32;
        if !(1..=32767).contains(&distance) {
            return Err("Invalid Tenchu 2 draw distance".into());
        }
        Ok(distance)
    }

    pub fn available(disc: &Disc<impl ReadAt>) -> Result<Vec<usize>> {
        let volume = Volume::open(disc)?;
        (0..24)
            .filter_map(|id| match volume.optional(&directory(id), "map.msd") {
                Ok(Some(data)) if data.len() >= 12 => Some(Ok(id)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    pub fn load(disc: &Disc<impl ReadAt>, id: usize) -> Result<Self> {
        if id >= 24 {
            return Err("Tenchu 2 map index must be 0–23".into());
        }
        let volume = Volume::open(disc)?;
        let directory = directory(id);
        let read = |name: &str| volume.read(&directory, name);
        let msd = read("map.msd")?;
        let origin = short3(&msd, 0)?.map(i32::from);
        let dimensions = short3(&msd, 6)?;
        if dimensions.iter().any(|n| !(1..=64).contains(n)) {
            return Err("Invalid Tenchu 2 map dimensions".into());
        }
        let [nx, ny, nz] = dimensions.map(|v| v as usize);
        let cells = slice(&msd, 12, nx * ny * nz * 4)?;
        let mut map = Vec::new();
        for (index, row) in cells.chunks_exact(4).enumerate() {
            if u32_le(row, 0)? == 0 {
                continue;
            }
            let x = (index / (ny * nz)) as i32;
            let z = ((index / ny) % nz) as i32;
            let y = (index % ny) as i32;
            let height = if id == 4 { 6 } else { 16 };
            let center = [
                origin[0] + x * 16 + 8,
                origin[1] - y * height - height / 2,
                origin[2] + z * 16 + 8,
            ];
            let mut position = [0; 3];
            for axis in 0..3 {
                position[axis] = (center[axis] * 100)
                    .try_into()
                    .map_err(|_| "Tenchu 2 map exceeds world bounds")?;
            }
            let geometry = model::scenery(&read(&format!("map{index:04}.mdl"))?)?;
            map.push(Part {
                parent: -1000,
                child: -1,
                position,
                geometry: vec![geometry],
            });
        }
        if map.is_empty() {
            return Err("Tenchu 2 map has no geometry".into());
        }
        let mut vram = Vram::default();
        let textures = read("mapTEX.arc")?;
        let count = u32_le(&textures, 0)? as usize;
        if count > 4096 {
            return Err("Excessive Tenchu 2 texture count".into());
        }
        for i in 0..count {
            vram.upload_tim(crate::model::arc_entry(&textures, i)?)?;
        }
        let archive = read("mapOBJ.arc")?;
        let count = u32_le(&archive, 0)? as usize;
        if count > 1024 {
            return Err("Excessive Tenchu 2 object count".into());
        }
        let mut objects = Vec::new();
        for i in 0..count {
            objects.push(Part {
                parent: -1,
                child: -1,
                position: [0; 3],
                geometry: vec![model::scenery(crate::model::arc_entry(&archive, i)?)?],
            });
        }
        model::resolve_transparency(map.iter_mut().chain(&mut objects), &vram);
        let osd = read("map.osd")?;
        if usize::from(u16_le(&osd, 6)?) != objects.len() {
            return Err("Tenchu 2 object table mismatch".into());
        }
        let count = usize::from(u16_le(&osd, 0)?);
        let mut placements = Vec::new();
        let mut omitted_effects = usize::from(u16_le(&osd, 2)?) + usize::from(u16_le(&osd, 4)?);
        for row in slice(&osd, 12 + objects.len() * 12, count * 12)?.chunks_exact(12) {
            // Type zero invokes procedural effects, not an entry in mapOBJ.arc.
            if u16_le(row, 8)? == 0 {
                omitted_effects += 1;
                continue;
            }
            let part = usize::from(u16_le(row, 10)?);
            if part >= objects.len() {
                return Err("Tenchu 2 placement references a missing object".into());
            }
            placements.push(Object {
                part,
                position: short3(row, 0)?.map(|v| i32::from(v) * 10),
                rotation: [i16::from(row[7]) * 16, i16::from(row[6]) * 16, 0],
            });
        }
        Ok(Self {
            id,
            map,
            objects,
            placements,
            vram,
            collision: super::collision::Collision::parse(&read("map.mcd")?)?,
            omitted_effects,
        })
    }
}

fn directory(id: usize) -> String {
    format!(r"o:\cdimage\game\stage\map{id}")
}
