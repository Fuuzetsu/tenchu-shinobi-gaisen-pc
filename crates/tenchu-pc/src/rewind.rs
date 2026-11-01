use serde::{Serialize, de::DeserializeOwned};
use std::collections::VecDeque;
use tenchu_assets::Result;

const MEMORY_LIMIT: usize = 96 * 1024 * 1024;
const INTERVAL: u64 = tenchu_core::TICKS_PER_SECOND as u64;

#[derive(Default)]
pub struct Snapshot(Vec<u8>);

impl Snapshot {
    pub fn encode(value: &impl Serialize) -> Result<Self> {
        let payload = postcard::to_allocvec(value).map_err(|e| e.to_string())?;
        if payload.len() > 512 * 1024 * 1024 {
            return Err("State is too large for rewind".into());
        }
        Ok(Self(payload))
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Result<T> {
        postcard::from_bytes(&self.0).map_err(|e| e.to_string())
    }

    fn apply(&mut self, block: &Block, length: usize) -> Result<()> {
        let bytes =
            lz4_flex::decompress(&block.bytes, block.encoded_length).map_err(|e| e.to_string())?;
        if block.key {
            self.0 = bytes;
        } else {
            self.0.resize(bytes.len(), 0);
            for (value, delta) in self.0.iter_mut().zip(bytes) {
                *value ^= delta;
            }
            self.0.truncate(length);
        }
        Ok(())
    }
}

struct Block {
    bytes: Box<[u8]>,
    encoded_length: usize,
    length: usize,
    key: bool,
}

impl Block {
    fn new(current: &Snapshot, previous: Option<&Snapshot>) -> Self {
        let delta = previous.map(|previous| {
            let mut bytes = current.0.clone();
            bytes.resize(bytes.len().max(previous.0.len()), 0);
            for (value, before) in bytes.iter_mut().zip(&previous.0) {
                *value ^= before;
            }
            bytes
        });
        let bytes = delta.as_deref().unwrap_or(&current.0);
        Self {
            bytes: lz4_flex::compress(bytes).into_boxed_slice(),
            encoded_length: bytes.len(),
            length: current.0.len(),
            key: previous.is_none(),
        }
    }
}

pub struct Entry {
    pub tick: u64,
    block: Block,
}

#[derive(Default)]
pub struct History {
    pub entries: VecDeque<Entry>,
    pub bytes: usize,
    tail: Snapshot,
    cursor: Option<(usize, Snapshot)>,
}

impl History {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn due(&self, tick: u64) -> bool {
        self.entries.back().is_none_or(|entry| tick > entry.tick)
    }

    pub fn push(&mut self, tick: u64, seconds: u16, state: Snapshot) -> Result<()> {
        let largest = self
            .entries
            .iter()
            .map(|e| e.block.length)
            .max()
            .unwrap_or(0)
            .max(state.0.len());
        let budget = MEMORY_LIMIT.saturating_sub(largest * 2);
        let key = self
            .entries
            .iter()
            .rev()
            .find(|e| e.block.key)
            .is_none_or(|e| tick.saturating_sub(e.tick) >= INTERVAL);
        let block = Block::new(&state, (!key).then_some(&self.tail));
        if block.bytes.len() > budget {
            return Err("State exceeds the 96 MiB rewind memory limit".into());
        }
        self.cursor = None;
        self.tail = state;
        self.bytes += block.bytes.len();
        self.entries.push_back(Entry { tick, block });
        let oldest = tick.saturating_sub(u64::from(seconds.min(30)) * INTERVAL);
        self.trim(oldest, budget);
        if self.bytes > budget {
            self.clear();
            return Err("State exceeds the 96 MiB rewind memory limit".into());
        }
        Ok(())
    }

    fn trim(&mut self, oldest: u64, limit: usize) {
        while self.bytes > limit || self.entries.front().is_some_and(|s| s.tick < oldest) {
            if let Some(next) = self.entries.iter().skip(1).position(|e| e.block.key) {
                for _ in 0..=next {
                    self.bytes -= self.entries.pop_front().unwrap().block.bytes.len();
                }
            } else {
                let tick = self.entries.back().unwrap().tick;
                let block = Block::new(&self.tail, None);
                self.bytes = block.bytes.len();
                self.entries = VecDeque::from([Entry { tick, block }]);
                break;
            }
            self.cursor = None;
        }
    }

    pub fn snapshot(&mut self, index: usize) -> Result<&Snapshot> {
        if index >= self.entries.len() {
            return Err("Rewind position is unavailable".into());
        }
        if self.cursor.as_ref().is_none_or(|(at, _)| *at != index) {
            let next = self.cursor.take();
            let state = if let Some((at, mut state)) = next
                && at == index + 1
                && !self.entries[at].block.key
            {
                state.apply(&self.entries[at].block, self.entries[index].block.length)?;
                state
            } else {
                let key = self
                    .entries
                    .iter()
                    .take(index + 1)
                    .rposition(|e| e.block.key)
                    .ok_or("Rewind keyframe is unavailable")?;
                let mut state = Snapshot::default();
                for entry in self.entries.range(key..=index) {
                    state.apply(&entry.block, entry.block.length)?;
                }
                state
            };
            self.cursor = Some((index, state));
        }
        Ok(&self.cursor.as_ref().unwrap().1)
    }

    pub fn previous(&self, tick: u64) -> Option<usize> {
        self.entries.iter().rposition(|entry| entry.tick < tick)
    }

    pub fn branch(&mut self, index: usize) -> Result<()> {
        self.tail = Snapshot(self.snapshot(index)?.0.clone());
        while self.entries.len() > index + 1 {
            self.bytes -= self.entries.pop_back().unwrap().block.bytes.len();
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
    fn memory_eviction_and_branching_keep_complete_decodable_checkpoints() {
        let mut history = History::default();
        for tick in [0, 30, 60, 90] {
            history
                .push(tick, 30, Snapshot::encode(&vec![tick; 200]).unwrap())
                .unwrap();
        }
        let capacity = history
            .entries
            .iter()
            .rev()
            .take(2)
            .map(|e| e.block.bytes.len())
            .sum();
        history.trim(0, capacity);
        assert_eq!(
            history.entries.iter().map(|e| e.tick).collect::<Vec<_>>(),
            [60, 90]
        );
        assert_eq!(history.bytes, capacity);
        history.branch(0).unwrap();
        assert_eq!(history.bytes, history.entries[0].block.bytes.len());
        assert_eq!(
            history.snapshot(0).unwrap().decode::<Vec<u64>>().unwrap(),
            vec![60; 200]
        );
        assert_eq!(history.previous(60), None);
        assert_eq!(history.previous(61), Some(0));
        history.clear();
        assert_eq!(history.bytes, 0);
    }

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
        assert!(history.entries.front().unwrap().block.key);
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
