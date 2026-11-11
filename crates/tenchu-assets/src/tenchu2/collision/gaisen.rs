use super::{Cell, Collision};
use crate::{Result, terrain::AreaMap};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Gaisen {
    pub map: AreaMap,
    pub offsets: Vec<Vec<usize>>,
    /// Added underwater collision depth in game units; ACM itself is unchanged.
    #[serde(default)]
    pub water_depth: i16,
}

impl Gaisen {
    pub const DEFAULT_WATER_DEPTH: i16 = 4000;

    pub fn material(attributes: u16) -> u16 {
        let slope = match attributes & 0xc000 {
            0x4000 => 0x8000,
            0x8000 => 0xc000,
            _ => 0x4000,
        };
        slope
            | if attributes & 2 != 0 {
                0x100
            } else if attributes & 4 != 0 {
                0x200
            } else if attributes & 0x200 != 0 {
                0x400
            } else if attributes & 0x100 != 0 {
                0x401
            } else {
                0
            }
    }
}

impl Collision {
    /// Keep ACM's height rows for surface sampling, with MCD rectangles for T2's
    /// probes, ledges and rays. Flattening the rows changes overlapping roofs.
    pub fn from_gaisen(map: &AreaMap) -> Result<Self> {
        if map.grid.is_some() {
            return Err("Expected a Gaisen height-row terrain map".into());
        }
        let mut nodes = Vec::new();
        let mut bounds = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
        for row in &map.rows {
            for i in 0..row.count {
                let mut node = map.node(row.list + i * 16)?;
                for axis in 0..2 {
                    node.bounds[axis + 2] = node.bounds[axis + 2]
                        .checked_add(1)
                        .ok_or("Gaisen terrain exceeds Tenchu 2 coordinate range")?;
                    bounds[axis] = bounds[axis].min(node.bounds[axis]);
                    bounds[axis + 2] = bounds[axis + 2].max(node.bounds[axis + 2]);
                }
                node.attributes = Gaisen::material(node.attributes);
                nodes.push((row.list + i * 16, node));
            }
        }
        if nodes.is_empty() {
            return Err("Empty Gaisen terrain".into());
        }
        let cell_size = 512i16;
        let dimensions = std::array::from_fn(|axis| {
            ((i32::from(bounds[axis + 2]) - i32::from(bounds[axis])) as usize)
                .div_ceil(cell_size as usize)
        });
        if dimensions.iter().any(|&n| n == 0 || n > 64) {
            return Err("Gaisen terrain exceeds Tenchu 2 collision grid".into());
        }
        let mut cells = vec![
            Cell {
                minimum: i16::MIN,
                maximum: i16::MAX,
                starts: [0; 4],
                nodes: Vec::new(),
            };
            dimensions[0] * dimensions[1]
        ];
        let mut offsets = vec![Vec::new(); cells.len()];
        for (offset, node) in nodes {
            let cell = |axis: usize, end: bool| {
                (i32::from(node.bounds[axis + if end { 2 } else { 0 }])
                    - i32::from(bounds[axis])
                    - i32::from(end))
                    / i32::from(cell_size)
            };
            for z in cell(1, false)..=cell(1, true) {
                for x in cell(0, false)..=cell(0, true) {
                    let index = z as usize * dimensions[0] + x as usize;
                    cells[index].nodes.push(node);
                    offsets[index].push(offset);
                }
            }
        }
        Ok(Self {
            origin: [bounds[0], bounds[1]],
            dimensions,
            cell_size,
            cells,
            gaisen: Some(Gaisen {
                map: map.clone(),
                offsets,
                water_depth: 0,
            }),
        })
    }

    pub fn set_gaisen_water_depth(&mut self, depth: i16) -> Result<()> {
        if !(0..=16000).contains(&depth) || depth % 10 != 0 {
            return Err(
                "Imported water depth must be 0–16000 game units, in multiples of 10".into(),
            );
        }
        let source = self
            .gaisen
            .as_mut()
            .ok_or("Water depth requires an imported Gaisen stage")?;
        if source.water_depth == depth {
            return Ok(());
        }
        for node in self.cells.iter().flat_map(|c| &c.nodes) {
            if node.attributes & 0xf00 == 0x200 {
                node.y
                    .checked_add(depth / 10)
                    .ok_or("Imported water bottom exceeds terrain coordinates")?;
            }
        }
        for (cell, offsets) in self.cells.iter_mut().zip(&mut source.offsets) {
            let original = cell
                .nodes
                .iter()
                .position(|n| n.attributes & 0xf00 == 0x300)
                .unwrap_or(cell.nodes.len());
            cell.nodes.truncate(original);
            offsets.truncate(original);
            if depth == 0 {
                continue;
            }
            for index in 0..original {
                let mut node = cell.nodes[index];
                if node.attributes & 0xf00 != 0x200 {
                    continue;
                }
                node.y += depth / 10;
                node.attributes = (node.attributes & !0xf00) | 0x300;
                cell.nodes.push(node);
                offsets.push(offsets[index]);
            }
        }
        source.water_depth = depth;
        Ok(())
    }
}
