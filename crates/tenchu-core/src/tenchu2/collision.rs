use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::ModelId,
};
use serde::{Deserialize, Serialize};

const RETAIL_CAPACITY: usize = 64;
pub const REMOVED: u16 = 0x400;
pub const OBJECT: u16 = 0x1000;
pub const BODY: u16 = 0x2000;
pub const WEAPON: u16 = 0x4000;
pub const DISABLED: u16 = 0x8000;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Slot {
    pub model: ModelId,
    pub owner: Option<usize>,
    pub position: Position,
    pub offset: Vector3<i16>,
    pub size: Vector3<i16>,
    pub flags: u16,
    contacts: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collision {
    slots: Vec<Slot>,
    len: usize,
    expanded: bool,
}

impl Default for Collision {
    fn default() -> Self {
        Self {
            slots: vec![Slot::default(); RETAIL_CAPACITY],
            len: 0,
            expanded: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    pub index: usize,
    pub distance: Vector3<i16>,
}

impl Collision {
    pub fn validate_references(&self, models: usize, actors: usize) -> Result<()> {
        if self.len > self.slots.len()
            || self.slots.iter().any(|slot| {
                slot.model >= models
                    || slot.owner.is_some_and(|owner| owner >= actors)
                    // Source contacts can retain slots beyond the active prefix.
                    || slot.contacts.iter().any(|&id| id >= self.slots.len())
            })
        {
            return Err("Invalid Tenchu 2 collision references".into());
        }
        Ok(())
    }

    pub(super) fn allow_expansion(&mut self) {
        // Keep expanded storage until area reload: frozen contacts may still use it.
        self.expanded = true;
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots[..self.len]
    }

    pub fn slot(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    pub(crate) fn slot_mut(&mut self, index: usize) -> Option<&mut Slot> {
        self.slots.get_mut(index)
    }

    pub fn index(&self, model: ModelId) -> Option<usize> {
        self.slots()
            .iter()
            .position(|slot| slot.model == model && slot.flags & REMOVED == 0)
    }

    pub fn get_mut(&mut self, model: ModelId) -> Option<&mut Slot> {
        let index = self.index(model)?;
        Some(&mut self.slots[index])
    }

    pub fn insert(&mut self, model: ModelId) -> Result<&mut Slot> {
        if !self.expanded && self.len == RETAIL_CAPACITY {
            return Err("No free Tenchu 2 collision slot".into());
        }
        let index = if let Some(index) = self.index(model) {
            index
        } else {
            if self.len == self.slots.len() {
                self.slots
                    .resize_with(self.slots.len().max(RETAIL_CAPACITY) * 2, Slot::default);
            }
            let index = self.len;
            self.len += 1;
            self.slots[index] = Slot {
                model,
                ..Slot::default()
            };
            index
        };
        Ok(&mut self.slots[index])
    }

    pub fn remove(&mut self, model: ModelId) {
        if let Some(slot) = self.get_mut(model) {
            slot.flags |= REMOVED;
        }
    }

    pub fn compute(&mut self, mut world: impl FnMut(ModelId) -> Result<Matrix>) -> Result<()> {
        let mut index = 0;
        while index < self.len {
            if self.slots[index].flags & REMOVED != 0 {
                while index + 1 < self.len && self.slots[self.len - 1].flags & REMOVED != 0 {
                    self.len -= 1;
                }
                self.len -= 1;
                if index != self.len {
                    // Frozen contacts can still refer to the vacated tail slot.
                    self.slots[index] = self.slots[self.len].clone();
                }
            }
            index += 1;
        }
        let mut active = Vec::with_capacity(self.len);
        for (index, slot) in self.slots[..self.len].iter_mut().enumerate() {
            if slot.flags & DISABLED == 0 {
                slot.contacts.clear();
                slot.position = world(slot.model)?.transform(slot.offset.to_array());
                active.push(index);
            }
        }
        for (i, &first) in active.iter().enumerate() {
            for &second in &active[i + 1..] {
                let a = &self.slots[first];
                let b = &self.slots[second];
                if a.owner.is_some() && a.owner == b.owner {
                    continue;
                }
                if [1, 0, 2].into_iter().all(|axis| {
                    a.position[axis]
                        .wrapping_sub(b.position[axis])
                        .wrapping_abs()
                        <= i32::from(a.size[axis]) + i32::from(b.size[axis])
                }) {
                    self.slots[first].contacts.push(second);
                    self.slots[second].contacts.push(first);
                }
            }
        }
        Ok(())
    }

    pub fn contact(&self, model: ModelId, index: usize) -> Option<Contact> {
        let origin = &self.slots[self.index(model)?];
        if origin.contacts.is_empty() {
            return None;
        }
        let other = self.slots.get(index)?;
        Some(Contact {
            index,
            distance: Vector3::from_fn(|axis| {
                (other.position[axis] as i16).wrapping_sub(origin.position[axis] as i16)
            }),
        })
    }

    pub fn contacts(&self, model: ModelId) -> impl Iterator<Item = Contact> + '_ {
        self.index(model)
            .into_iter()
            .flat_map(move |index| self.slots[index].contacts.iter())
            .filter_map(move |&index| self.contact(model, index))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cover {
    excluded: Option<usize>,
    cached: Option<usize>,
}

impl Cover {
    pub fn new(excluded: Option<usize>) -> Self {
        Self {
            excluded,
            cached: None,
        }
    }

    pub fn check(&mut self, collision: &Collision, position: Position, tick: i32) -> Option<usize> {
        if tick & 63 == 0 {
            self.cached = collision
                .slots()
                .iter()
                .enumerate()
                .find_map(|(index, slot)| {
                    (Some(index) != self.excluded
                        && slot.flags & OBJECT != 0
                        && slot.flags & 0x8240 == 0
                        && slot
                            .position
                            .wrapping_sub(position)
                            .into_iter()
                            .all(|distance| distance.wrapping_abs() < 2000))
                    .then_some(index)
                });
        }
        self.cached
    }
}

#[cfg(test)]
mod tests;
