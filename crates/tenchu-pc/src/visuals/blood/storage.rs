use super::{Kind, Mark, Vertex};
use std::{collections::VecDeque, sync::Arc};

pub(super) const VERTEX_LIMIT: usize = 4_194_304;
const PAGE_MARKS: usize = 128;
const MARK_VERTEX_LIMIT: usize = 3072;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Page {
    entries: VecDeque<(u64, usize, Kind)>,
    vertices: Arc<Vec<Vertex>>,
}

#[derive(Default, serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct Marks {
    pages: VecDeque<Page>,
    pub attached: VecDeque<(u64, Arc<Mark>)>,
    pub vertices: usize,
    count: usize,
    serial: u64,
}

impl Marks {
    pub(super) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            pages,
            attached,
            vertices,
            count,
            serial,
        } = self;
        out.value(&pages.len())?;
        for Page { entries, vertices } in pages {
            out.value(entries)?;
            out.shared_arc(vertices, entries.len() == PAGE_MARKS)?;
        }
        out.value(&attached.len())?;
        for (id, mark) in attached {
            out.value(id)?;
            out.shared_arc(mark, true)?;
        }
        out.value(vertices)?;
        out.value(count)?;
        out.value(serial)
    }
}

impl Marks {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.count
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn pages(&self) -> impl Iterator<Item = &Arc<Vec<Vertex>>> {
        self.pages.iter().map(|page| &page.vertices)
    }

    pub fn insert(&mut self, mark: Mark, limit: usize) {
        let len = mark.len();
        if len == 0 || len > MARK_VERTEX_LIMIT {
            return;
        }
        let id = self.serial;
        self.serial += 1;
        self.count += 1;
        self.vertices += len;
        if mark.0.iter().all(|p| p.part.is_none()) {
            if self
                .pages
                .back()
                .is_none_or(|p| p.entries.len() >= PAGE_MARKS)
            {
                self.pages.push_back(Page {
                    entries: VecDeque::new(),
                    vertices: Arc::default(),
                });
            }
            let page = self.pages.back_mut().unwrap();
            page.entries.push_back((id, len, mark.1));
            Arc::make_mut(&mut page.vertices).extend(mark.0.into_iter().flat_map(|p| p.vertices));
        } else {
            self.attached.push_back((id, Arc::new(mark)));
        }
        self.trim(limit);
    }

    pub fn trim(&mut self, limit: usize) {
        while self.count > limit || self.vertices > VERTEX_LIMIT {
            let world = self
                .pages
                .front()
                .and_then(|p| p.entries.front())
                .map(|e| e.0);
            let attached = self.attached.front().map(|e| e.0);
            let len = if world.is_some_and(|id| attached.is_none_or(|other| id < other)) {
                let page = self.pages.front_mut().unwrap();
                let (_, len, _) = page.entries.pop_front().unwrap();
                if page.entries.is_empty() {
                    self.pages.pop_front();
                } else {
                    Arc::make_mut(&mut page.vertices).drain(..len);
                }
                len
            } else {
                self.attached.pop_front().unwrap().1.len()
            };
            self.count -= 1;
            self.vertices -= len;
        }
    }

    pub fn retain_attached(&mut self, mut keep: impl FnMut(&Mark) -> bool) {
        self.attached.retain(|(_, mark)| {
            if keep(mark) {
                true
            } else {
                self.count -= 1;
                self.vertices -= mark.len();
                false
            }
        });
    }

