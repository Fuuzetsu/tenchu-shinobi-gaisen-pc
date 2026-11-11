use crate::{Result, math::Position};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::story::{Animation, Key};

mod faces;
pub use faces::{Face, Faces, TextureChange};
mod presentation;
pub use presentation::{Fade, Overlay, Presentation, PresentationFrame};
mod playback;
pub use playback::Playback;
mod effects;
pub use effects::Effects;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Motion {
    pub position: Position,
    pub rotation: Vec<[i16; 3]>,
    previous: Position,
    root: usize,
    joints: Vec<usize>,
}

impl Motion {
    pub fn validate(&self, animation: &Animation) -> Result<()> {
        if self.root >= animation.root.len().saturating_sub(1)
            || self.rotation.len() != animation.joints.len()
            || self.joints.len() != animation.joints.len()
            || self
                .joints
                .iter()
                .zip(&animation.joints)
                .any(|(&index, keys)| index >= keys.len().saturating_sub(1))
        {
            return Err("Invalid saved Tenchu 2 story motion".into());
        }
        Ok(())
    }

    pub fn camera_view(&self, roll: i32, math: &crate::math::Math) -> Result<crate::math::Matrix> {
        crate::camera::view_matrix(
            self.position,
            self.position.wrapping_add(self.camera_direction(math)?),
            roll,
            math,
        )
    }

    pub fn camera_direction(&self, math: &crate::math::Math) -> Result<Position> {
        let rotation = *self
            .rotation
            .first()
            .ok_or("Missing Tenchu 2 story camera rotation")?;
        Ok(crate::math::Matrix {
            rotation: math.rotate_yxz(rotation),
            ..Default::default()
        }
        .transform([0, 0, 4096]))
    }

    pub fn new(animation: &Animation) -> Self {
        Self {
            position: animation.origin.into(),
            rotation: vec![[0; 3]; animation.joints.len()],
            previous: Position::ZERO,
            root: 0,
            joints: vec![0; animation.joints.len()],
        }
    }

    pub fn update(
        &mut self,
        animation: &Animation,
        frame: i16,
        basis: &[[i16; 4]; 33],
    ) -> Result<()> {
        if frame <= animation.duration {
            let (a, b, tangents) = interval(
                &animation.root,
                &mut self.root,
                animation.duration,
                frame,
                true,
            )?;
            let time = i32::from(frame) - i32::from(a.time());
            let duration = i32::from(b.time()) - i32::from(a.time());
            let position = if !b.linear() && duration < 64 {
                let mut a = a;
                a.value = [0; 3];
                let value = spline(a, b, tangents, time, duration, basis);
                Position::from_fn(|axis| i32::from(value[axis]) << b.shift())
            } else {
                Position::from_fn(|axis| {
                    if duration == 0 {
                        i32::from(b.value[axis]) << b.shift()
                    } else {
                        ((i32::from(b.value[axis]) * time) << b.shift()) / duration
                    }
                })
            };
            self.position = self
                .position
                .wrapping_add(position.wrapping_sub(self.previous));
            self.previous = if frame == b.time() {
                Position::ZERO
            } else {
                position
            };
        }
        if self.rotation.len() != animation.joints.len() || self.joints.len() != self.rotation.len()
        {
            return Err("Mismatched Tenchu 2 story skeleton".into());
        }
        let frame = frame.min(animation.duration);
        for ((rotation, segment), keys) in self
            .rotation
            .iter_mut()
            .zip(&mut self.joints)
            .zip(&animation.joints)
        {
            let (a, b, tangents) = interval(keys, segment, animation.duration, frame, false)?;
            let time = i32::from(frame) - i32::from(a.time());
            let duration = i32::from(b.time()) - i32::from(a.time());
            *rotation = if !b.linear() && duration < 64 {
                spline(a, b, tangents, time, duration, basis)
            } else if duration == 0 {
                a.value
            } else {
                std::array::from_fn(|axis| {
                    (i32::from(a.value[axis])
                        + (i32::from(b.value[axis]) - i32::from(a.value[axis])) * time / duration)
                        as i16
                })
            };
        }
        Ok(())
    }
}

fn interval(
    keys: &[Key],
    segment: &mut usize,
    duration: i16,
    frame: i16,
    relative: bool,
) -> Result<(Key, Key, [[i16; 3]; 2])> {
    let valid = |i: usize| {
        keys.get(i)
            .zip(keys.get(i + 1))
            .is_some_and(|(a, b)| (a.time()..=b.time()).contains(&frame))
    };
    if !valid(*segment) {
        *segment = keys
            .partition_point(|key| key.time() < frame)
            .saturating_sub(1);
        if !valid(*segment) {
            return Err("Missing Tenchu 2 story interval".into());
        }
    }
    let i = *segment;
    let a = keys[i];
    let b = keys[i + 1];
    let previous = if a.time() == 0 {
        a
    } else {
        *keys
            .get(i.wrapping_sub(1))
            .ok_or("Missing preceding story key")?
    };
    let next = if b.time() < duration {
        *keys.get(i + 2).ok_or("Missing following story key")?
    } else {
        b
    };
    let delta = (i32::from(b.time()) - i32::from(a.time())) * 256;
    let ratio = |left: Key, right: Key| {
        if left.time() == right.time() {
            0
        } else {
            delta / (i32::from(right.time()) - i32::from(left.time()))
        }
    };
    let left = ratio(previous, b);
    let right = ratio(a, next);
    let tangents = [
        std::array::from_fn(|axis| {
            let value = if relative {
                i32::from(b.value[axis]) + i32::from(a.value[axis])
            } else {
                i32::from(b.value[axis]) - i32::from(previous.value[axis])
            };
            ((left * value) >> 8) as i16
        }),
        std::array::from_fn(|axis| {
            let value = if relative {
                i32::from(next.value[axis]) + i32::from(b.value[axis])
            } else {
                i32::from(next.value[axis]) - i32::from(a.value[axis])
            };
            ((right * value) >> 8) as i16
        }),
    ];
    Ok((a, b, tangents))
}

fn spline(
    a: Key,
    b: Key,
    tangents: [[i16; 3]; 2],
    time: i32,
    duration: i32,
    basis: &[[i16; 4]; 33],
) -> [i16; 3] {
    let fraction = if duration == 0 {
        0
    } else {
        time * 32 / duration
    };
    crate::motion::evaluate(
        tenchu_assets::motion::Key {
            value: a.value,
            time: a.time(),
        },
        tenchu_assets::motion::Key {
            value: b.value,
            time: b.time(),
        },
        tangents,
        basis[fraction as usize],
    )
}
