use super::{Kind, Mark, Vertex};
use std::{collections::VecDeque, sync::Arc};

pub(super) const VERTEX_LIMIT: usize = 4_194_304;
const PAGE_MARKS: usize = 128;
const MARK_VERTEX_LIMIT: usize = 3072;

struct Page {
    entries: VecDeque<(u64, usize, Kind)>,
    vertices: Arc<Vec<Vertex>>,
}

#[derive(Default)]
pub(super) struct Marks {
    pages: VecDeque<Page>,
    pub attached: VecDeque<(u64, Mark)>,
    pub vertices: usize,
    count: usize,
    serial: u64,
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
            self.attached.push_back((id, mark));
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
