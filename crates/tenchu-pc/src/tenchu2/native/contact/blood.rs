use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tenchu_core::tenchu2::effects::surface::{Blend, Blood, Image, Quad};

#[derive(Clone, Copy)]
pub enum Phase {
    Landed,
    Faded,
}

struct Pending {
    quad: Quad,
    retained: bool,
}

#[derive(Default)]
pub struct History {
    pending: BTreeMap<([i32; 3], u8), Pending>,
    events: VecDeque<(u64, Phase, Quad)>,
    serial: u64,
}

impl History {
    pub fn reset(&mut self) {
        self.pending.clear();
        self.events.clear();
    }

    pub fn record(&mut self, quads: &[Quad]) {
        let mut seen = BTreeSet::new();
        let mut events = Vec::new();
        for quad in quads {
            if quad.blood != Some(Blood::Ground)
                || quad.parent.is_some()
                || quad.blend == Blend::SubtractMask
            {
                continue;
            }
            let Image::Effect(image) = quad.image else {
                continue;
            };
            let corners = quad
                .corners
                .map(|p| glam::Vec3::from(p.to_array().map(f32::from)));
            if (corners[1] - corners[0])
                .cross(corners[2] - corners[0])
                .length_squared()
                == 0.0
            {
                continue;
            }
            let key = (quad.position.to_array(), image);
            if !seen.insert(key) {
                continue;
            }
            let entry = self.pending.entry(key).or_insert_with(|| {
                events.push((Phase::Landed, quad.clone()));
                Pending {
                    quad: quad.clone(),
                    retained: false,
                }
            });
            if quad.blend == Blend::Add && !entry.retained {
                // Retain the last opaque shape, including the full wall streak.
                let mut last = entry.quad.clone();
                last.color = [128; 3];
                last.blend = Blend::Opaque;
                events.push((Phase::Faded, last));
                entry.retained = true;
            }
            entry.quad = quad.clone();
        }
        self.pending.retain(|key, entry| {
            if seen.contains(key) {
                return true;
            }
            if !entry.retained {
                events.push((Phase::Faded, entry.quad.clone()));
            }
            false
        });
        for (phase, quad) in events {
            self.serial += 1;
            self.events.push_back((self.serial, phase, quad));
            if self.events.len() > 65_536 {
                self.events.pop_front();
            }
        }
    }

    pub fn latest(&self) -> u64 {
        self.serial
    }

    pub fn since(&self, serial: u64) -> impl Iterator<Item = (Phase, &Quad)> {
        let start = self.events.partition_point(|(id, _, _)| *id <= serial);
        self.events
            .iter()
            .skip(start)
            .map(|(_, phase, quad)| (*phase, quad))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::math::{Position, Vector3};

    fn ground() -> Quad {
        Quad {
            blood: Some(Blood::Ground),
            image: Image::Effect(2),
            position: Position::ZERO,
            parent: None,
            corners: [
                [-100, 0, -100],
                [100, 0, -100],
                [-100, 0, 100],
                [100, 0, 100],
            ]
            .map(Vector3::from),
            color: [128; 3],
            blend: Blend::Opaque,
        }
    }

    #[test]
    fn records_landings_and_full_streaks_once_even_when_source_slots_are_evicted() {
        let mut history = History::default();
        let mut quad = ground();
        history.record(&[quad.clone()]);
        assert_eq!(history.latest(), 1);
        for _ in 0..30 {
            history.record(&[quad.clone()]);
        }
        assert_eq!(history.latest(), 1);
        quad.corners[2].y = 30;
        quad.corners[3].y = 30;
        history.record(&[quad.clone()]);
        let mut faded = quad.clone();
        faded.color = [64; 3];
        faded.blend = Blend::Add;
        let mut mask = faded.clone();
        mask.blend = Blend::SubtractMask;
        history.record(&[faded.clone(), mask]);
        let retained: Vec<_> = history.since(1).collect();
        assert_eq!(retained.len(), 1);
        assert!(matches!(retained[0].0, Phase::Faded));
        assert_eq!(retained[0].1.corners, quad.corners);
        assert_eq!(retained[0].1.color, [128; 3]);
        history.record(&[faded]);
        history.record(&[]);
        assert_eq!(history.latest(), 2);
        history.record(&[quad.clone()]);
        let mut collapsed = quad.clone();
        collapsed.corners = [quad.corners[0]; 4];
        history.record(&[collapsed]);
        assert_eq!(history.since(3).next().unwrap().1.corners, quad.corners);
        history.record(&[]);
        assert_eq!(
            history.latest(),
            4,
            "eviction before fading must retain the last drawn shape"
        );
        let mut water = ground();
        water.blood = Some(Blood::Water);
        let mut death = ground();
        death.blood = Some(Blood::Death);
        history.record(&[water, death]);
        assert_eq!(history.latest(), 4);
        history.reset();
        assert!(history.since(0).next().is_none());
        history.record(&[ground()]);
        assert_eq!(
            history.since(4).count(),
            1,
            "reset must keep event serials monotonic"
        );
    }
}
