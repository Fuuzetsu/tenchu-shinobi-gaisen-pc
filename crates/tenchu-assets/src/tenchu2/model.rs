use crate::{
    Result, i16_le,
    model::{Corner, Face, Geometry, Part, Texture},
    short3, slice, u16_le, u32_le,
};

pub(super) fn resolve_transparency<'a>(
    parts: impl Iterator<Item = &'a mut crate::model::Part>,
    vram: &crate::texture::Vram,
) {
    let mut cache = std::collections::BTreeMap::new();
    for face in parts
        .flat_map(|p| &mut p.geometry)
        .flat_map(|g| &mut g.faces)
    {
        if !face.translucent {
            continue;
        }
        let Some(texture) = face.texture else {
            continue;
        };
        let mut bounds = [255, 255, 0, 0];
        for corner in &face.corners {
            for axis in 0..2 {
                bounds[axis] = bounds[axis].min(corner.uv[axis]);
                bounds[axis + 2] = bounds[axis + 2].max(corner.uv[axis]);
            }
        }
        // The PS1 blend flag affects only texels with the STP bit set. Ordinary
        // opaque floors still need depth writes and shadow/footprint receivers.
        face.translucent = *cache
            .entry((texture.page, texture.palette, bounds))
            .or_insert_with(|| {
                (bounds[1]..=bounds[3]).any(|v| {
                    (bounds[0]..=bounds[2]).any(|u| {
                        vram.texel(texture.page, texture.palette, [u16::from(u), u16::from(v)])
                            & 0x8000
                            != 0
                    })
                })
            });
    }
}

struct Mesh {
    parents: Vec<i16>,
    positions: Vec<[i16; 3]>,
    vertices: Vec<[i16; 3]>,
    joints: Vec<usize>,
    faces: Vec<Face>,
}

fn parse(data: &[u8]) -> Result<Mesh> {
    if !matches!(u32_le(data, 0)? >> 24, 0x99 | 0xa0) || u16_le(data, 4)? != 0 {
        return Err("Expected an unrelocated Tenchu 2 MDL".into());
    }
    let joints = usize::from(u16_le(data, 6)?);
    if !(1..=64).contains(&joints) {
        return Err("Invalid Tenchu 2 joint count".into());
    }
    let count = usize::from(u16_le(data, 8)?);
    if count == 0 {
        return Err("Invalid Tenchu 2 scenery vertex count".into());
    }
    let mut vertices = vec![[0; 3]; count];
    let mut owners = vec![usize::MAX; count];
    let mut parents = Vec::new();
    let mut positions = Vec::new();
    for (joint, header) in slice(data, 16, joints * 16)?.chunks_exact(16).enumerate() {
        let parent = i16_le(header, 6)?;
        if parent < -1 || parent >= joints as i16 || parent == joint as i16 {
            return Err("Invalid Tenchu 2 joint parent".into());
        }
        parents.push(parent);
        positions.push(short3(header, 0)?);
        let n = u32_le(header, 8)? as usize;
        if n > count {
            return Err("Invalid Tenchu 2 joint vertex count".into());
        }
        let at = u32_le(header, 12)? as usize;
        for row in slice(data, at, n * 8)?.chunks_exact(8) {
            let index = usize::from(u16_le(row, 6)?);
            if index >= count || owners[index] != usize::MAX {
                return Err("Invalid Tenchu 2 vertex identity".into());
            }
            owners[index] = joint;
            vertices[index] = short3(row, 0)?;
        }
    }
    if owners.contains(&usize::MAX) {
        return Err("Missing Tenchu 2 vertex".into());
    }
    let count = usize::from(u16_le(data, 10)?);
    let face_at = u32_le(data, 12)? as usize;
    let mut faces = Vec::with_capacity(count);
    for row in slice(data, face_at, count * 28)?.chunks_exact(28) {
        let corners = if u16_le(row, 26)? == u16::MAX { 3 } else { 4 };
        let mut face = Face {
            mode: if corners == 3 { 0x34 } else { 0x3c },
            flags: 1,
            corners: Vec::with_capacity(corners),
            texture: Some(Texture {
                page: u16_le(row, 2)? & 0x1ff,
                palette: u16_le(row, 0)?,
            }),
            double_sided: false,
            translucent: u16_le(row, 2)? & 0x60 != 0,
        };
        for i in 0..corners {
            let offset = u16_le(row, 20 + i * 2)?;
            if !offset.is_multiple_of(8) || usize::from(offset / 8) >= vertices.len() {
                return Err("Tenchu 2 face references a missing vertex".into());
            }
            face.corners.push(Corner {
                vertex: offset / 8,
                normal: None,
                color: [row[12 + i * 2]; 3],
                uv: [row[4 + i * 2], row[5 + i * 2]],
            });
        }
        faces.push(face);
    }
    Ok(Mesh {
        parents,
        positions,
        vertices,
        joints: owners,
        faces,
    })
}

