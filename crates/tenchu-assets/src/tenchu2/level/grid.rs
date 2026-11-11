use crate::{Result, short3, slice, u16_le};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Grid {
    pub origin: [i16; 3],
    pub dimensions: [i16; 3],
    pub height: i32,
    pub cells: Vec<Option<usize>>,
}

impl Grid {
    pub fn parse(bytes: &[u8], map: usize) -> Result<Self> {
        let origin = short3(bytes, 0)?;
        let dimensions = short3(bytes, 6)?;
        if dimensions.iter().any(|n| !(1..=64).contains(n)) {
            return Err("Invalid Tenchu 2 map dimensions".into());
        }
        let count: usize = dimensions.iter().map(|&n| n as usize).product();
        let mut next = 0;
        let cells = slice(bytes, 12, count * 4)?
            .chunks_exact(4)
            .map(|cell| {
                Ok((u16_le(cell, 0)? != 0).then(|| {
                    let index = next;
                    next += 1;
                    index
                }))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            origin,
            dimensions,
            height: if map == 4 { 6 } else { 16 },
            cells,
        })
    }

    pub fn center(&self, index: usize) -> [i32; 3] {
        let [_, ny, nz] = self.dimensions.map(i32::from);
        let index = index as i32;
        let [x, y, z] = self.origin.map(i32::from);
        [
            x + (index / (ny * nz)) * 16 + 8,
            y - (index % ny) * self.height - self.height / 2,
            z + ((index / ny) % nz) * 16 + 8,
        ]
    }

    pub fn object_cell(&self, position: [i32; 3]) -> Result<usize> {
        let [_, ny, nz] = self.dimensions.map(i32::from);
        let [x, y, z] = std::array::from_fn(|i| position[i] / 1000);
        let x = ((x - i32::from(self.origin[0])) >> 4) as i16;
        let z = ((z - i32::from(self.origin[2])) >> 4) as i16;
        let y = i32::from(self.origin[1]) - y;
        let y = if self.height == 6 { y / 6 } else { y >> 4 };
        let index = (y + (i32::from(x) * nz + i32::from(z)) * ny) as i16;
        let index =
            usize::try_from(index).map_err(|_| "Tenchu 2 object is outside its map grid")?;
        let cell = self
            .cells
            .get(index)
            .ok_or("Tenchu 2 object is outside its map grid")?;
        Ok(if cell.is_some() {
            index
        } else if index == 0 || (y as i16) < 1 {
            0
        } else {
            index - 1
        })
    }

    pub fn contains(&self, index: usize) -> bool {
        self.cells.get(index).is_some_and(Option::is_some)
    }
}
