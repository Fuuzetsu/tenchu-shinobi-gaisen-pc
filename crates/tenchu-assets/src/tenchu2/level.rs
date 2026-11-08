use super::{model, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, model::Part, texture::Vram, u32_le};

pub mod grid;
pub mod objects;
use objects::{Objects, Variant};

#[derive(Clone)]
pub struct Level {
    pub id: usize,
    pub grid: grid::Grid,
    pub map: Vec<Part>,
    pub objects: Vec<Part>,
    pub object_sizes: Vec<i32>,
    pub scene: Objects,
    pub vram: Vram,
    pub collision: super::collision::Collision,
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
        Self::load_variant(disc, id, Variant::Base)
    }

    pub fn japanese_sequence(
        disc: &Disc<impl ReadAt>,
        layout: &super::layout::Layout,
        hidden: bool,
    ) -> Result<Self> {
        if super::Edition::detect(disc)? != super::Edition::Japan {
            return Err("Japanese Tenchu 2 scene selection requires the Japanese disc".into());
        }
        Self::load_variant(
            disc,
            layout.map,
            Variant::japanese(layout.map, layout.sequence, hidden),
        )
    }

    fn load_variant(disc: &Disc<impl ReadAt>, id: usize, variant: Variant) -> Result<Self> {
        if id >= 24 {
            return Err("Tenchu 2 map index must be 0–23".into());
        }
        let volume = Volume::open(disc)?;
        let directory = directory(id);
        let read = |name: &str| volume.read(&directory, name);
        let msd = read("map.msd")?;
        let grid = grid::Grid::parse(&msd, id)?;
        let mut map = Vec::new();
        for (index, cell) in grid.cells.iter().enumerate() {
            if cell.is_none() {
                continue;
            }
            let center = grid.center(index);
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
        let (layout, archive) = variant.files();
        let archive = read(archive)?;
        let count = u32_le(&archive, 0)? as usize;
        if count > 1024 {
            return Err("Excessive Tenchu 2 object count".into());
        }
        let mut objects = Vec::new();
        let mut object_sizes = Vec::new();
        for i in 0..count {
            let geometry = model::scenery(crate::model::arc_entry(&archive, i)?)?;
            let (low, high) = geometry
                .vertices
                .iter()
                .fold((i16::MAX, i16::MIN), |(low, high), v| {
                    (low.min(v[1]), high.max(v[1]))
                });
            object_sizes.push((i32::from(high) - i32::from(low)) / 2);
            objects.push(Part {
                parent: -1,
                child: -1,
                position: [0; 3],
                geometry: vec![geometry],
            });
        }
        model::resolve_transparency(map.iter_mut().chain(&mut objects), &vram);
        let scene = Objects::parse(&read(layout)?, variant, objects.len())?;
        Ok(Self {
            id,
            grid,
            map,
            objects,
            object_sizes,
            scene,
            vram,
            collision: super::collision::Collision::parse(&read("map.mcd")?)?,
        })
    }
}

fn directory(id: usize) -> String {
    format!(r"o:\cdimage\game\stage\map{id}")
}