pub(super) fn scenery(data: &[u8]) -> Result<Geometry> {
    scenery_mesh(parse(data)?)
}

pub(super) fn stump(data: &[u8]) -> Result<Geometry> {
    let mut mesh = parse(data)?;
    mesh.faces = mesh
        .faces
        .into_iter()
        .filter(|face| {
            face.corners
                .iter()
                .take(3)
                .any(|corner| mesh.vertices[usize::from(corner.vertex)][1] == 0)
        })
        .take(3)
        .map(|mut face| {
            for corner in &mut face.corners {
                if mesh.vertices[usize::from(corner.vertex)][1] != 0 {
                    corner.uv[1] = corner.uv[1].wrapping_add(17);
                }
            }
            face
        })
        .collect();
    for vertex in &mut mesh.vertices {
        if vertex[1] == -2000 {
            vertex[1] = -1000;
        }
    }
    scenery_mesh(mesh)
}

fn scenery_mesh(mesh: Mesh) -> Result<Geometry> {
    if mesh.parents != [-1] {
        return Err("Tenchu 2 scenery must have one root part".into());
    }
    let vertices = mesh
        .vertices
        .into_iter()
        .map(|point| {
            let mut vertex = [0; 3];
            for axis in 0..3 {
                vertex[axis] = point[axis]
                    .checked_add(mesh.positions[0][axis])
                    .ok_or("Tenchu 2 scenery vertex overflow")?;
            }
            Ok(vertex)
        })
        .collect::<Result<_>>()?;
    Ok(Geometry {
        vertices,
        joints: Vec::new(),
        normals: Vec::new(),
        faces: mesh.faces,
    })
}

pub(super) fn character(data: &[u8]) -> Result<Vec<Part>> {
    parts(parse(data)?, &super::BONES)
}

pub(super) fn rig(data: &[u8]) -> Result<Vec<Part>> {
    let mesh = parse(data)?;
    let bones: Vec<_> = (0..mesh.parents.len()).collect();
    parts(mesh, &bones)
}

