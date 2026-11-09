use super::{Edition, volume::Volume};
use crate::{
    ReadAt, Result, disc::Disc, frontend::Image, i16_le, model::arc_entry, u16_le, u32_le,
};

#[derive(Clone, Debug)]
pub struct Cell {
    pub position: [i16; 2],
    pub neighbours: [Option<usize>; 4],
    pub item: usize,
    pub name: usize,
}

pub struct Rules {
    pub cells: Vec<Cell>,
    pub descriptions: Vec<String>,
    pub unlock: [u16; 17],
    pub replenish: [u16; 7],
}

impl Rules {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        Self::read(&Volume::open(disc)?)
    }

    fn read(volume: &Volume<'_, impl ReadAt>) -> Result<Self> {
        if volume.edition != Edition::Japan {
            return Err("Native Tenchu 2 equipment requires the Japanese disc".into());
        }
        let data = volume.game_module()?;
        let offset = |address: u32| address.wrapping_sub(0x800d0610) as usize;
        let mut cells = Vec::with_capacity(19);
        for index in 0..19 {
            let at = offset(0x800d8c04) + index * 16;
            let mut neighbours = [None; 4];
            for (direction, neighbour) in neighbours.iter_mut().enumerate() {
                let target = i16_le(&data, at + 4 + direction * 2)?;
                if !(-1..19).contains(&target) {
                    return Err("Invalid Japanese equipment neighbour".into());
                }
                *neighbour = (target >= 0).then_some(target as usize);
            }
            let item = usize::from(u16_le(&data, at + 12)?);
            let name = usize::from(u16_le(&data, at + 14)?);
            if item == 0 || item >= 21 || name >= 20 {
                return Err("Invalid Japanese equipment item".into());
            }
            cells.push(Cell {
                position: [i16_le(&data, at)?, i16_le(&data, at + 2)?],
                neighbours,
                item,
                name,
            });
        }
        let mut descriptions = Vec::with_capacity(22);
        for index in 0..22 {
            let at = offset(u32_le(&data, offset(0x800d8bac) + index * 4)?);
            let text = data.get(at..).ok_or("Invalid Japanese equipment text")?;
            let end = text
                .iter()
                .take(256)
                .position(|&c| c == 0)
                .ok_or("Unterminated Japanese equipment text")?;
            let (text, _, malformed) = encoding_rs::SHIFT_JIS.decode(&text[..end]);
            if malformed {
                return Err("Invalid Japanese equipment text encoding".into());
            }
            descriptions.push(text.into_owned());
        }
        Ok(Self {
            cells,
            descriptions,
            unlock: std::array::from_fn(|i| u16_le(&data, offset(0x800d8d34) + i * 2).unwrap()),
            replenish: std::array::from_fn(|i| u16_le(&data, offset(0x800d8d58) + i * 2).unwrap()),
        })
    }
}

pub struct Images {
    pub background: Image,
    pub names: Image,
    pub capacity: Image,
    pub rings: [Image; 3],
    pub items: Vec<Image>,
    pub digits: Image,
}

impl Images {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let volume = Volume::open(disc)?;
        let menu = volume.read(r"o:\cdimage\game\image", "items.arc")?;
        let items = volume.read(r"o:\cdimage\game\item", "item.arc")?;
        let effects = volume.read(r"o:\cdimage\game\effect", "effect.arc")?;
        let image = |data: &[u8], i| super::texture::image(arc_entry(data, i)?);
        let result = Self {
            background: image(&menu, 0)?,
            names: image(&menu, 1)?,
            capacity: image(&menu, 2)?,
            rings: [image(&menu, 4)?, image(&menu, 5)?, image(&menu, 6)?],
            items: (42..63).map(|i| image(&items, i)).collect::<Result<_>>()?,
            digits: image(&effects, 27)?,
        };
        if !result.names.size[0].is_multiple_of(2)
            || !result.names.size[1].is_multiple_of(10)
            || !result.capacity.size[1].is_multiple_of(10)
            || result.digits.size[0] < 40
            || result.digits.size[1] < 5
        {
            return Err("Invalid Japanese equipment image sheets".into());
        }
        Ok(result)
    }
}
