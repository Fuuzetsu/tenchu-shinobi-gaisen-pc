use crate::{Result, i16_le, slice, u16_le, u32_le};

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub y: i16,
    pub dy: i16,
    pub bounds: [i16; 4],
    pub attributes: u16,
    pub division: u16,
}

impl Node {
    pub fn contains(&self, x: i32, z: i32) -> bool {
        let [x1, z1, x2, z2] = self.bounds.map(i32::from);
        x1 <= x && x <= x2 && z1 <= z && z <= z2
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub height: i16,
    pub bounds: [i16; 4],
    pub count: usize,
    pub list: usize,
    pub lookup: Option<usize>,
}

pub struct AreaMap {
    data: Vec<u8>,
    pub rows: Vec<Row>,
}

impl AreaMap {
    pub fn parse(mut data: Vec<u8>) -> Result<Self> {
        let mut rows = Vec::new();
        for i in 0..4096 {
            let at = i * 16;
            let record = slice(&data, at, 16)?;
            let offset = u32_le(record, 4)? as usize;
            if offset == 0 {
                return Ok(Self { data, rows });
            }
            let count = i16_le(record, 2)?;
            let lookup = (count < 0).then_some(offset);
            let list = if count < 0 {
                u32_le(&data, offset)? as usize
            } else {
                offset
            };
            let count = i32::from(count).unsigned_abs() as usize;
            slice(&data, list, count * 16)?;
            if let Some(lookup) = lookup {
                slice(&data, lookup, 36)?;
            }
            let height = i16_le(record, 0)?.wrapping_add(2);
            let bounds = [
                i16_le(record, 8)?,
                i16_le(record, 10)?,
                i16_le(record, 12)?,
                i16_le(record, 14)?,
            ];
            rows.push(Row {
                height,
                bounds,
                count,
                list,
                lookup,
            });
            data[at..at + 2].copy_from_slice(&height.to_le_bytes());
        }
        Err("Unterminated ACM row table".into())
    }

    pub fn node(&self, offset: usize) -> Result<Node> {
        let row = slice(&self.data, offset, 16)?;
        Ok(Node {
            y: i16_le(row, 0)?,
            dy: i16_le(row, 2)?,
            bounds: [
                i16_le(row, 4)?,
                i16_le(row, 6)?,
                i16_le(row, 8)?,
                i16_le(row, 10)?,
            ],
            attributes: u16_le(row, 12)?,
            division: u16_le(row, 14)?,
        })
    }

    pub fn first_node(&self, row: &Row, x: i32, z: i32) -> Result<usize> {
        let Some(lookup) = row.lookup else {
            return Ok(0);
        };
        let [x1, z1, x2, z2] = row.bounds.map(i32::from);
        if x1 == x2 || z1 == z2 {
            return Err("Zero ACM index span".into());
        }
        let qx = ((x - x1) * 4 / (x2 - x1)) as i16;
        let qz = ((z - z1) * 4 / (z2 - z1)) as i16;
        let at = lookup as i64 + 4 + (i64::from(qz) * 4 + i64::from(qx)) * 2;
        // Inclusive row edges can read past the nominal 4×4 lookup table.
        Ok(u16_le(
            &self.data,
            usize::try_from(at).map_err(|_| "ACM lookup before file")?,
        )? as usize)
    }
}