fn parts(mesh: Mesh, bones: &[usize]) -> Result<Vec<Part>> {
    let mut roots = vec![[0; 3]; mesh.parents.len()];
    let mut mapped = vec![None; mesh.parents.len()];
    // Detached costume pieces can have several roots and forward parent links.
    while mapped.iter().any(Option::is_none) {
        let mut changed = false;
        for (joint, &parent) in mesh.parents.iter().enumerate() {
            if mapped[joint].is_some() || (parent >= 0 && mapped[parent as usize].is_none()) {
                continue;
            }
            let origin = if parent < 0 {
                [0; 3]
            } else {
                roots[parent as usize]
            };
            roots[joint] = std::array::from_fn(|i| origin[i] + i32::from(mesh.positions[joint][i]));
            mapped[joint] = Some(
                bones
                    .iter()
                    .position(|&b| b == joint)
                    .or_else(|| (parent >= 0).then(|| mapped[parent as usize]).flatten())
                    .ok_or("Unmapped Tenchu 2 root joint")?,
            );
            changed = true;
        }
        if !changed {
            return Err("Cyclic Tenchu 2 joint hierarchy".into());
        }
    }
    let mapped: Vec<_> = mapped.into_iter().map(Option::unwrap).collect();
    let mut parts: Vec<_> = bones
        .iter()
        .enumerate()
        .map(|(index, &joint)| {
            let parent = mesh.parents.get(joint).copied().unwrap_or(0);
            Part {
                parent: if parent < 0 {
                    -1
                } else {
                    mapped[parent as usize] as i16
                },
                child: index as i16,
                position: mesh.positions.get(joint).copied().unwrap_or([0; 3]),
                geometry: vec![Geometry {
                    vertices: Vec::new(),
                    joints: Vec::new(),
                    normals: Vec::new(),
                    faces: Vec::new(),
                }],
            }
        })
        .collect();
    let mut indices = vec![std::collections::BTreeMap::new(); parts.len()];
    for mut face in mesh.faces {
        let mut counts = vec![0; parts.len()];
        for corner in &face.corners {
            counts[mapped[mesh.joints[usize::from(corner.vertex)]]] += 1;
        }
        let owner = (0..parts.len()).max_by_key(|&i| counts[i]).unwrap();
        let root = roots[bones[owner]];
        let geometry = &mut parts[owner].geometry[0];
        for corner in &mut face.corners {
            let source = usize::from(corner.vertex);
            if let Some(&index) = indices[owner].get(&source) {
                corner.vertex = index;
                continue;
            }
            let joint = mesh.joints[source];
            let mut point = [0; 3];
            for axis in 0..3 {
                point[axis] = (i32::from(mesh.vertices[source][axis]) + roots[joint][axis]
                    - root[axis])
                    .try_into()
                    .map_err(|_| "Tenchu 2 character vertex overflow")?;
            }
            let index =
                u16::try_from(geometry.vertices.len()).map_err(|_| "Too many Tenchu 2 vertices")?;
            geometry.vertices.push(point);
            geometry.joints.push(mapped[joint] as u8);
            indices[owner].insert(source, index);
            corner.vertex = index;
        }
        geometry.faces.push(face);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenery_checks_global_indices_and_resolves_opaque_blend_textures() {
        let mut data = vec![0; 84];
        let mut word =
            |at: usize, value: u16| data[at..at + 2].copy_from_slice(&value.to_le_bytes());
        word(2, 0xa000);
        word(6, 1);
        word(8, 3);
        word(10, 1);
        word(12, 56);
        word(22, u16::MAX);
        word(24, 3);
        word(28, 32);
        word(46, 1);
        word(54, 2);
        word(56, 64);
        word(58, 0x20);
        word(78, 8);
        word(80, 16);
        word(82, u16::MAX);
        let geometry = scenery(&data).unwrap();
        let mut part = crate::model::Part {
            parent: -1,
            child: -1,
            position: [0; 3],
            geometry: vec![geometry],
        };
        let mut vram = crate::texture::Vram::default();
        vram.words[0] = 0x1111;
        vram.words[1025] = 0x7fff;
        resolve_transparency(std::iter::once(&mut part), &vram);
        assert!(!part.geometry[0].faces[0].translucent);
        part.geometry[0].faces[0].translucent = true;
        vram.words[1025] |= 0x8000;
        resolve_transparency(std::iter::once(&mut part), &vram);
        assert!(part.geometry[0].faces[0].translucent);
        data[80..82].copy_from_slice(&24u16.to_le_bytes());
        assert!(scenery(&data).is_err());
        data[80..82].copy_from_slice(&16u16.to_le_bytes());
        data[46..48].copy_from_slice(&0u16.to_le_bytes());
        assert!(scenery(&data).is_err());
    }
}
