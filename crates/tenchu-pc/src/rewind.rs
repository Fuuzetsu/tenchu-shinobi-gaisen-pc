#[cfg(test)]
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tenchu_assets::Result;

mod capture;
pub use capture::Recorder;

const MEMORY_LIMIT: usize = 96 * 1024 * 1024;
const INTERVAL: u64 = tenchu_core::TICKS_PER_SECOND as u64;

struct Resource {
    bytes: Box<[u8]>,
    decoded_length: usize,
}

impl Resource {
    fn new(bytes: Vec<u8>, stable: bool) -> Self {
        let decoded_length = bytes.len();
        let stored = stable
            .then(|| lz4_flex::compress(&bytes))
            .filter(|b| b.len() < bytes.len())
            .unwrap_or(bytes);
        Self {
            bytes: stored.into_boxed_slice(),
            decoded_length,
        }
    }

    fn append(&self, output: &mut Vec<u8>) -> Result<()> {
        if self.bytes.len() == self.decoded_length {
            output.extend_from_slice(&self.bytes);
        } else {
            let start = output.len();
            output.resize(start + self.decoded_length, 0);
            lz4_flex::decompress_into(&self.bytes, &mut output[start..])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn size(&self) -> usize {
        self.bytes.len() + std::mem::size_of::<Self>()
    }
}

#[derive(Clone)]
enum Segment {
    Shared { offset: usize, bytes: Arc<Resource> },
    Words(std::ops::Range<usize>),
}

impl Segment {
    fn resource(&self) -> Option<&Arc<Resource>> {
        match self {
            Self::Shared { bytes, .. } => Some(bytes),
            Self::Words(_) => None,
        }
    }

    fn extra_bytes(&self) -> usize {
        match self {
            Self::Shared { bytes, .. } => bytes.decoded_length,
            Self::Words(range) => range.len() / 2,
        }
    }
}

#[derive(Default)]
pub struct Snapshot {
    data: Vec<u8>,
    segments: Vec<Segment>,
}

impl Snapshot {
    #[cfg(test)]
    pub fn encode(value: &impl Serialize) -> Result<Self> {
        Ok(Self {
            data: postcard::to_allocvec(value).map_err(|e| e.to_string())?,
            segments: Vec::new(),
        })
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Result<T> {
        if self.segments.is_empty() {
            postcard::from_bytes(&self.data).map_err(|e| e.to_string())
        } else {
            postcard::from_bytes(&self.bytes()?).map_err(|e| e.to_string())
        }
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        let size = self.data.len()
            + self
                .segments
                .iter()
                .map(Segment::extra_bytes)
                .sum::<usize>();
        let mut bytes = Vec::with_capacity(size);
        let mut offset = 0;
        for segment in &self.segments {
            let start = match segment {
                Segment::Shared { offset, .. } => *offset,
                Segment::Words(range) => range.start,
            };
            bytes.extend_from_slice(&self.data[offset..start]);
            match segment {
                Segment::Shared { bytes: shared, .. } => {
                    shared.append(&mut bytes)?;
                    offset = start;
                }
                Segment::Words(range) => {
                    // VRAM is copied in bulk while recording; expand its varints only on restore.
                    for word in self.data[range.clone()].chunks_exact(2) {
                        let mut value = u16::from_ne_bytes([word[0], word[1]]);
                        while value >= 128 {
                            bytes.push(value as u8 | 128);
                            value >>= 7;
                        }
                        bytes.push(value as u8);
                    }
                    offset = range.end;
                }
            }
        }
        bytes.extend_from_slice(&self.data[offset..]);
        Ok(bytes)
    }

    fn apply(&mut self, block: &Block, length: usize, scratch: &mut Vec<u8>) -> Result<()> {
        scratch.resize(block.encoded_length, 0);
        lz4_flex::decompress_into(&block.bytes, scratch).map_err(|e| e.to_string())?;
        self.data.resize(scratch.len(), 0);
        for (value, delta) in self.data.iter_mut().zip(scratch) {
            *value ^= *delta;
        }
        self.data.truncate(length);
        Ok(())
    }
}

struct Block {
    bytes: Box<[u8]>,
    encoded_length: usize,
}

impl Block {
    fn between(current: &[u8], previous: &mut Vec<u8>) -> Self {
        previous.resize(previous.len().max(current.len()), 0);
        for (value, now) in previous.iter_mut().zip(current) {
            *value ^= now;
        }
        Self {
            bytes: lz4_flex::compress(previous).into_boxed_slice(),
            encoded_length: previous.len(),
        }
    }
}

pub struct Entry {
    pub tick: u64,
    length: usize,
    before: Option<Block>,
    segments: Vec<Segment>,
}

impl Entry {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.segments.capacity() * std::mem::size_of::<Segment>()
            + self.before.as_ref().map_or(0, |b| b.bytes.len())
    }
}

#[derive(Default)]
pub struct History {
    pub entries: VecDeque<Entry>,
    pub bytes: usize,
    shared: HashMap<usize, (usize, usize)>,
    tail: Snapshot,
    cursor: Option<(usize, Snapshot)>,
    scratch: Vec<u8>,
    pub recorder: Recorder,
}

impl History {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn due(&self, tick: u64) -> bool {
        self.entries.back().is_none_or(|entry| tick > entry.tick)
    }

    pub fn push(&mut self, tick: u64, seconds: u16, mut state: Snapshot) -> Result<()> {
        self.cursor = None;
        let largest = self
            .entries
            .iter()
            .map(|e| e.length)
            .max()
            .unwrap_or(0)
            .max(state.data.len());
        let buffers =
            state.data.capacity() + self.tail.data.capacity() + self.scratch.capacity() + largest;
        let budget = MEMORY_LIMIT.saturating_sub(buffers + self.recorder.bytes());
        let before =
            (!self.entries.is_empty()).then(|| Block::between(&state.data, &mut self.tail.data));
        let entry = Entry {
            tick,
            length: state.data.len(),
            before,
            segments: std::mem::take(&mut state.segments),
        };
        self.recorder
            .recycle(std::mem::replace(&mut self.tail, state).data);
        self.bytes += entry.bytes();
        for resource in entry.segments.iter().filter_map(Segment::resource) {
            let id = Arc::as_ptr(resource) as usize;
            let count = self.shared.entry(id).or_insert_with(|| {
                self.bytes += resource.size();
                (0, resource.size())
            });
            count.0 += 1;
        }
        self.entries.push_back(entry);
        let oldest = tick.saturating_sub(u64::from(seconds.min(30)) * INTERVAL);
        self.trim(oldest, budget);
        if self.bytes > budget {
            self.clear();
            return Err("State exceeds the 96 MiB rewind memory limit".into());
        }
        Ok(())
    }

    fn remove(&mut self, entry: Entry) {
        self.bytes -= entry.bytes();
        for resource in entry.segments.iter().filter_map(Segment::resource) {
            let id = Arc::as_ptr(resource) as usize;
            let count = self.shared.get_mut(&id).unwrap();
            count.0 -= 1;
            if count.0 == 0 {
                self.bytes -= count.1;
                self.shared.remove(&id);
            }
        }
    }

    fn trim(&mut self, oldest: u64, limit: usize) {
        while self.entries.len() > 1
            && (self.bytes > limit || self.entries.front().is_some_and(|e| e.tick < oldest))
        {
            let entry = self.entries.pop_front().unwrap();
            self.remove(entry);
            if let Some(before) = self.entries.front_mut().unwrap().before.take() {
                self.bytes -= before.bytes.len();
            }
            self.cursor = None;
        }
    }

    pub fn snapshot(&mut self, index: usize) -> Result<&Snapshot> {
        if index >= self.entries.len() {
            return Err("Rewind position is unavailable".into());
        }
        let (mut at, mut state) = self
            .cursor
            .take()
            .filter(|(at, _)| at.abs_diff(index) <= self.entries.len() - 1 - index)
            .unwrap_or_else(|| {
                (
                    self.entries.len() - 1,
                    Snapshot {
                        data: self.tail.data.clone(),
                        segments: Vec::new(),
                    },
                )
            });
        while at > index {
            state.apply(
                self.entries[at].before.as_ref().unwrap(),
                self.entries[at - 1].length,
                &mut self.scratch,
            )?;
            at -= 1;
        }
        while at < index {
            at += 1;
            state.apply(
                self.entries[at].before.as_ref().unwrap(),
                self.entries[at].length,
                &mut self.scratch,
            )?;
        }
        state.segments.clone_from(&self.entries[index].segments);
        self.cursor = Some((index, state));
        Ok(&self.cursor.as_ref().unwrap().1)
    }

    pub fn previous(&self, tick: u64) -> Option<usize> {
        self.entries.iter().rposition(|entry| entry.tick < tick)
    }

    pub fn branch(&mut self, index: usize) -> Result<()> {
        self.snapshot(index)?;
        let (_, mut state) = self.cursor.take().unwrap();
        state.segments.clear();
        self.recorder
            .recycle(std::mem::replace(&mut self.tail, state).data);
        while self.entries.len() > index + 1 {
            let entry = self.entries.pop_back().unwrap();
            self.remove(entry);
        }
        Ok(())
    }

    pub fn ticks(&self, tick: u64) -> u64 {
        self.entries
            .front()
            .map_or(0, |s| tick.saturating_sub(s.tick))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_deltas_restore_variable_length_states_and_branch_after_eviction() {
        let mut seed = 1u32;
        let base: Vec<u8> = (0..200000)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        let mut history = History::default();
        let mut expected = Vec::new();
        for tick in 0..120u64 {
            let mut value = base.clone();
            value.truncate(value.len() - (tick as usize % 7) * 51);
            value[tick as usize * 17] ^= 1;
            history
                .push(tick, 2, Snapshot::encode(&value).unwrap())
                .unwrap();
            expected.push(value);
        }
        assert!(history.bytes < base.len() * 5);
        assert!(history.entries.front().unwrap().before.is_none());
        for index in (0..history.entries.len()).rev() {
            let tick = history.entries[index].tick;
            assert_eq!(
                history
                    .snapshot(index)
                    .unwrap()
                    .decode::<Vec<u8>>()
                    .unwrap(),
                expected[tick as usize]
            );
        }
        for index in [5, 13, 0, 29, 7] {
            let tick = history.entries[index].tick;
            assert_eq!(
                history
                    .snapshot(index)
                    .unwrap()
                    .decode::<Vec<u8>>()
                    .unwrap(),
                expected[tick as usize]
            );
        }
        history.branch(10).unwrap();
        let tick = history.entries.back().unwrap().tick;
        let replacement = vec![123u8; 100];
        history
            .push(tick + 1, 2, Snapshot::encode(&replacement).unwrap())
            .unwrap();
        let last = history.entries.len() - 1;
        assert_eq!(
            history.snapshot(last).unwrap().decode::<Vec<u8>>().unwrap(),
            replacement
        );
        assert_eq!(
            history
                .snapshot(last - 1)
                .unwrap()
                .decode::<Vec<u8>>()
                .unwrap(),
            expected[tick as usize]
        );
    }
}
