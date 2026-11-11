use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tenchu_core::tenchu2::effects::surface::{Blend, Blood, Image, Quad};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Landed,
    Faded,
    Changed,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Pending {
    id: usize,
    quad: Quad,
    retained: bool,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct History {
    pending: BTreeMap<([i32; 3], u8), Pending>,
    events: VecDeque<(usize, usize, Phase, Quad)>,
    serial: usize,
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
            if !matches!(quad.blood, Some(Blood::Ground | Blood::Death))
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
                let id = self.serial + events.len() + 1;
                events.push((id, Phase::Landed, quad.clone()));
                Pending {
                    id,
                    quad: quad.clone(),
                    retained: false,
                }
            });
            if quad.blood == Some(Blood::Death)
                && quad.blend == Blend::Opaque
                && quad.corners != entry.quad.corners
            {
                events.push((entry.id, Phase::Changed, quad.clone()));
            }
            if quad.blend == Blend::Add && !entry.retained {
                // Retain the last opaque shape, including the full wall streak.
                let mut last = entry.quad.clone();
                last.color = [128; 3];
                last.blend = Blend::Opaque;
                events.push((entry.id, Phase::Faded, last));
                entry.retained = true;
            }
            entry.quad = quad.clone();
        }
        self.pending.retain(|key, entry| {
            if seen.contains(key) {
                return true;
            }
            if !entry.retained {
                events.push((entry.id, Phase::Faded, entry.quad.clone()));
            }
            false
        });
        for (id, phase, quad) in events {
            self.serial += 1;
            self.events.push_back((self.serial, id, phase, quad));
            if self.events.len() > 65_536 {
                self.events.pop_front();
            }
        }
    }

    pub fn pools(&self) -> impl Iterator<Item = (usize, Phase, &Quad)> {
        self.pending
            .values()
            .filter(|p| p.quad.blood == Some(Blood::Death))
            .map(|p| {
                (
                    p.id,
                    if p.retained {
                        Phase::Faded
                    } else {
                        Phase::Changed
                    },
                    &p.quad,
                )
            })
    }

    pub fn latest(&self) -> usize {
        self.serial
    }

    pub fn since(&self, serial: usize) -> impl Iterator<Item = (usize, Phase, &Quad)> {
        let start = self
            .events
            .partition_point(|(serial_id, _, _, _)| *serial_id <= serial);
        self.events
            .iter()
            .skip(start)
            .map(|(_, id, phase, quad)| (*id, *phase, quad))
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
        assert!(matches!(retained[0].1, Phase::Faded));
        assert_eq!(retained[0].2.corners, quad.corners);
        assert_eq!(retained[0].2.color, [128; 3]);
        history.record(&[faded]);
        history.record(&[]);
        assert_eq!(history.latest(), 2);
        history.record(&[quad.clone()]);
        let mut collapsed = quad.clone();
        collapsed.corners = [quad.corners[0]; 4];
        history.record(&[collapsed]);
        assert_eq!(history.since(3).next().unwrap().2.corners, quad.corners);
        history.record(&[]);
        assert_eq!(
            history.latest(),
            4,
            "eviction before fading must retain the last drawn shape"
        );
        let mut water = ground();
        water.blood = Some(Blood::Water);
        history.record(&[water]);
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
