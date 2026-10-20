use glam::Vec3;
use tenchu_assets::Result;

#[derive(Clone, Copy)]
pub struct Corner {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Triangle {
    pub corners: [Corner; 3],
    pub material: String,
}

pub fn decode(text: &str) -> Result<Vec<Triangle>> {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut material = String::new();
    let mut triangles = Vec::new();
    for (line, text) in text.lines().enumerate() {
        let fields: Vec<_> = text.split('#').next().unwrap().split_whitespace().collect();
        let Some(&command) = fields.first() else {
            continue;
        };
        let invalid = || format!("OBJ line {}: invalid {command}", line + 1);
        let number = |word: &str| -> Result<f32> {
            word.parse::<f32>()
                .ok()
                .filter(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                .ok_or_else(invalid)
        };
        match command {
            "v" => {
                if ![4, 7].contains(&fields.len()) {
                    return Err(invalid());
                }
                positions.push([number(fields[1])?, number(fields[2])?, number(fields[3])?]);
                colors.push(if fields.len() == 7 {
                    [number(fields[4])?, number(fields[5])?, number(fields[6])?]
                        .map(|n| n.clamp(0.0, 2.0))
                } else {
                    [1.0; 3]
                });
            }
            "vt" => {
                if fields.len() < 3 || fields.len() > 4 {
                    return Err(invalid());
                }
                uvs.push([number(fields[1])?, number(fields[2])?]);
            }
            "vn" => {
                if fields.len() != 4 {
                    return Err(invalid());
                }
                normals.push(
                    Vec3::new(number(fields[1])?, number(fields[2])?, number(fields[3])?)
                        .normalize_or_zero()
                        .to_array(),
                );
            }
            "usemtl" => {
                if fields.len() != 2 {
                    return Err(invalid());
                }
                material = fields[1].to_owned();
            }
            "f" => {
                if fields.len() != 4 {
                    return Err(format!(
                        "OBJ line {}: triangulate faces before exporting",
                        line + 1
                    ));
                }
                let corner = |field: &str| -> Result<Corner> {
                    let parts: Vec<_> = field.split('/').collect();
                    if parts.len() > 3 {
                        return Err(invalid());
                    }
                    let index = |word: &str, count: usize| -> Result<usize> {
                        let n = word.parse::<i64>().map_err(|_| invalid())?;
                        let n = if n < 0 { count as i64 + n } else { n - 1 };
                        if n < 0 || n >= count as i64 {
                            return Err(invalid());
                        }
                        Ok(n as usize)
                    };
                    let vertex = index(parts[0], positions.len())?;
                    let uv = if parts.len() > 1 && !parts[1].is_empty() {
                        uvs[index(parts[1], uvs.len())?]
                    } else {
                        [0.0; 2]
                    };
                    let normal = if parts.len() > 2 && !parts[2].is_empty() {
                        normals[index(parts[2], normals.len())?]
                    } else {
                        [0.0; 3]
                    };
                    Ok(Corner {
                        position: positions[vertex],
                        color: colors[vertex],
                        uv,
                        normal,
                    })
                };
                triangles.push(Triangle {
                    corners: [corner(fields[1])?, corner(fields[2])?, corner(fields[3])?],
                    material: material.clone(),
                });
                if triangles.len() > 500_000 {
                    return Err("OBJ exceeds 500,000 triangles".into());
                }
            }
            "o" | "g" | "s" | "mtllib" => {}
            _ => return Err(format!("OBJ line {}: unsupported {command}", line + 1)),
        }
    }
    if triangles.is_empty() {
        return Err("OBJ contains no triangles".into());
    }
    Ok(triangles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_separates_corner_uvs_and_normals_and_rejects_invalid_indices() {
        let text = "v 0 0 0 1 .5 .2\nv 10 0 0\nv 0 10 0\nvt .125 .25\nvn 0 0 2\nusemtl cloth\nf -3/1/1 -2//1 -1/1\n";
        let triangle = &decode(text).unwrap()[0];
        assert_eq!(triangle.material, "cloth");
        assert_eq!(triangle.corners[0].uv, [0.125, 0.25]);
        assert_eq!(triangle.corners[1].normal, [0.0, 0.0, 1.0]);
        assert_eq!(triangle.corners[2].position, [0.0, 10.0, 0.0]);
        assert!(decode(&text.replace("-1/1", "4/1")).is_err());
        assert!(decode(&text.replace("-3/1/1", "0/1/1")).is_err());
        assert!(decode(&text.replace("v 0 0 0", "v NaN 0 0")).is_err());
    }
}
