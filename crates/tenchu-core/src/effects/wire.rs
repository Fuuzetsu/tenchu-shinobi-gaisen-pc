use super::*;
use crate::math::Math;
use crate::math::Position;

#[derive(Clone, Debug)]
pub struct FlyWire {
    pub start: Position,
    pub end: Position,
    pub center: Position,
    pub straight: Position,
    pub count: i16,
    pub time: i16,
    pub mode: u8,
    segments: i32,
    rotation: [[i16; 3]; 3],
}

#[derive(Clone, Debug)]
pub struct Wire {
    pub lines: Vec<([[i16; 2]; 2], i16)>,
    pub points: Vec<Position>,
    pub hook: Option<(Matrix, bool)>,
    pub world_hook: Option<Matrix>,
}

impl Effects {
    pub fn fly_wire(
        &mut self,
        start: Position,
        end: Position,
        math: &Math,
        rng: &mut Random,
    ) -> Result<i16> {
        let slot = self.allocate();
        let distance = math.vector_distance(start.wrapping_sub(end));
        let straight = std::array::from_fn(|i| start[i].wrapping_add(end[i]) / 2);
        let time = (distance / 1000) as i16;
        let bend = distance / 16;
        let center = Position::from_fn(|i| {
            let range = if i == 1 { bend } else { bend.wrapping_mul(2) };
            straight[i]
                .wrapping_add(if range > 0 { rng.next15() % range } else { 0 })
                .wrapping_sub(bend)
        });
        if time > 0 {
            self.slots[slot] = Some(Effect::FlyWire(FlyWire {
                start,
                end,
                center,
                straight: straight.into(),
                count: 0,
                time,
                mode: 0,
                segments: distance / 300,
                rotation: math.rotate_yxz(math.vector_rotation(start, end)?),
            }));
            Ok(time.wrapping_add(5))
        } else {
            Ok(0)
        }
    }

    pub fn wire(
        &mut self,
        start: Position,
        end: Position,
        math: &Math,
        view: View,
    ) -> Result<Option<bool>> {
        let distance = math.vector_distance(start.wrapping_sub(end));
        let mut center = std::array::from_fn(|i| start[i].wrapping_add(end[i]) / 2);
        center[1] = center[1].wrapping_add(distance / 32);
        let wire = sample(
            start,
            end,
            center.into(),
            distance / 300,
            4096,
            math.rotate_yxz(math.vector_rotation(start, end)?),
            view,
        );
        let fog = wire.hook.map(|(_, fog)| fog);
        self.wires.push(wire);
        Ok(fog)
    }

    pub(super) fn update_wire(
        &mut self,
        wire: &mut FlyWire,
        rng: &mut Random,
        view: View,
        owner_position: Position,
    ) -> bool {
        let (center, length, alive) = if wire.mode == 0 {
            wire.count = (i32::from(wire.count as u16) + 4096 / i32::from(wire.time)) as i16;
            if wire.count > 4096 {
                wire.count = 0;
                wire.mode = 1;
                self.scatter_bleeds(
                    wire.end,
                    50,
                    crate::effects::Burst {
                        spread: 0,
                        count: 10,
                        lifetime: 30,
                        color: [255, 255, 0],
                    },
                    rng,
                );
                self.sounds.push((owner_position, 0x31));
                return true;
            }
            (wire.center, wire.count, true)
        } else {
            let count = i32::from(wire.count);
            let center = Position::from_fn(|i| {
                wire.center[i]
                    .wrapping_mul(5 - count)
                    .wrapping_add(wire.straight[i].wrapping_mul(count))
                    / 5
            });
            let alive = wire.count < 5;
            wire.count = wire.count.wrapping_add(1);
            (center, 4096, alive)
        };
        self.wires.push(sample(
            wire.start,
            wire.end,
            center,
            wire.segments,
            length,
            wire.rotation,
            view,
        ));
        alive
    }
}

fn sample(
    start: Position,
    end: Position,
    center: Position,
    segments: i32,
    length: i16,
    rotation: [[i16; 3]; 3],
    view: View,
) -> Wire {
    let count = segments.wrapping_mul(i32::from(length)) / 4096;
    let mut previous = view.project(start);
    let mut lines = Vec::new();
    let mut points = vec![start];
    let mut tail = None;
    for i in 0..count {
        let t = 4096 - i * 4096 / segments;
        let q = t * 2;
        let r = t * t / 4096;
        let point = std::array::from_fn(|j| {
            (4096i32 - q + r)
                .wrapping_mul(end[j])
                .wrapping_add((q - r * 2).wrapping_mul(center[j]))
                .wrapping_add(r.wrapping_mul(start[j]))
                / 4096
        });
        let current = view.project(point.into());
        points.push(point.into());
        if current.1 > 0 && previous.1 > 0 {
            lines.push(([previous.0, current.0], (current.1 >> 2).clamp(0, 1249)));
        }
        previous = current;
        tail = Some(point);
    }
    // Empty curves leave retail stack locals undefined; do not manufacture a hook position.
    let world_hook = tail.map(|translation| Matrix {
        rotation,
        translation: translation.into(),
    });
    let hook = world_hook.and_then(|matrix| {
        visibility::clip(&view.matrix.compose(&matrix), 0, [0; 3]).map(|clip| (matrix, clip.fog))
    });
    Wire {
        lines,
        points,
        hook,
        world_hook,
    }
}

#[cfg(test)]
mod tests;
