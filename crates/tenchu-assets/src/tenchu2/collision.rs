use crate::{Result, i16_le, slice, terrain::Node, u16_le, u32_le};

mod gaisen;
pub use gaisen::Gaisen;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Rules {
    pub wall_angles: [Option<i16>; 16],
    pub escape: [[i16; 2]; 16],
    pub corners: [[i16; 2]; 4],
    pub blocked: [[bool; 4]; 4],
}

impl Rules {
    pub fn load(disc: &crate::disc::Disc<impl crate::ReadAt>) -> Result<Self> {
        let (edition, data) = super::executable(disc)?;
        let executable = crate::executable::Executable::new(&data)?;
        let bytes = executable.bytes(edition.addresses().collision_rules, 128)?;
        let short = |at| i16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let pair = |at| [short(at), short(at + 2)];
        Ok(Self {
            wall_angles: std::array::from_fn(|i| {
                let value = short(i * 2);
                (value != -1).then_some(value)
            }),
            escape: std::array::from_fn(|i| pair(32 + i * 4)),
            corners: std::array::from_fn(|i| pair(96 + i * 4)),
            blocked: std::array::from_fn(|i| std::array::from_fn(|j| bytes[112 + i * 4 + j] != 0)),
        })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Collision {
    pub origin: [i16; 2],
    pub dimensions: [usize; 2],
    pub cell_size: i16,
    pub cells: Vec<Cell>,
    #[serde(default)]
    pub gaisen: Option<Gaisen>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub minimum: i16,
    pub maximum: i16,
    pub starts: [usize; 4],
    pub nodes: Vec<Node>,
}

impl Collision {
    pub fn into_terrain(self) -> Result<crate::terrain::AreaMap> {
        if let Some(source) = self.gaisen {
            return Ok(source.map);
        }
        let cells = self
            .cells
            .into_iter()
            .map(|cell| {
                cell.nodes
                    .into_iter()
                    .filter(|node| {
                        node.bounds[0] < node.bounds[2] && node.bounds[1] < node.bounds[3]
                    })
                    .map(|mut node| {
                        // Preserve MCD's ordered blockers, including flat masked holes.
                        node.bounds[2] -= 1;
                        node.bounds[3] -= 1;
                        let blocked = node.attributes & 0xf00 == 0x100;
                        node.attributes = 1
                            | if blocked { 2 } else { 0 }
                            | match node.attributes & 0xc000 {
                                0 => 0x2000,
                                0x8000 => 0x4000,
                                0xc000 => 0x8000,
                                _ => 0,
                            };
                        node
                    })
                    .collect()
            })
            .collect();
        crate::terrain::AreaMap::from_grid(
            crate::terrain::Grid {
                origin: self.origin,
                dimensions: self.dimensions,
                cell_size: self.cell_size,
            },
            cells,
        )
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if u32_le(data, 0)? >> 24 != 0xa0 {
            return Err("Invalid Tenchu 2 MCD".into());
        }
        let origin = [i16_le(data, 4)?, i16_le(data, 6)?];
        let dimensions = [i16_le(data, 8)?, i16_le(data, 10)?];
        let cell_size = i16_le(data, 12)?;
        if dimensions.iter().any(|n| !(1..=64).contains(n)) || cell_size <= 0 {
            return Err("Invalid Tenchu 2 collision grid".into());
        }
        let dimensions = dimensions.map(|v| v as usize);
        let count = dimensions[0] * dimensions[1];
        if usize::from(u16_le(data, 14)?) != count {
            return Err("Tenchu 2 collision cell count mismatch".into());
        }
        let mut cells = Vec::with_capacity(count);
        for row in slice(data, 16, count * 16)?.chunks_exact(16) {
            let count = usize::from(u16_le(row, 4)?);
            if count > 8192 {
                return Err("Excessive Tenchu 2 collision cell".into());
            }
            let offset = u32_le(row, 12)? as usize;
            let mut nodes = Vec::with_capacity(count);
            for node in slice(data, offset, count * 16)?.chunks_exact(16) {
                let bounds = [
                    i16_le(node, 4)?,
                    i16_le(node, 6)?,
                    i16_le(node, 8)?,
                    i16_le(node, 10)?,
                ];
                if bounds[0] > bounds[2] || bounds[1] > bounds[3] {
                    return Err("Reversed Tenchu 2 collision surface".into());
                }
                nodes.push(Node {
                    y: i16_le(node, 0)?,
                    dy: i16_le(node, 2)?,
                    bounds,
                    attributes: u16_le(node, 12)?,
                    division: u16_le(node, 14)?,
                });
            }
            let starts = [
                0,
                usize::from(u16_le(row, 6)?),
                usize::from(u16_le(row, 8)?),
                usize::from(u16_le(row, 10)?),
            ];
            if starts.iter().any(|&start| start > count) {
                return Err("Invalid Tenchu 2 collision height index".into());
            }
            cells.push(Cell {
                minimum: i16_le(row, 0)?,
                maximum: i16_le(row, 2)?,
                starts,
                nodes,
            });
        }
        Ok(Self {
            origin,
            dimensions,
            cell_size,
            cells,
            gaisen: None,
        })
    }
}
