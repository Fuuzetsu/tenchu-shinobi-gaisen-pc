use super::*;
use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 8] = b"TENCHUR2";
const EXPANDED_LIMIT: usize = 512 * 1024 * 1024;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Input {
    pub buttons: u16,
    pub bandage: bool,
    pub observing_death: bool,
    pub navigation: tenchu_core::game::navigation::Navigation,
    pub walking: Option<tenchu_core::game::walking::State>,
    pub sheathing: Option<tenchu_core::game::sheathing::State>,
    pub camera: Option<crate::debug::camera::Camera>,
    pub camera_actor: Option<crate::debug::camera::Subject>,
    pub light: Option<glam::Vec3>,
    pub light_follow: bool,
    pub native: Option<NativeInput>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NativeInput {
    pub navigation: Option<crate::navigation::NativeInput>,
    pub walking: Option<tenchu_core::tenchu2::world::controls::Walking>,
    pub sheathing: Option<tenchu_core::tenchu2::sheathing::State>,
    pub initial: bool,
}

#[derive(Serialize, Deserialize)]
enum Part {
    Shared {
        offset: usize,
        resource: usize,
    },
    Words {
        range: std::ops::Range<usize>,
        little_endian: bool,
    },
}

#[derive(Serialize, Deserialize)]
struct Frame {
    tick: u64,
    input: Option<Box<Input>>,
    length: usize,
    before: Option<Block>,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Recording {
    format: u32,
    executable: String,
    report: String,
    resources: Vec<Arc<Resource>>,
    frames: Vec<Frame>,
    tail: Vec<u8>,
}

impl History {
    pub fn export(&mut self, range: [u64; 2], report: String) -> Result<Vec<u8>> {
        self.export_for(range, report, tenchu_assets::game::EXE_SHA256)
    }

    pub fn export_for(
        &mut self,
        range: [u64; 2],
        report: String,
        executable: &str,
    ) -> Result<Vec<u8>> {
        let first = self.entries.iter().position(|e| e.tick == range[0]);
        let last = self.entries.iter().position(|e| e.tick == range[1]);
        let (first, last) = first
            .zip(last)
            .filter(|(a, b)| a < b)
            .ok_or("Select at least two recorded frames to export")?;
        let mut ids = HashMap::new();
        let mut resources = Vec::new();
        let frames = self
            .entries
            .range(first..=last)
            .enumerate()
            .map(|(i, entry)| Frame {
                tick: entry.tick,
                input: entry.input.clone(),
                length: entry.length,
                before: (i != 0).then(|| entry.before.clone()).flatten(),
                parts: entry
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        Segment::Shared { offset, bytes } => {
                            let resource =
                                *ids.entry(Arc::as_ptr(bytes) as usize).or_insert_with(|| {
                                    resources.push(bytes.clone());
                                    resources.len() - 1
                                });
                            Part::Shared {
                                offset: *offset,
                                resource,
                            }
                        }
                        Segment::Words {
                            range,
                            little_endian,
                        } => Part::Words {
                            range: range.clone(),
                            little_endian: *little_endian,
                        },
                    })
                    .collect(),
            })
            .collect();
        let recording = Recording {
            format: 2,
            executable: executable.into(),
            report,
            resources,
            frames,
            tail: self.snapshot(last)?.data.clone(),
        };
        let mut bytes = MAGIC.to_vec();
        bytes.extend(crate::savestate::encode(&recording)?);
        if bytes.len() > crate::savestate::FILE_LIMIT {
            return Err(
                "Recording exceeds the 128 MiB file limit; select a shorter interval".into(),
            );
        }
        Ok(bytes)
    }

    pub fn import(bytes: &[u8]) -> Result<(Self, String)> {
        Self::import_for(bytes, tenchu_assets::game::EXE_SHA256)
    }

    pub fn import_for(bytes: &[u8], executable: &str) -> Result<(Self, String)> {
        if bytes.get(..8) == Some(b"TENCHURP") {
            return Err("This recording uses the previous format; open it with the game version that exported it".into());
        }
        if bytes.len() > crate::savestate::FILE_LIMIT || bytes.get(..MAGIC.len()) != Some(MAGIC) {
            return Err("Not a Tenchu rewind recording".into());
        }
        let recording: Recording = crate::savestate::decode(&bytes[MAGIC.len()..])?;
        recording.restore(executable)
    }
}

