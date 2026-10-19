use crate::Result;
use tenchu_assets::executable::MathTables;

pub struct Math {
    pub tables: MathTables,
}

impl Math {
    pub fn pair(&self, angle: i16) -> [i32; 2] {
        let a = i32::from(angle);
        let [s, c] = self.tables.trig[(a.abs() & 4095) as usize].map(i32::from);
        [if a < 0 { -s } else { s }, c]
    }

    pub fn square_root(&self, value: i32) -> i32 {
        let leading = if value < 0 {
            value.leading_ones()
        } else {
            value.leading_zeros()
        };
        if leading == 32 {
            return 0;
        }
        let even = leading & !1;
        let exponent = (31 - even) >> 1;
        let mantissa = if even >= 24 {
            value << (even - 24)
        } else {
            value >> (24 - even)
        };
        let entry = i32::from(self.tables.sqrt[(mantissa + 256) as usize]);
        ((entry as u32).wrapping_shl(exponent) >> 12) as i32
    }

    pub fn length(&self, vector: [i32; 3]) -> i32 {
        self.square_root(
            vector
                .into_iter()
                .fold(0i32, |sum, x| sum.wrapping_add(x.wrapping_mul(x))),
        )
    }

    pub fn vector_distance(&self, delta: [i32; 3]) -> i32 {
        if delta.iter().any(|x| x.wrapping_abs() > 4096) {
            self.length(delta.map(|x| x / 256)).wrapping_mul(256)
        } else {
            self.length(delta)
        }
    }

    pub fn atan2(&self, y: i32, x: i32) -> Result<i16> {
        let ay = y.checked_abs().ok_or("ratan2 y overflow")?;
        let ax = x.checked_abs().ok_or("ratan2 x overflow")?;
        if ay == 0 && ax == 0 {
            return Ok(0);
        }
        let (n, d) = if ay < ax { (ay, ax) } else { (ax, ay) };
        let q = if n & 0x7fe00000 != 0 {
            n / (d >> 10)
        } else {
            n.wrapping_shl(10) / d
        };
        let mut a = i32::from(
            *self
                .tables
                .atan
                .get(q as usize)
                .ok_or("ratan2 table overflow")?,
        );
        if ay >= ax {
            a = 1024 - a;
        }
        if x < 0 {
            a = 2048 - a;
        }
        if y < 0 {
            a = -a;
        }
        Ok(a as i16)
    }

    pub fn direction(&self, dx: i32, dz: i32, yaw: i16) -> Result<i16> {
        let d = i32::from(self.atan2(dx.wrapping_neg(), dz.wrapping_neg())?) - i32::from(yaw);
        Ok(if d as i16 > 2048 {
            4096 - d
        } else if d as i16 <= -2048 {
            d + 4096
        } else {
            d
        } as i16)
    }

    pub fn velocity(&self, yaw: i16, forward: i16, side: i16) -> [i16; 3] {
        let [s, c] = self.pair(yaw).map(|v| -v);
        [
            ((s * i32::from(forward) - c * i32::from(side)) >> 12) as i16,
            0,
            ((c * i32::from(forward) + s * i32::from(side)) >> 12) as i16,
        ]
    }

    pub fn rotate_yxz(&self, angles: [i16; 3]) -> [[i16; 3]; 3] {
        let [sx, cx] = self.pair(angles[0]);
        let [sy, cy] = self.pair(angles[1]);
        let [sz, cz] = self.pair(angles[2]);
        let m = |a: i32, b: i32| a.wrapping_mul(b) >> 12;
        let sysx = m(sy, sx);
        let cysx = m(cy, sx);
        [
            [
                (m(cy, cz) + m(sysx, sz)) as i16,
                (-m(cy, sz) + m(sysx, cz)) as i16,
                m(sy, cx) as i16,
            ],
            [m(sz, cx) as i16, m(cz, cx) as i16, -sx as i16],
            [
                (-m(sy, cz) + m(cysx, sz)) as i16,
                (m(sy, sz) + m(cysx, cz)) as i16,
                m(cy, cx) as i16,
            ],
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Matrix {
    pub rotation: [[i16; 3]; 3],
    pub translation: [i32; 3],
}

impl Default for Matrix {
    fn default() -> Self {
        Self {
            rotation: [[4096, 0, 0], [0, 4096, 0], [0, 0, 4096]],
            translation: [0; 3],
        }
    }
}

fn dot(row: [i16; 3], vector: [i16; 3]) -> i64 {
    (0..3)
        .map(|i| i64::from(row[i]) * i64::from(vector[i]))
        .sum()
}

impl Matrix {
    pub fn apply_large(&self, vector: [i32; 3]) -> [i32; 3] {
        let hi = vector.map(|x| (x / 32768) as i16);
        let lo = vector.map(|x| (x % 32768) as i16);
        std::array::from_fn(|i| {
            (dot(self.rotation[i], hi) as i32)
                .wrapping_shl(3)
                .wrapping_add((dot(self.rotation[i], lo) >> 12) as i32)
        })
    }

    pub fn compose(&self, child: &Self) -> Self {
        let rotation = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let column = std::array::from_fn(|k| child.rotation[k][j]);
                (dot(self.rotation[i], column) >> 12).clamp(-32768, 32767) as i16
            })
        });
        let v = self.apply_large(child.translation);
        Self {
            rotation,
            translation: std::array::from_fn(|i| v[i].wrapping_add(self.translation[i])),
        }
    }

    pub fn transform(&self, vector: [i16; 3]) -> [i32; 3] {
        std::array::from_fn(|i| {
            let sum = (i64::from(self.translation[i]) << 12) + dot(self.rotation[i], vector);
            let mac = (sum << 20) >> 20;
            (mac >> 12) as i32
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_paths_preserve_distinct_mac_and_ir_rounding() {
        let m = Matrix {
            rotation: [[30000; 3]; 3],
            translation: [i32::MAX; 3],
        };
        assert_eq!(m.transform([30000; 3]), [i32::MIN + 659178; 3]);
        let child = Matrix {
            rotation: [[30000; 3]; 3],
            translation: [0; 3],
        };
        assert_eq!(m.compose(&child).rotation, [[i16::MAX; 3]; 3]);
        let m = Matrix {
            rotation: [[4095, 1, 0], [0, 4096, 0], [0, 0, 4096]],
            translation: [0; 3],
        };
        assert_eq!(m.apply_large([-32769, 32768, -1]), [-32753, 32768, -1]);
    }
}
