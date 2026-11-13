use super::*;
use std::collections::{BTreeMap, BTreeSet};

impl AreaMap {
    /// Authoring edits address nodes in the original map. Unaffected rows retain
    /// their source lookup tables and ordering; changed rows get fresh lists.
    pub fn edited(&self, changes: &[(usize, usize, Node)]) -> Result<Self> {
        let mut pending = BTreeMap::<usize, BTreeMap<usize, Node>>::new();
        let mut seen = BTreeSet::new();
        for &(row, index, node) in changes {
            if !seen.insert((row, index)) {
                return Err("Duplicate terrain surface edit".into());
            }
            let source = self.rows.get(row).ok_or("Missing terrain row")?;
            if index >= source.count {
                return Err("Missing terrain surface".into());
            }
            let original = self.node(source.list + index * 16)?;
            if node.bounds != original.bounds {
                return Err(
                    "Keep the source surface footprint when editing a terrain layer".into(),
                );
            }
            validate(node)?;
            if node != original {
                pending.entry(row).or_default().insert(index, node);
            }
        }
        if pending.is_empty() {
            return Ok(self.clone());
        }
        let mut out = self.clone();
        let mut rows = BTreeMap::<usize, Vec<Node>>::new();
        let mut removed = BTreeMap::<usize, BTreeSet<usize>>::new();
        for (&row, nodes) in &pending {
            removed.insert(row, nodes.keys().copied().collect());
        }
        for (row, nodes) in pending {
            rows.entry(row).or_default();
            for (index, node) in nodes {
                let top = i32::from(node.y) + i32::from(node.dy).max(0) + 2;
                let original = self.node(self.rows[row].list + index * 16)?;
                let unchanged_height = original.y == node.y && original.dy == node.dy;
                let destination = if self.grid.is_some() || unchanged_height {
                    row
                } else {
                    out.rows
                        .iter()
                        .position(|r| i32::from(r.height) >= top)
                        .unwrap_or_else(|| {
                            out.rows.push(Row {
                                height: top as i16,
                                bounds: node.bounds,
                                count: 0,
                                list: 0,
                                lookup: None,
                            });
                            out.rows.len() - 1
                        })
                };
                rows.entry(destination).or_default().push(node);
            }
        }
        for (index, mut additions) in rows {
            let mut nodes = Vec::new();
            if let Some(source) = self.rows.get(index) {
                for i in 0..source.count {
                    if !removed.get(&index).is_some_and(|set| set.contains(&i)) {
                        nodes.push(self.node(source.list + i * 16)?);
                    }
                }
            }
            nodes.append(&mut additions);
            nodes.sort_by_key(|n| n.bounds[1]);
            if nodes.len() > i16::MAX as usize {
                return Err("Too many surfaces in one terrain layer".into());
            }
            let row = &mut out.rows[index];
            row.list = out.data.len();
            row.count = nodes.len();
            row.lookup = None;
            if self.grid.is_none() && !nodes.is_empty() {
                row.bounds = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
                for node in &nodes {
                    for axis in 0..2 {
                        row.bounds[axis] = row.bounds[axis].min(node.bounds[axis]);
                        row.bounds[axis + 2] = row.bounds[axis + 2].max(node.bounds[axis + 2]);
                    }
                }
            }
            for node in nodes {
                for value in [
                    node.y as u16,
                    node.dy as u16,
                    node.bounds[0] as u16,
                    node.bounds[1] as u16,
                    node.bounds[2] as u16,
                    node.bounds[3] as u16,
                    node.attributes,
                    node.division,
                ] {
                    out.data.extend(value.to_le_bytes());
                }
            }
        }
        Ok(out)
    }
}

pub(super) fn validate(node: Node) -> Result<()> {
    let end = i32::from(node.y) + i32::from(node.dy);
    if !(-32_000..=32_000).contains(&node.y)
        || !(-32_000..=32_000).contains(&end)
        || node.bounds[0] > node.bounds[2]
        || node.bounds[1] > node.bounds[3]
    {
        return Err("Edited terrain exceeds the supported coordinate range".into());
    }
    Ok(())
}