impl Recording {
    fn restore(self, executable: &str) -> Result<(History, String)> {
        if self.format != 2 || self.executable != executable {
            return Err("Recording requires a different format or game disc".into());
        }
        if !(2..=901).contains(&self.frames.len())
            || self.frames[0].before.is_some()
            || self
                .frames
                .last()
                .unwrap()
                .tick
                .saturating_sub(self.frames[0].tick)
                > 900
            || self.tail.len() != self.frames.last().unwrap().length
            || self.report.len() > 4 * 1024 * 1024
            || self.resources.len() > 100_000
        {
            return Err("Invalid recording interval or metadata".into());
        }
        let mut expanded = 0usize;
        let mut stored = self.tail.len();
        let mut scratch = Vec::new();
        for resource in &self.resources {
            expanded = expanded.saturating_add(resource.decoded_length);
            stored = stored.saturating_add(resource.size());
            if expanded > EXPANDED_LIMIT || stored > MEMORY_LIMIT {
                return Err("Recording resources exceed the memory limit".into());
            }
            scratch.clear();
            resource.append(&mut scratch)?;
        }
        let mut history = History::default();
        for frame in self.frames {
            if frame
                .input
                .as_ref()
                .and_then(|i| i.native.as_ref())
                .and_then(|i| i.navigation)
                .is_some_and(|n| {
                    !n.yaw.is_finite()
                        || n.sticks.iter().any(|s| !s.is_finite())
                        || !(5..=40).contains(&n.deadzone)
                })
            {
                return Err("Invalid recording native navigation".into());
            }
            stored = stored.saturating_add(
                frame
                    .parts
                    .len()
                    .saturating_mul(std::mem::size_of::<Segment>()),
            );
            if frame.length > EXPANDED_LIMIT || frame.parts.len() > 100_000 {
                return Err("Recording frame exceeds the memory limit".into());
            }
            if frame
                .input
                .as_ref()
                .and_then(|i| i.camera)
                .is_some_and(|c| {
                    !c.position.is_finite()
                        || !c.yaw.is_finite()
                        || !c.pitch.is_finite()
                        || !(1.0..179.0).contains(&c.fov)
                })
            {
                return Err("Invalid recording camera".into());
            }
            if frame
                .input
                .as_ref()
                .and_then(|i| i.light)
                .is_some_and(|p| !p.is_finite())
            {
                return Err("Invalid recording light".into());
            }
            if let Some(previous) = history.entries.back() {
                let block = frame
                    .before
                    .as_ref()
                    .ok_or("Missing recording frame delta")?;
                stored = stored.saturating_add(block.bytes.len());
                if frame.tick <= previous.tick
                    || block.encoded_length != frame.length.max(previous.length)
                {
                    return Err("Invalid recording frame order or delta length".into());
                }
                if stored > MEMORY_LIMIT {
                    return Err("Recording exceeds the memory limit".into());
                }
                scratch.resize(block.encoded_length, 0);
                if lz4_flex::decompress_into(&block.bytes, &mut scratch)
                    .map_err(|e| e.to_string())?
                    != block.encoded_length
                {
                    return Err("Incomplete recording frame".into());
                }
            }
            let mut offset = 0;
            let mut expanded = frame.length;
            let mut segments = Vec::new();
            for part in frame.parts {
                match part {
                    Part::Shared {
                        offset: start,
                        resource,
                    } => {
                        if start < offset || start > frame.length {
                            return Err("Invalid recording resource position".into());
                        }
                        let bytes = self
                            .resources
                            .get(resource)
                            .ok_or("Missing recording resource")?
                            .clone();
                        expanded = expanded.saturating_add(bytes.decoded_length);
                        offset = start;
                        let count = history
                            .shared
                            .entry(Arc::as_ptr(&bytes) as usize)
                            .or_insert_with(|| {
                                history.bytes += bytes.size();
                                (0, bytes.size())
                            });
                        count.0 += 1;
                        segments.push(Segment::Shared { offset, bytes });
                    }
                    Part::Words {
                        range,
                        little_endian,
                    } => {
                        if range.start < offset
                            || range.end < range.start
                            || range.end > frame.length
                            || !range.len().is_multiple_of(2)
                        {
                            return Err("Invalid recording texture range".into());
                        }
                        expanded = expanded.saturating_add(range.len() / 2);
                        offset = range.end;
                        segments.push(Segment::Words {
                            range,
                            little_endian,
                        });
                    }
                }
            }
            if expanded > EXPANDED_LIMIT || stored > MEMORY_LIMIT {
                return Err("Recording frame exceeds the memory limit".into());
            }
            let entry = Entry {
                tick: frame.tick,
                input: frame.input,
                length: frame.length,
                before: frame.before,
                segments,
            };
            history.bytes += entry.bytes();
            history.entries.push_back(entry);
        }
        history.tail.data = self.tail;
        Ok((history, self.report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::checkpoint::Capture;

    fn fixture() -> History {
        let mut history = History::default();
        let a = Arc::new(vec![17u8; 10000]);
        let b = Arc::new(vec![91u8; 800]);
        for tick in 0..9u64 {
            history.recorder.begin();
            history.recorder.value(&tick).unwrap();
            history
                .recorder
                .words(&[0, 32767, tick as u16, 65535])
                .unwrap();
            history
                .recorder
                .shared_arc(if tick % 2 == 0 { &a } else { &b }, true)
                .unwrap();
            history.recorder.shared_arc(&a, true).unwrap();
            history
                .recorder
                .value(&vec![tick as u8; tick as usize * 7])
                .unwrap();
            let mut state = history.recorder.finish().unwrap();
            // Mixed-endian word blocks exercise imports across native and browser targets.
            if tick % 2 == 0 {
                for segment in &mut state.segments {
                    if let Segment::Words {
                        range,
                        little_endian,
                    } = segment
                    {
                        for word in state.data[range.clone()].chunks_exact_mut(2) {
                            word.swap(0, 1);
                        }
                        *little_endian = !*little_endian;
                    }
                }
            }
            history.push(tick, 30, state, None).unwrap();
        }
        history
    }

    #[test]
    fn selected_recording_preserves_shared_assets_words_deltas_and_branching() {
        let mut history = fixture();
        let bytes = history.export([2, 7], "report".into()).unwrap();
        let (mut restored, report) = History::import(&bytes).unwrap();
        assert_eq!(report, "report");
        let shared: std::collections::HashSet<_> = history
            .entries
            .range(2..=7)
            .flat_map(|e| e.segments.iter().filter_map(Segment::resource))
            .map(Arc::as_ptr)
            .collect();
        assert_eq!(restored.shared.len(), shared.len());
        assert!(
            restored.shared.len() < 12,
            "repeated resource references were duplicated"
        );
        assert_eq!(restored.entries.len(), 6);
        assert_eq!(
            history.entries.len(),
            9,
            "export discarded the original future"
        );
        for index in [5, 0, 4, 1, 5] {
            let expected = history.snapshot(index + 2).unwrap().bytes().unwrap();
            assert_eq!(restored.snapshot(index).unwrap().bytes().unwrap(), expected);
            let (tick, words, _, _, tail): (u64, Vec<u16>, Vec<u8>, Vec<u8>, Vec<u8>) =
                restored.snapshot(index).unwrap().decode().unwrap();
            assert_eq!(words, [0, 32767, tick as u16, 65535]);
            assert_eq!(tail, vec![tick as u8; tick as usize * 7]);
        }
        restored.branch(2).unwrap();
        restored
            .push(5, 30, Snapshot::encode(&123u16).unwrap(), None)
            .unwrap();
        assert_eq!(restored.snapshot(3).unwrap().decode::<u16>().unwrap(), 123);
        assert!(history.export([4, 4], String::new()).is_err());
        assert!(history.export([7, 2], String::new()).is_err());
    }

    #[test]
    fn malformed_recordings_reject_invalid_ranges_references_and_lengths() {
        let bytes = fixture().export([0, 8], String::new()).unwrap();
        for length in [0, 7, 20, bytes.len() - 1] {
            assert!(History::import(&bytes[..length]).is_err());
        }
        let mutate = |change: fn(&mut Recording)| {
            let mut recording: Recording = crate::savestate::decode(&bytes[8..]).unwrap();
            change(&mut recording);
            assert!(recording.restore(tenchu_assets::game::EXE_SHA256).is_err());
        };
        mutate(|r| r.frames[1].tick = r.frames[0].tick);
        mutate(|r| r.frames[1].before = None);
        mutate(|r| r.frames[1].before.as_mut().unwrap().encoded_length += 1);
        mutate(|r| {
            r.frames[1].parts.push(Part::Shared {
                offset: 0,
                resource: usize::MAX,
            })
        });
        mutate(|r| {
            r.frames[0].parts = vec![Part::Words {
                range: 0..usize::MAX,
                little_endian: true,
            }]
        });
        mutate(|r| r.frames[0].length = EXPANDED_LIMIT + 1);
        mutate(|r| r.tail.pop().map(|_| ()).unwrap());
        mutate(|r| r.executable = "wrong disc".into());
        mutate(|r| {
            r.frames[0].input = Some(Box::new(Input {
                native: Some(NativeInput {
                    initial: false,
                    walking: None,
                    sheathing: None,
                    navigation: Some(crate::navigation::NativeInput {
                        yaw: f32::NAN,
                        pitch: 0,
                        sticks: [glam::Vec2::ZERO; 2],
                        deadzone: 18,
                        align_aim: true,
                    }),
                }),
                ..Default::default()
            }))
        });
        assert!(
            History::import(b"TENCHURP")
                .err()
                .unwrap()
                .contains("previous format")
        );
    }
}
