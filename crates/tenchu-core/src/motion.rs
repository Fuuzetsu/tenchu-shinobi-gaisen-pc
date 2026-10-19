use crate::{
    Result,
    math::{Math, Matrix},
};
use tenchu_assets::{
    executable::MotionRegistration,
    model::Part,
    motion::{Clip, Key},
};

pub fn tangents([previous, a, b, next]: [Key; 4], duration: i16) -> Result<[[i16; 3]; 2]> {
    let left = if a.time == 0 { a } else { previous };
    let right = if b.time < duration { next } else { b };
    if b.time <= a.time || b.time <= left.time || right.time <= a.time {
        return Err("Invalid spline interval".into());
    }
    let dt = i32::from(b.time.wrapping_sub(a.time) as i8) * 256;
    let slope0 = (dt / (i32::from(b.time) - i32::from(left.time))) as i16;
    let slope1 = (dt / (i32::from(right.time) - i32::from(a.time))) as i16;
    let tangent = |p: Key, q: Key, slope: i16| {
        std::array::from_fn(|i| {
            ((i32::from(slope) * (i32::from(q.value[i]) - i32::from(p.value[i]))) >> 8) as i16
        })
    };
    Ok([tangent(left, b, slope0), tangent(a, right, slope1)])
}

pub fn evaluate(a: Key, b: Key, tangent: [[i16; 3]; 2], basis: [i16; 4]) -> [i16; 3] {
    std::array::from_fn(|i| {
        // eval_spline_gte_ reads MAC, whereas matrix multiplication reads IR.
        let mac = (i64::from(a.value[i]) * i64::from(basis[0])
            + i64::from(b.value[i]) * i64::from(basis[1])
            + i64::from(tangent[0][i]) * i64::from(basis[2]))
            >> 12;
        let tail = ((i32::from(tangent[1][i]) * i32::from(basis[3])) as u32) >> 12;
        (mac as i32).wrapping_add(tail as i32) as i16
    })
}

pub fn sample(keys: &[Key], frame: i16, duration: i16, table: &[[i16; 4]; 33]) -> Result<[i16; 3]> {
    let first = keys.first().ok_or("Empty animation track")?;
    if duration == 0 {
        return Ok(first.value);
    }
    if frame < first.time || frame > duration {
        return Err("Animation time outside clip".into());
    }
    let i = keys
        .windows(2)
        .position(|p| frame <= p[1].time)
        .ok_or("Animation interval missing")?;
    let (a, b) = (keys[i], keys[i + 1]);
    let tangent = tangents(
        [
            keys[i.saturating_sub(1)],
            a,
            b,
            keys[(i + 2).min(keys.len() - 1)],
        ],
        duration,
    )?;
    let fraction =
        (i32::from(frame) - i32::from(a.time)) * 32 / (i32::from(b.time) - i32::from(a.time));
    Ok(evaluate(a, b, tangent, table[fraction as usize]))
}

#[derive(Clone, Debug)]
pub struct Pose {
    pub translation: Vec<[i32; 3]>,
    pub rotation: Vec<[i16; 3]>,
    parents: Vec<Option<usize>>,
    root_height: i16,
}

impl Pose {
    pub fn new(parts: &[Part]) -> Result<Self> {
        let mut parents = Vec::new();
        for part in parts {
            parents.push(if part.parent < 0 {
                None
            } else {
                Some(
                    parts
                        .iter()
                        .position(|p| p.child == part.parent)
                        .ok_or("Missing model parent")?,
                )
            });
        }
        for i in 0..parents.len() {
            let mut p = parents[i];
            for depth in 0..=parents.len() {
                let Some(id) = p else {
                    break;
                };
                if id == i || depth == parents.len() {
                    return Err("Cyclic model hierarchy".into());
                }
                p = parents[id];
            }
        }
        Ok(Self {
            translation: parts.iter().map(|p| p.position.map(i32::from)).collect(),
            rotation: vec![[0; 3]; parts.len()],
            parents,
            root_height: parts.first().map_or(0, |p| p.position[1]),
        })
    }

