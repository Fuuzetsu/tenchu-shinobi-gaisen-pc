use super::{model, volume::Volume};
use crate::{ReadAt, Result, disc::Disc, model::Part, texture::Vram, u32_le};

pub mod background;
pub mod grid;
pub mod objects;
mod shading;
use objects::{Objects, Variant};
pub use shading::Shading;

#[derive(Clone)]
pub struct Level {
    pub id: usize,
    pub gaisen: Option<crate::game::Stage>,
    pub draw_distance: i32,
    pub grid: grid::Grid,
    /// MDL corner colors retain shade indices for the stage's distance palette.
    pub map: Vec<Part>,
    pub objects: Vec<Part>,
    pub object_sizes: Vec<i32>,
    pub stumps: std::collections::BTreeMap<usize, Part>,
    pub scene: Objects,
    pub vram: Vram,
    pub collision: super::collision::Collision,
    pub background: background::Background,
    pub shading: Shading,
}

impl Level {
    pub fn draw_distance(disc: &Disc<impl ReadAt>, id: usize) -> Result<i32> {
        Self::parameters(disc, id).map(|(distance, _)| distance)
    }

    fn parameters(disc: &Disc<impl ReadAt>, id: usize) -> Result<(i32, u32)> {
        if id >= 24 {
            return Err("Tenchu 2 map index must be 0–23".into());
        }
        let (edition, bytes) = super::executable(disc)?;
        let executable = crate::executable::Executable::new(&bytes)?;
        let bytes = executable.bytes(edition.addresses().stages + id as u32 * 20, 8)?;
        let distance = u32_le(bytes, 0)? as i32;
        if !(1..=32767).contains(&distance) {
            return Err("Invalid Tenchu 2 draw distance".into());
        }
        Ok((distance, u32_le(bytes, 4)?))
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
        Self::load_variant(disc, id, Variant::Base, None)
    }

    pub fn japanese_sequence(
        disc: &Disc<impl ReadAt>,
        layout: &super::layout::Layout,
        hidden: bool,
    ) -> Result<Self> {
        Self::japanese_scene(disc, layout.map, layout.sequence, hidden)
    }

    pub fn japanese_scene(
        disc: &Disc<impl ReadAt>,
        map: usize,
        sequence: usize,
        hidden: bool,
    ) -> Result<Self> {
        if super::Edition::detect(disc)? != super::Edition::Japan {
            return Err("Japanese Tenchu 2 scene selection requires the Japanese disc".into());
        }
        Self::load_variant(
            disc,
            map,
            Variant::japanese(map, sequence, hidden),
            Some(sequence),
        )
    }

    fn load_variant(
        disc: &Disc<impl ReadAt>,
        id: usize,
        variant: Variant,
        sequence: Option<usize>,
    ) -> Result<Self> {
        let (draw_distance, flags) = Self::parameters(disc, id)?;
        let volume = Volume::open(disc)?;
        let directory = directory(id);
        let shading = Shading::load(&volume, &directory)?;
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
        upload_textures(&read("mapTEX.arc")?, &mut vram)?;
        let background = background::Background::load(&volume, id, sequence, flags, &mut vram)?;
        initialize_water(id, &mut vram)?;
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
        let mut stumps = std::collections::BTreeMap::new();
        for object in scene.dynamic.iter().filter(|o| o.kind == 14) {
            if let std::collections::btree_map::Entry::Vacant(entry) = stumps.entry(object.model) {
                entry.insert(Part {
                    parent: -1,
                    child: -1,
                    position: [0; 3],
                    geometry: vec![model::stump(crate::model::arc_entry(
                        &archive,
                        object.model,
                    )?)?],
                });
            }
        }
        model::resolve_transparency(stumps.values_mut(), &vram);
        let collision = match sequence {
            Some(179 | 216) => "map2.mcd",
            Some(245) => "map3.mcd",
            _ => "map.mcd",
        };
        Ok(Self {
            id,
            gaisen: None,
            draw_distance,
            grid,
            map,
            objects,
            object_sizes,
            stumps,
            scene,
            vram,
            collision: super::collision::Collision::parse(&read(collision)?)?,
            background,
            shading,
        })
    }
}

fn upload_textures(archive: &[u8], vram: &mut Vram) -> Result<()> {
    let count = u32_le(archive, 0)? as usize;
    if count > 4096 {
        return Err("Excessive Tenchu 2 texture count".into());
    }
    for i in 0..count {
        super::texture::upload(vram, crate::model::arc_entry(archive, i)?)?;
    }
    Ok(())
}

fn directory(id: usize) -> String {
    format!(r"o:\cdimage\game\stage\map{id}")
}

fn initialize_water(id: usize, vram: &mut Vram) -> Result<()> {
    let speeds = match id {
        0 | 17 => [-4, -8],
        9 | 19 => [-4, -12],
        16 => [-4, -4],
        18 => [-1, -3],
        _ => return Ok(()),
    };
    for (layer, speed) in speeds.into_iter().enumerate() {
        let rect = [384, layer as u16 * 64, 16, 64];
        let image = vram
            .images
            .iter()
            .copied()
            .find(|image| image.word_rect() == rect)
            .ok_or("Missing Tenchu 2 scrolling water texture")?;
        vram.copy_rect(rect, [960 + layer as u16 * 16, 256])?;
        vram.animations.push(crate::texture::TextureScroll {
            image,
            velocity: [0, speed],
            delay: 5,
            water: true,
        });
    }
    Ok(())
}