    pub fn retain_kind(&mut self, keep: impl Fn(Kind) -> bool) {
        self.retain_attached(|mark| keep(mark.1));
        self.pages.retain_mut(|page| {
            if page.entries.iter().all(|e| keep(e.2)) {
                return true;
            }
            let mut vertices = Vec::new();
            let mut offset = 0;
            page.entries.retain(|&(_, len, kind)| {
                let retained = keep(kind);
                if retained {
                    vertices.extend_from_slice(&page.vertices[offset..offset + len]);
                } else {
                    self.count -= 1;
                    self.vertices -= len;
                }
                offset += len;
                retained
            });
            page.vertices = Arc::new(vertices);
            !page.entries.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visuals::blood::Patch;
    use bytemuck::Zeroable;
    use std::time::Instant;

    #[test]
    fn rewind_restores_stains_after_expiry_and_style_changes() {
        use crate::{rewind::History, visuals::interpolation::Part};

        let mut marks = Marks::default();
        let mut history = History::default();
        let mut expected = Vec::new();
        for tick in 0..20 {
            let attached = tick % 2 == 0;
            marks.insert(
                Mark(
                    vec![Patch {
                        part: attached.then_some(Part::Actor {
                            root: 0,
                            model: 1,
                            kind: 0,
                            inspection: false,
                        }),
                        vertices: vec![
                            Vertex {
                                position: [tick as f32, 0.0, 0.0],
                                ..Vertex::zeroed()
                            };
                            6
                        ],
                        bounds: Default::default(),
                    }],
                    if attached { Kind::Body } else { Kind::Ground },
                ),
                8,
            );
            if tick == 12 {
                marks.retain_kind(|kind| kind == Kind::Body);
            } else if tick == 16 {
                marks.retain_attached(|_| false);
            }
            expected.push(postcard::to_allocvec(&marks).unwrap());
            history.recorder.begin();
            marks.capture(&mut history.recorder).unwrap();
            let snapshot = history.recorder.finish().unwrap();
            history.push(tick, 30, snapshot).unwrap();
        }
        for (tick, expected) in expected.into_iter().enumerate().rev() {
            let snapshot = history.snapshot(tick).unwrap();
            assert_eq!(snapshot.bytes().unwrap(), expected);
            let restored: Marks = snapshot.decode().unwrap();
            assert_eq!(postcard::to_allocvec(&restored).unwrap(), expected);
        }
    }

    #[test]
    #[ignore = "CPU benchmark for retaining and rewinding thousands of persistent blood splats"]
    fn profile_rewind_with_accumulated_blood() {
        let mut marks = Marks::default();
        for i in 0..4096 {
            let vertices = (0..24)
                .map(|j| Vertex {
                    position: [i as f32, 0.0, j as f32],
                    color: [0.5, 0.0, 0.0],
                    ..Vertex::zeroed()
                })
                .collect();
            marks.insert(
                Mark(
                    vec![Patch {
                        part: None,
                        vertices,
                        bounds: Default::default(),
                    }],
                    Kind::Ground,
                ),
                16384,
            );
        }
        let mut history = crate::rewind::History::default();
        let mut full = std::time::Duration::ZERO;
        let mut cached = std::time::Duration::ZERO;
        for tick in 0..120 {
            let started = Instant::now();
            let expected = postcard::to_allocvec(&marks).unwrap();
            full += started.elapsed();
            let started = Instant::now();
            history.recorder.begin();
            marks.capture(&mut history.recorder).unwrap();
            let state = history.recorder.finish().unwrap();
            history.push(tick, 30, state).unwrap();
            cached += started.elapsed();
            if tick % 30 == 0 {
                assert_eq!(
                    history
                        .snapshot(history.entries.len() - 1)
                        .unwrap()
                        .bytes()
                        .unwrap(),
                    expected
                );
            }
        }
        eprintln!(
            "4096 splats ({} vertices): full encode {:.3} ms/update; cached encode+history {:.3} ms/update; {} KiB history",
            marks.vertices,
            full.as_secs_f64() * 1000.0 / 120.0,
            cached.as_secs_f64() * 1000.0 / 120.0,
            history.bytes / 1024
        );
        for index in (0..history.entries.len()).rev() {
            let restored: Marks = history.snapshot(index).unwrap().decode().unwrap();
            assert_eq!(restored.vertices, marks.vertices);
        }
    }
}