    pub fn matrices(&self, math: &Math, root: Matrix) -> Vec<Matrix> {
        let mut out = vec![None; self.parents.len()];
        for _ in 0..out.len() {
            let mut complete = true;
            for i in 0..out.len() {
                if out[i].is_some() {
                    continue;
                }
                let parent = self.parents[i].map_or(Some(root), |p| out[p]);
                if let Some(parent) = parent {
                    out[i] = Some(parent.compose(&Matrix {
                        rotation: math.rotate_yxz(self.rotation[i]),
                        translation: self.translation[i],
                    }));
                } else {
                    complete = false;
                }
            }
            if complete {
                break;
            }
        }
        out.into_iter().map(Option::unwrap).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Motion {
    pub action: i16,
    pub clip: Option<usize>,
    pub count: i16,
    pub loops: i16,
    pub mask: u32,
    pub pose: Pose,
}

impl Motion {
    pub fn new(parts: &[Part]) -> Result<Self> {
        Ok(Self {
            action: -1,
            clip: None,
            count: 0,
            loops: 0,
            mask: u32::MAX,
            pose: Pose::new(parts)?,
        })
    }

    pub fn change(
        &mut self,
        action: i16,
        registry: &[MotionRegistration],
        common: &[MotionRegistration],
        clips: &[Clip],
    ) -> bool {
        if action == self.action {
            return false;
        }
        let find = |table: &[MotionRegistration]| {
            table
                .iter()
                .find(|r| r.action == action)
                .and_then(|r| clips.iter().position(|c| c.id == r.clip))
        };
        let Some(index) = find(registry).or_else(|| find(common)) else {
            return false;
        };
        self.action = action;
        self.clip = Some(index);
        self.count = i16::from(clips[index].sweep);
        self.loops = 0;
        for angles in &mut self.pose.rotation {
            for angle in angles {
                let a = i32::from(*angle);
                *angle = (if a.abs() > 2048 {
                    if a < 0 { a + 4096 } else { a - 4096 }
                } else {
                    a
                } % 4096) as i16;
            }
        }
        true
    }

    pub fn step(&mut self, clips: &[Clip], table: &[[i16; 4]; 33]) -> Result<i16> {
        if self.loops < 0 {
            return Ok(0);
        }
        let clip = &clips[self.clip.ok_or("Motion not selected")?];
        let n = clip.bones.len().min(self.pose.rotation.len());
        if n == 0 {
            return Err("Selected motion has no model parts".into());
        }
        let sweep = self.count < 0;
        let held = !sweep && clip.duration == 0;
        let frame = if sweep { 0 } else { self.count };
        let sample = |track: &[Key]| sample(track, frame, clip.duration, table);
        if self.mask & 1 != 0 {
            let root = sample(&clip.root)?;
            let target = [
                i32::from(root[0]),
                i32::from(self.pose.root_height) * i32::from(root[1]) >> 12,
                i32::from(root[2]),
            ];
            for (v, target) in self.pose.translation[0].iter_mut().zip(target) {
                *v = if sweep {
                    v.wrapping_add(target.wrapping_sub(*v) / -i32::from(self.count))
                } else {
                    target
                };
            }
        }
        for i in 0..n {
            if self.mask & 1u32.wrapping_shl(i as u32) == 0 {
                continue;
            }
            let target = sample(&clip.bones[i])?;
            for (v, target) in self.pose.rotation[i].iter_mut().zip(target) {
                *v = if sweep {
                    (i32::from(*v) + (i32::from(target) - i32::from(*v)) / -i32::from(self.count))
                        as i16
                } else {
                    target
                };
            }
        }
        if held {
            self.loops = -1;
            self.count = 0;
        } else {
            self.count = self.count.wrapping_add(1);
            if !sweep && self.count > clip.duration {
                self.count = 0;
                self.loops = self.loops.wrapping_add(1);
            }
        }
        Ok(self.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(value: i16, time: i16) -> Key {
        Key {
            value: [value; 3],
            time,
        }
    }

    #[test]
    fn spline_signed_interval_and_mac_edges_match_retail_routines() {
        let a = key(0, 0);
        let b = key(40, 128);
        assert_eq!(tangents([a, a, b, b], 128).unwrap(), [[-40; 3]; 2]);
        let b = key(40, 10);
        assert_eq!(tangents([a, a, b, b], 10).unwrap(), [[40; 3]; 2]);
        let a = key(30000, 0);
        assert_eq!(
            evaluate(a, a, [[30000; 3], [0; 3]], [30000, 30000, 30000, 0]),
            [(2_700_000_000i64 >> 12) as i16; 3]
        );
        let a = key(0, 0);
        assert_eq!(evaluate(a, a, [[0; 3], [-3; 3]], [0, 0, 0, 2048]), [-2; 3]);
    }
}
