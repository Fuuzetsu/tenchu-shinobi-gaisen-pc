use crate::{
    math::{Math, Matrix, Position, Vector3},
    tenchu2::world::Foot,
    visibility::project,
};

pub const IMAGE: u8 = 28;

pub struct Shadow {
    pub position: Position,
    pub corners: [Vector3<i16>; 4],
}

impl Shadow {
    pub fn new(feet: &[Foot; 2], width: i16, math: &Math) -> Option<Self> {
        let [a, b] = feet.map(|foot| Position {
            y: foot.position.y.max(foot.floor.unwrap_or(i32::MIN)),
            ..foot.position
        });
        let position = a.wrapping_add(b).map(|v| v >> 1);
        let mut along = b.wrapping_sub(a).map(|v| (v.wrapping_mul(3) >> 2) as i16);
        if i32::from(along.y).abs() > 1024 {
            return None;
        }
        let normal = math.normalize_short(along.to_array());
        let scaled = normal.map(|v| ((i32::from(v) * i32::from(width)) >> 12) as i16);
        let squared = along
            .into_iter()
            .fold(0i32, |sum, v| sum.wrapping_add(i32::from(v) * i32::from(v)));
        if squared < i32::from(width) * i32::from(width) {
            along = scaled.into();
        }
        let across = Vector3::new((-i32::from(scaled[2]) >> 1) as i16, 0, scaled[0] >> 1);
        Some(Self {
            position,
            corners: [
                along.map(i16::wrapping_neg),
                across.map(i16::wrapping_neg),
                across,
                along,
            ],
        })
    }

    pub fn visible(&self, view: Matrix, distance: i32) -> bool {
        let transform = view.compose(&Matrix {
            translation: self.position,
            ..Default::default()
        });
        let (screen, z) = project(&transform, self.corners[3].to_array());
        let depth = z >> 4;
        depth != 0
            && i32::from(depth) <= distance >> 4
            && i32::from(screen[0]).abs() <= 192
            && i32::from(screen[1]).abs() <= 152
    }
}
