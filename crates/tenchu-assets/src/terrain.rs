use crate::{Result, i16_le, slice, u16_le, u32_le};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub height: i16,
    pub bounds: [i16; 4],
    pub count: usize,
    pub list: usize,
    pub lookup: Option<usize>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AreaMap {
    data: Vec<u8>,
    pub rows: Vec<Row>,
    pub grid: Option<Grid>,
}

#[derive(Eq, PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Grid {
    pub origin: [i16; 2],
    pub dimensions: [usize; 2],
    pub cell_size: i16,
}

impl Grid {
    pub fn cell(&self, x: i32, z: i32) -> Option<usize> {
        let [x, z] = [x - i32::from(self.origin[0]), z - i32::from(self.origin[1])];
        if x < 0 || z < 0 || self.cell_size <= 0 {
            return None;
        }
        let [x, z] = [x, z].map(|v| (v / i32::from(self.cell_size)) as usize);
        (x < self.dimensions[0] && z < self.dimensions[1]).then_some(z * self.dimensions[0] + x)
    }
}

impl AreaMap {
    pub fn from_grid(grid: Grid, cells: Vec<Vec<Node>>) -> Result<Self> {
        if grid.cell_size <= 0
            || grid.dimensions.contains(&0)
            || grid.dimensions.iter().any(|&n| n > 64)
            || grid.dimensions[0] * grid.dimensions[1] != cells.len()
        {
            return Err("Invalid terrain cell grid".into());
        }
        for nodes in &cells {
            if nodes.len() > i16::MAX as usize {
                return Err("Excessive terrain cell".into());
            }
        }
        let mut map = Self::from_nodes(
            cells
                .into_iter()
                .map(|nodes| (i16::MAX - 2, nodes))
                .collect(),
        )?;
        for (i, row) in map.rows.iter_mut().enumerate() {
            let x = i32::from(grid.origin[0])
                + (i % grid.dimensions[0]) as i32 * i32::from(grid.cell_size);
            let z = i32::from(grid.origin[1])
                + (i / grid.dimensions[0]) as i32 * i32::from(grid.cell_size);
            for (i, bound) in [
                x,
                z,
                x + i32::from(grid.cell_size) - 1,
                z + i32::from(grid.cell_size) - 1,
            ]
            .into_iter()
            .enumerate()
            {
                row.bounds[i] = bound
                    .try_into()
                    .map_err(|_| "Terrain grid exceeds coordinate range")?;
            }
        }
        map.grid = Some(grid);
        Ok(map)
    }

    pub fn from_nodes(levels: Vec<(i16, Vec<Node>)>) -> Result<Self> {
        let mut data = vec![0; (levels.len() + 1) * 16];
        for (index, (height, nodes)) in levels.into_iter().enumerate() {
            let mut bounds = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
            let at = index * 16;
            data[at..at + 2].copy_from_slice(&height.to_le_bytes());
            data[at + 2..at + 4].copy_from_slice(&(nodes.len() as i16).to_le_bytes());
            let offset = data.len() as u32;
            data[at + 4..at + 8].copy_from_slice(&offset.to_le_bytes());
            for node in nodes {
                for i in 0..2 {
                    bounds[i] = bounds[i].min(node.bounds[i]);
                    bounds[i + 2] = bounds[i + 2].max(node.bounds[i + 2]);
                }
                data.extend(node.y.to_le_bytes());
                data.extend(node.dy.to_le_bytes());
                for value in node.bounds {
                    data.extend(value.to_le_bytes());
                }
                data.extend(node.attributes.to_le_bytes());
                data.extend(node.division.to_le_bytes());
            }
            for (i, value) in bounds.into_iter().enumerate() {
                data[at + 8 + i * 2..at + 10 + i * 2].copy_from_slice(&value.to_le_bytes());
            }
        }
        Self::parse(data)
    }

