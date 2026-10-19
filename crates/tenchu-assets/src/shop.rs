use crate::{Result, executable::Executable, u32_le};

pub const ITEMS: usize = 20;

pub struct Shop {
    pub order: [usize; ITEMS - 1],
    pub limits: [u8; ITEMS],
}

impl Shop {
    pub fn load(executable: &Executable) -> Result<Self> {
        let mut shop = Self {
            order: [0; ITEMS - 1],
            limits: [255; ITEMS],
        };
        let mut seen = [false; ITEMS];
        for (i, row) in executable
            .bytes(0x8008edfc, (ITEMS - 1) * 12)?
            .chunks_exact(12)
            .enumerate()
        {
            let item = u32_le(row, 4)? as usize;
            if item == 0 || item >= ITEMS || seen[item] {
                return Err("Invalid inventory shop table".into());
            }
            seen[item] = true;
            shop.order[i] = item;
            shop.limits[item] = row[8];
        }
        Ok(shop)
    }
}
