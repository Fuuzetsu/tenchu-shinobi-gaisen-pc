use super::{Resource, Segment, Snapshot};
use serde::Serialize;
use std::{any::Any, sync::Arc};
use tenchu_assets::{Result, checkpoint::Capture};

struct Cached {
    value: Box<dyn Any>,
    bytes: Arc<Resource>,
    owned_bytes: usize,
}

#[derive(Default)]
pub struct Recorder {
    cache: Vec<Cached>,
    cursor: usize,
    output: Vec<u8>,
    segments: Vec<Segment>,
}

impl Recorder {
    pub fn begin(&mut self) {
        self.cursor = 0;
        self.output.clear();
        self.segments.clear();
    }

    pub fn finish(&mut self) -> Result<Snapshot> {
        self.cache.truncate(self.cursor);
        if self.output.len()
            + self
                .segments
                .iter()
                .map(Segment::extra_bytes)
                .sum::<usize>()
            > 512 * 1024 * 1024
        {
            return Err("State is too large for rewind".into());
        }
        Ok(Snapshot {
            data: std::mem::take(&mut self.output),
            segments: std::mem::take(&mut self.segments),
        })
    }

    pub fn bytes(&self, shared: impl Fn(usize) -> bool) -> usize {
        self.output.capacity()
            + self.cache.capacity() * std::mem::size_of::<Cached>()
            + self
                .cache
                .iter()
                .map(|entry| {
                    entry.owned_bytes
                        + if shared(Arc::as_ptr(&entry.bytes) as usize) {
                            0
                        } else {
                            entry.bytes.size()
                        }
                })
                .sum::<usize>()
    }

    pub fn recycle(&mut self, mut bytes: Vec<u8>) {
        bytes.clear();
        self.output = bytes;
    }

    fn cached<T: 'static>(
        &mut self,
        stable: bool,
        owns_value: bool,
        matches: impl FnOnce(&T) -> bool,
        make: impl FnOnce() -> Result<(T, Vec<u8>)>,
    ) -> Result<()> {
        if self.cache.get(self.cursor).is_none_or(|entry| {
            entry
                .value
                .downcast_ref::<T>()
                .is_none_or(|old| !matches(old))
        }) {
            let (value, bytes) = make()?;
            let entry = Cached {
                owned_bytes: std::mem::size_of::<T>()
                    + if owns_value { bytes.len() * 2 } else { 0 },
                value: Box::new(value),
                bytes: Arc::new(Resource::new(bytes, stable)),
            };
            if self.cursor == self.cache.len() {
                self.cache.push(entry);
            } else {
                self.cache[self.cursor] = entry;
            }
        }
        if stable {
            self.segments.push(Segment::Shared {
                offset: self.output.len(),
                bytes: self.cache[self.cursor].bytes.clone(),
            });
        } else {
            self.cache[self.cursor].bytes.append(&mut self.output)?;
        }
        self.cursor += 1;
        Ok(())
    }
}

impl Capture for Recorder {
    type Error = String;