    pub fn set_surface(&mut self, offset: usize, y: i16, division: u16) -> Result<()> {
        slice(&self.data, offset, 16)?;
        self.data[offset..offset + 2].copy_from_slice(&y.to_le_bytes());
        self.data[offset + 14..offset + 16].copy_from_slice(&division.to_le_bytes());
        Ok(())
    }

    pub fn parse(mut data: Vec<u8>) -> Result<Self> {
        let mut rows = Vec::new();
        for i in 0..4096 {
            let at = i * 16;
            let record = slice(&data, at, 16)?;
            let offset = u32_le(record, 4)? as usize;
            if offset == 0 {
                return Ok(Self {
                    data,
                    rows,
                    grid: None,
                });
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
        let row: &[u8; 16] = slice(&self.data, offset, 16)?.try_into().unwrap();
        let [y, dy, x1, z1, x2, z2, attributes, division] =
            std::array::from_fn(|i| u16::from_le_bytes([row[i * 2], row[i * 2 + 1]]));
        Ok(Node {
            y: y as i16,
            dy: dy as i16,
            bounds: [x1 as i16, z1 as i16, x2 as i16, z2 as i16],
            attributes,
            division,
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
        let index = i64::from(qz) * 4 + i64::from(qx);
        // Retail can read neighboring pointers at inclusive row edges. Treat
        // entries outside the table as empty instead of using pointer bits.
        if !(0..16).contains(&index) {
            return Ok(row.count);
        }
        let at = lookup as i64 + 4 + index * 2;
        Ok(u16_le(
            &self.data,
            usize::try_from(at).map_err(|_| "ACM lookup before file")?,
        )? as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_edge_ignores_bytes_after_the_lookup_table() {
        let row = Row {
            height: 0,
            bounds: [0, 0, 400, 400],
            count: 64,
            list: 128,
            lookup: Some(0),
        };
        let mut map = AreaMap {
            data: vec![0; 36],
            rows: vec![row.clone()],
            grid: None,
        };
        for index in 0..16u16 {
            let at = 4 + usize::from(index) * 2;
            map.data[at..at + 2].copy_from_slice(&index.to_le_bytes());
        }
        assert_eq!(map.first_node(&row, 399, 399).unwrap(), 15);
        // The original flattened lookup also permits an X-edge alias within
        // the same table; only reads beyond that table are rejected.
        assert_eq!(map.first_node(&row, 400, 0).unwrap(), 4);
        for tail in [Vec::new(), vec![0; 10], vec![0x1a, 0x80, 0xff, 0xff, 0, 0]] {
            map.data.truncate(36);
            map.data.extend(tail);
            for x in [0, 100, 200, 300, 400] {
                assert_eq!(map.first_node(&row, x, 400).unwrap(), row.count);
            }
            assert_eq!(map.first_node(&row, 400, 399).unwrap(), row.count);
        }
    }

    #[test]
    fn unaligned_nodes_preserve_signed_fields_and_surface_edits() {
        let mut map = AreaMap {
            grid: None,
            data: vec![
                0xaa, 0x00, 0x80, 0xff, 0x7f, 0xff, 0xff, 0x00, 0x00, 0xd2, 0x04, 0x00, 0xf8, 0x23,
                0x81, 0xdc, 0xfe,
            ],
            rows: Vec::new(),
        };
        let mut expected = Node {
            y: i16::MIN,
            dy: i16::MAX,
            bounds: [-1, 0, 1234, -2048],
            attributes: 0x8123,
            division: 0xfedc,
        };
        assert_eq!(map.node(1).unwrap(), expected);
        map.set_surface(1, -17, 0x0401).unwrap();
        expected.y = -17;
        expected.division = 0x0401;
        assert_eq!(map.node(1).unwrap(), expected);
        for offset in [2, usize::MAX] {
            assert!(map.node(offset).is_err());
            assert!(map.set_surface(offset, 0, 0).is_err());
        }
    }
}
