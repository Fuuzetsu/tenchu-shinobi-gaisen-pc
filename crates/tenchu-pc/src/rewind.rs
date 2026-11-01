use serde::{Serialize, de::DeserializeOwned};
use std::collections::VecDeque;
use tenchu_assets::Result;

const MEMORY_LIMIT: usize = 96 * 1024 * 1024;
const INTERVAL: u64 = tenchu_core::TICKS_PER_SECOND as u64;

pub struct Snapshot {
    bytes: Box<[u8]>,
    length: usize,
}

impl Snapshot {
    pub fn encode(value: &impl Serialize) -> Result<Self> {
        let payload = postcard::to_allocvec(value).map_err(|e| e.to_string())?;
        if payload.len() > 512 * 1024 * 1024 {
            return Err("State is too large for rewind".into());
        }
        Ok(Self {
            bytes: lz4_flex::compress(&payload).into_boxed_slice(),
            length: payload.len(),
        })
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Result<T> {
        let bytes = lz4_flex::decompress(&self.bytes, self.length).map_err(|e| e.to_string())?;
        postcard::from_bytes(&bytes).map_err(|e| e.to_string())
    }
}

pub struct Entry {
    pub tick: u64,
    pub state: Snapshot,
}

#[derive(Default)]
pub struct History {
    pub entries: VecDeque<Entry>,
    pub bytes: usize,
}

impl History {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    pub fn due(&self, tick: u64) -> bool {
        self.entries
            .back()
            .is_none_or(|entry| tick.saturating_sub(entry.tick) >= INTERVAL)
    }

    pub fn push(&mut self, tick: u64, seconds: u16, state: Snapshot) -> Result<()> {
        if state.bytes.len() > MEMORY_LIMIT {
            return Err("State exceeds the 96 MiB rewind memory limit".into());
        }
        self.bytes += state.bytes.len();
        self.entries.push_back(Entry { tick, state });
        let oldest = tick.saturating_sub(u64::from(seconds.min(30)) * INTERVAL);
        self.trim(oldest, MEMORY_LIMIT);
        Ok(())
    }

    fn trim(&mut self, oldest: u64, limit: usize) {
        while self.bytes > limit || self.entries.front().is_some_and(|s| s.tick < oldest) {
            self.bytes -= self.entries.pop_front().unwrap().state.bytes.len();
        }
    }

    pub fn previous(&self, tick: u64) -> Option<usize> {
        self.entries.iter().rposition(|entry| entry.tick < tick)
    }

    pub fn branch(&mut self, index: usize) {
        while self.entries.len() > index + 1 {
            self.bytes -= self.entries.pop_back().unwrap().state.bytes.len();
        }
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
            .map(|e| e.state.bytes.len())
            .sum();
        history.trim(0, capacity);
        assert_eq!(
            history.entries.iter().map(|e| e.tick).collect::<Vec<_>>(),
            [60, 90]
        );
        assert_eq!(history.bytes, capacity);
        history.branch(0);
        assert_eq!(history.bytes, history.entries[0].state.bytes.len());
        assert_eq!(
            history.entries[0].state.decode::<Vec<u64>>().unwrap(),
            vec![60; 200]
        );
        assert_eq!(history.previous(60), None);
        assert_eq!(history.previous(61), Some(0));
        history.clear();
        assert_eq!(history.bytes, 0);
    }
}