    fn value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.output = postcard::to_extend(value, std::mem::take(&mut self.output))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn shared<T: Serialize + Clone + PartialEq + 'static>(&mut self, value: &T) -> Result<()> {
        self.cached(
            true,
            true,
            |old: &T| old == value,
            || {
                Ok((
                    value.clone(),
                    postcard::to_allocvec(value).map_err(|e| e.to_string())?,
                ))
            },
        )
    }

    fn words(&mut self, value: &[u16]) -> Result<()> {
        self.value(&value.len())?;
        let start = self.output.len();
        self.output.extend_from_slice(bytemuck::cast_slice(value));
        self.segments.push(Segment::Words(start..self.output.len()));
        Ok(())
    }

    fn shared_arc<T: Serialize + 'static>(&mut self, value: &Arc<T>, stable: bool) -> Result<()> {
        let matches = |entry: &Cached| {
            entry
                .value
                .downcast_ref::<Arc<T>>()
                .is_some_and(|old| Arc::ptr_eq(old, value))
        };
        // Expiring a blood page shifts the remaining resources, not their data.
        if stable
            && self.cache.get(self.cursor).is_some_and(|old| !matches(old))
            && let Some(offset) = self.cache[self.cursor + 1..].iter().position(matches)
        {
            self.cache.swap(self.cursor, self.cursor + 1 + offset);
        }
        self.cached(
            stable,
            false,
            |old: &Arc<T>| Arc::ptr_eq(old, value),
            || {
                Ok((
                    value.clone(),
                    postcard::to_allocvec(value).map_err(|e| e.to_string())?,
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rewind::History;

    #[test]
    fn expiring_and_reordering_shared_resources_keeps_their_encoded_data() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Tracked {
            value: u32,
            encodes: Arc<AtomicUsize>,
        }
        impl Serialize for Tracked {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                self.encodes.fetch_add(1, Ordering::Relaxed);
                self.value.serialize(serializer)
            }
        }
        let encodes = Arc::new(AtomicUsize::new(0));
        let resources: Vec<_> = (0..9)
            .map(|value| {
                Arc::new(Tracked {
                    value,
                    encodes: encodes.clone(),
                })
            })
            .collect();
        let orders = [
            vec![0, 1, 2, 3, 4, 5, 6, 7],
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![7, 6, 5, 4, 3, 2, 1],
            vec![7, 6, 4, 3, 2, 1, 8],
            vec![8, 1, 2, 3, 4, 6, 7],
        ];
        let mut history = History::default();
        for (tick, order) in orders.iter().enumerate() {
            let tick = tick as u64;
            history.recorder.begin();
            history.recorder.value(&tick).unwrap();
            history.recorder.value(&order.len()).unwrap();
            for &index in order {
                history
                    .recorder
                    .shared_arc(&resources[index], true)
                    .unwrap();
            }
            let snapshot = history.recorder.finish().unwrap();
            history.push(tick, 2, snapshot).unwrap();
            assert_eq!(
                encodes.load(Ordering::Relaxed),
                if tick < 3 { 8 } else { 9 }
            );
        }
        for (tick, order) in orders.into_iter().enumerate() {
            let expected: Vec<_> = order
                .into_iter()
                .map(|index| resources[index].value)
                .collect();
            let restored: (u64, Vec<u32>) = history.snapshot(tick).unwrap().decode().unwrap();
            assert_eq!(restored, (tick as u64, expected));
        }
    }

    #[test]
    fn shared_resources_packed_words_and_variable_segments_survive_eviction_and_branching() {
        let mut history = History::default();
        let mut resource = Arc::new(vec![19u32; 20_000]);
        let mut expected = Vec::new();
        for tick in 0..120 {
            let prefix = vec![tick as u8; tick % 9];
            let words: Vec<_> = (0..=u16::MAX).step_by(7).skip(tick % 3).collect();
            if tick % 40 == 0 {
                Arc::make_mut(&mut resource)[tick] += 1;
            }
            let recorder = &mut history.recorder;
            recorder.begin();
            recorder.value(&prefix).unwrap();
            recorder.words(&words).unwrap();
            recorder.shared_arc(&resource, true).unwrap();
            recorder.value(&tick).unwrap();
            let snapshot = recorder.finish().unwrap();
            let bytes = postcard::to_allocvec(&(&prefix, &words, &resource, tick)).unwrap();
            assert_eq!(snapshot.bytes().unwrap(), bytes);
            expected.push(bytes);
            history.push(tick as u64, 2, snapshot).unwrap();
        }
        assert!(history.entries.len() <= 61);
        assert!(history.shared.len() <= 3);
        let unshared: usize = history
            .entries
            .iter()
            .map(|e| expected[e.tick as usize].len())
            .sum();
        assert!(history.bytes < unshared / 2);
        let weak = Arc::downgrade(
            history
                .entries
                .front()
                .unwrap()
                .segments
                .iter()
                .find_map(Segment::resource)
                .unwrap(),
        );
        for index in (0..history.entries.len()).rev().chain([5, 20, 2, 35]) {
            let tick = history.entries[index].tick as usize;
            assert_eq!(
                history.snapshot(index).unwrap().bytes().unwrap(),
                expected[tick]
            );
        }
        let before = history.bytes;
        history.branch(20).unwrap();
        assert!(history.bytes < before);
        history
            .push(120, 2, Snapshot::encode(&42u64).unwrap())
            .unwrap();
        assert_eq!(
            history
                .snapshot(history.entries.len() - 1)
                .unwrap()
                .decode::<u64>()
                .unwrap(),
            42
        );
        history.trim(0, 100);
        assert_eq!(history.entries.len(), 1);
        assert!(history.shared.is_empty());
        assert!(history.bytes <= 100);
        history.clear();
        assert!(weak.upgrade().is_none());
        assert_eq!(history.bytes, 0);
    }
}
