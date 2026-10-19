use crate::{
    Result,
    math::Matrix,
    scene::{ModelId, Scene},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub depth: u16,
    pub fog: bool,
}

fn divide(h: u16, z: u16) -> u32 {
    if u32::from(h) >= u32::from(z) * 2 {
        return 0x1ffff;
    }
    let shift = z.leading_zeros();
    let d = u32::from(z) << shift;
    let n = u32::from(h) << shift;
    let index = (d - 0x7fc0) >> 7;
    let table = if index == 256 {
        0
    } else {
        (((0x40000 / (index + 0x100) + 1) / 2) - 0x101) as u8
    };
    let estimate = 0x101 + u32::from(table);
    let correction = (0x2000080 - d * estimate) >> 8;
    let reciprocal = (correction * estimate + 0x80) >> 8;
    (((u64::from(n) * u64::from(reciprocal) + 0x8000) >> 16) as u32).min(0x1ffff)
}

pub fn project(matrix: &Matrix, point: [i16; 3]) -> ([i16; 2], u16) {
    let transformed = matrix.transform(point);
    let depth = transformed[2].clamp(0, 65535) as u16;
    let ratio = i64::from(divide(300, depth));
    let xy = std::array::from_fn(|i| {
        ((i64::from(transformed[i].clamp(-32768, 32767)) * ratio) >> 16).clamp(-1024, 1023) as i16
    });
    (xy, depth)
}

pub fn clip(matrix: &Matrix, attribute: u16, center: [i16; 3]) -> Option<Clip> {
    if attribute & 1 != 0 {
        return None;
    }
    if attribute & 2 == 0 {
        let (xy, z) = project(matrix, center);
        let depth = z >> 4;
        if (attribute & 4 != 0 && depth == 0)
            || (attribute & 8 != 0
                && (i32::from(xy[0]).abs() > 240 || i32::from(xy[1]).abs() > 180))
            || (attribute & 0x10 != 0 && depth > 1250)
        {
            return None;
        }
    }
    let depth = matrix.transform([0; 3])[2].clamp(0, 65535) as u16 >> 4;
    (depth <= 1250).then_some(Clip {
        depth,
        fog: depth >= 300,
    })
}

pub fn model(scene: &mut Scene, view: Matrix, id: ModelId) -> Result<Option<Clip>> {
    let matrix = view.compose(&scene.world(id)?);
    let model = &scene.models[id];
    Ok(clip(&matrix, model.attribute, model.clip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reciprocal_and_clip_boundaries_preserve_the_source_depth_units() {
        assert_eq!(divide(0, 0), 131071);
        assert_eq!(divide(300, 150), 131071);
        assert_eq!(divide(29, 29), 65535);
        let at = |z| Matrix {
            translation: [0, 0, z],
            ..Matrix::default()
        };
        assert_eq!(
            clip(&at(4799), 0, [0, 0, 1000]),
            Some(Clip {
                depth: 299,
                fog: false
            })
        );
        assert_eq!(
            clip(&at(4800), 0, [0, 0, -1000]),
            Some(Clip {
                depth: 300,
                fog: true
            })
        );
        assert!(clip(&at(20015), 0x1c, [0; 3]).is_some());
        assert!(clip(&at(20016), 2, [0; 3]).is_none());
        assert!(clip(&at(-1), 0x1c, [0; 3]).is_none());
        assert!(clip(&at(-1), 2, [0; 3]).is_some());
        assert!(clip(&at(1000), 3, [0; 3]).is_none());
        assert!(clip(&at(1000), 8, [1000, 0, 0]).is_none());
        assert!(clip(&at(1000), 10, [1000, 0, 0]).is_some());
    }
}
