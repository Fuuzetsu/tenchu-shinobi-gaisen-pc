use crate::{Result, i16_le, slice, terrain::Node, u16_le, u32_le};

#[derive(Clone)]
pub struct Collision {
    pub origin: [i16; 2],
    pub dimensions: [usize; 2],
    pub cell_size: i16,
    pub cells: Vec<Vec<Node>>,
}

impl Collision {
    pub fn into_terrain(self) -> Result<crate::terrain::AreaMap> {
        let cells = self
            .cells
            .into_iter()
            .map(|nodes| {
                nodes
                    .into_iter()
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
                if bounds[0] == bounds[2] || bounds[1] == bounds[3] {
                    continue;
                }
                nodes.push(Node {
                    y: i16_le(node, 0)?,
                    dy: i16_le(node, 2)?,
                    bounds,
                    attributes: u16_le(node, 12)?,
                    division: u16_le(node, 14)?,
                });
            }
            cells.push(nodes);
        }
        Ok(Self {
            origin,
            dimensions,
            cell_size,
            cells,
        })
    }
}
