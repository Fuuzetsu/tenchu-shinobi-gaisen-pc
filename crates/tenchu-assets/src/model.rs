use crate::{Result, i16_le, relative, short3, slice, u16_le, u32_le};

#[derive(Clone, Debug)]
pub struct Part {
    pub parent: i16,
    pub child: i16,
    pub position: [i16; 3],
    pub geometry: Vec<Geometry>,
}

#[derive(Clone, Debug)]
pub struct Geometry {
    pub vertices: Vec<[i16; 3]>,
    pub normals: Vec<[i16; 3]>,
    pub faces: Vec<Face>,
}

#[derive(Clone, Debug)]
pub struct Face {
    pub corners: Vec<Corner>,
    pub texture: Option<Texture>,
    pub double_sided: bool,
    pub translucent: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub vertex: u16,
    pub normal: Option<u16>,
    pub color: [u8; 3],
    pub uv: [u8; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Texture {
    pub page: u16,
    pub palette: u16,
}

pub fn archive(data: &[u8]) -> Result<Vec<Part>> {
    if u32_le(data, 0)? != 0x96102900 {
        return Err("Invalid MAD signature".into());
    }
    let count = u16_le(data, 4)? as usize;
    if count > 4096 {
        return Err("Excessive MAD part count".into());
    }
    slice(data, 8, count * 16)?;
    let mut out = Vec::new();
    for i in 0..count {
        let at = 8 + i * 16;
        let tmd = relative(data, 8, at + 12)?;
        out.push(Part {
            parent: i16_le(data, at)?,
            child: i16_le(data, at + 2)?,
            position: short3(data, at + 4)?,
            geometry: tmd_model(&data[tmd..])?,
        });
    }
    Ok(out)
}

pub fn tmd_model(data: &[u8]) -> Result<Vec<Geometry>> {
    if u32_le(data, 0)? != 0x41 || u32_le(data, 4)? != 0 {
        return Err("Expected unrelocated TMD".into());
    }
    let count = u32_le(data, 8)? as usize;
    if count > 1024 {
        return Err("Excessive TMD object count".into());
    }
    slice(data, 12, count * 28)?;
    let vectors = |field: usize| -> Result<Vec<[i16; 3]>> {
        let n = u32_le(data, field + 4)? as usize;
        if n == 0 {
            return Ok(Vec::new());
        }
        if n > 65536 {
            return Err("Excessive TMD vector count".into());
        }
        let at = relative(data, 12, field)?;
        slice(data, at, n * 8)?
            .chunks_exact(8)
            .map(|row| short3(row, 0))
            .collect()
    };
    let mut objects = Vec::new();
    for obj in 0..count {
        let at = 12 + obj * 28;
        if u32_le(data, at + 24)? != 0 {
            return Err("Nonzero TMD scale is unsupported".into());
        }
        let vertices = vectors(at)?;
        let normals = vectors(at + 8)?;
        let n = u32_le(data, at + 20)? as usize;
        if n > 1_000_000 {
            return Err("Excessive TMD face count".into());
        }
        let mut primitive = if n == 0 {
            0
        } else {
            relative(data, 12, at + 16)?
        };
        let mut faces = Vec::new();
        for _ in 0..n {
            let head = slice(data, primitive, 4)?;
            let length = usize::from(head[1]) * 4;
            let (flags, mode) = (head[2], head[3]);
            let p = slice(data, primitive + 4, length)?;
            if mode & 0xe0 != 0x20 {
                return Err(format!("Unsupported TMD primitive {mode:#x}"));
            }
            let n = if mode & 8 != 0 { 4 } else { 3 };
            let textured = mode & 4 != 0;
            let gouraud = mode & 16 != 0;
            let unlit = flags & 1 != 0;
            let colors = if gouraud { n } else { 1 };
            let color_at = if textured { n * 4 } else { 0 };
            let indices = color_at + if unlit || !textured { colors * 4 } else { 0 };
            let mut corners = Vec::new();
            for v in 0..n {
                let index_at = if unlit {
                    indices + v * 2
                } else if gouraud {
                    indices + v * 4 + 2
                } else {
                    indices + 2 + v * 2
                };
                let vertex = u16_le(p, index_at)?;
                if vertex as usize >= vertices.len() {
                    return Err("TMD vertex outside table".into());
                }
                let normal = if unlit {
                    None
                } else {
                    Some(u16_le(p, indices + if gouraud { v * 4 } else { 0 })?)
                };
                if normal.is_some_and(|i| i as usize >= normals.len()) {
                    return Err("TMD normal outside table".into());
                }
                let color = if unlit || !textured {
                    slice(p, color_at + if gouraud { v * 4 } else { 0 }, 3)?
                        .try_into()
                        .unwrap()
                } else {
                    [128; 3]
                };
                let uv = if textured {
                    slice(p, v * 4, 2)?.try_into().unwrap()
                } else {
                    [0; 2]
                };
                corners.push(Corner {
                    vertex,
                    normal,
                    color,
                    uv,
                });
            }
            let texture = if textured {
                Some(Texture {
                    page: u16_le(p, 6)?,
                    palette: u16_le(p, 2)?,
                })
            } else {
                None
            };
            faces.push(Face {
                corners,
                texture,
                double_sided: flags & 2 != 0,
                translucent: mode & 2 != 0,
            });
            primitive += length + 4;
        }
        objects.push(Geometry {
            vertices,
            normals,
            faces,
        });
    }
    Ok(objects)
}
