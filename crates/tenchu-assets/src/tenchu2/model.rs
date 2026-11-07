use crate::{
    Result, i16_le,
    model::{Corner, Face, Geometry, Texture},
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

pub(super) fn scenery(data: &[u8]) -> Result<Geometry> {
    if u32_le(data, 0)? >> 24 != 0xa0 || u16_le(data, 4)? != 0 {
        return Err("Expected an unrelocated Tenchu 2 MDL".into());
    }
    if u16_le(data, 6)? != 1 || i16_le(data, 22)? != -1 {
        return Err("Tenchu 2 scenery must have one root part".into());
    }
    let count = usize::from(u16_le(data, 8)?);
    if count == 0 || u32_le(data, 24)? as usize != count {
        return Err("Invalid Tenchu 2 scenery vertex count".into());
    }
    let mut vertices = vec![[0; 3]; count];
    let mut seen = vec![false; count];
    let translation = short3(data, 16)?;
    let vertex_at = u32_le(data, 28)? as usize;
    for row in slice(data, vertex_at, count * 8)?.chunks_exact(8) {
        let index = usize::from(u16_le(row, 6)?);
        if index >= count || seen[index] {
            return Err("Invalid Tenchu 2 vertex identity".into());
        }
        seen[index] = true;
        let point = short3(row, 0)?;
        for axis in 0..3 {
            vertices[index][axis] = point[axis]
                .checked_add(translation[axis])
                .ok_or("Tenchu 2 scenery vertex overflow")?;
        }
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
            translucent: u16_le(row, 2)? & 0x8060 != 0,
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
    Ok(Geometry {
        vertices,
        normals: Vec::new(),
        faces,
    })
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
