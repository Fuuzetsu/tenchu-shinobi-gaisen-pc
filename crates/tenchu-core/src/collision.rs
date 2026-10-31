use crate::math::Position;
use crate::{
    Result,
    scene::{COLLIDE, CONFLICT, ModelId, Scene},
};

pub const RETAIL_CAPACITY: usize = 80;
pub const HIT: i16 = 1;
pub const STAND: i16 = 4;
pub const SOFT: i16 = 8;
const LIVE: u8 = 0x80;
const CONSUMED: u8 = 0x40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Actor(usize),
    Door,
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub model: Option<ModelId>,
    pub owner: Option<Owner>,
    pub position: Position,
    pub offset: [i16; 3],
    pub size: [i16; 3],
    pub class: i16,
    pub count: i32,
    pub results: Vec<u8>,
}

impl Slot {
    pub fn actor(&self) -> Option<usize> {
        match self.owner {
            Some(Owner::Actor(id)) => Some(id),
            Some(Owner::Door) | None => None,
        }
    }
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            model: None,
            owner: None,
            position: Position::ZERO,
            offset: [0; 3],
            size: [0; 3],
            class: 0,
            count: 0,
            results: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Collision {
    pub slots: Vec<Slot>,
    pub len: usize,
    pub distance: [i16; 3],
    pub other: Option<ModelId>,
}

impl Default for Collision {
    fn default() -> Self {
        Self {
            slots: vec![
                Slot {
                    results: vec![0; RETAIL_CAPACITY],
                    ..Slot::default()
                };
                RETAIL_CAPACITY
            ],
            len: 0,
            distance: [0; 3],
            other: None,
        }
    }
}

impl Collision {
    pub fn insert(&mut self, scene: &mut Scene, model: ModelId) -> Result<usize> {
        if let Some(id) = scene.models[model].conflict {
            return Ok(id);
        }
        if self.len == self.slots.len() {
            let capacity = self.slots.len() * 2;
            self.slots.resize_with(capacity, Slot::default);
            for slot in &mut self.slots {
                slot.results.resize(capacity, 0);
            }
        }
        let id = self.len;
        self.len += 1;
        self.slots[id] = Slot {
            model: Some(model),
            results: std::mem::take(&mut self.slots[id].results),
            ..Slot::default()
        };
        self.slots[id].results.fill(0);
        scene.models[model].conflict = Some(id);
        scene.models[model].attribute = (scene.models[model].attribute | COLLIDE) & !CONFLICT;
        Ok(id)
    }

    pub fn remove(&mut self, scene: &mut Scene, model: ModelId) {
        if scene.models[model].conflict.is_none() {
            return;
        }
        let mut i = 0;
        while i < self.len {
            if self.slots[i].model == Some(model) {
                self.len -= 1;
                self.slots[i] = self.slots[self.len].clone();
                scene.models[self.slots[i].model.unwrap()].conflict = Some(i);
            } else {
                i += 1;
            }
        }
        scene.models[model].conflict = None;
        scene.models[model].attribute &= !(COLLIDE | CONFLICT);
    }

    pub fn compute(&mut self, scene: &mut Scene) -> Result<()> {
        for slot in &mut self.slots[..self.len] {
            let id = slot.model.ok_or("Unbound conflict slot")?;
            let model = &mut scene.models[id];
            if model.attribute & COLLIDE == 0 {
                continue;
            }
            slot.results.fill(0);
            slot.count = 0;
            model.attribute &= !CONFLICT;
            slot.position = if model.parent == Some(0) {
                Position::from_fn(|i| {
                    model.local.translation[i].wrapping_add(i32::from(slot.offset[i]))
                })
            } else {
                scene.world(id)?.transform(slot.offset)
            };
        }
        for i in 0..self.len {
            let a = self.slots[i].model.unwrap();
            if scene.models[a].attribute & COLLIDE == 0 {
                continue;
            }
            for j in i + 1..self.len {
                let b = self.slots[j].model.unwrap();
                if scene.models[b].attribute & COLLIDE == 0 {
                    continue;
                }
                let touches = [1, 2, 0].into_iter().all(|axis| {
                    self.slots[j].position[axis]
                        .wrapping_sub(self.slots[i].position[axis])
                        .wrapping_abs()
                        <= i32::from(self.slots[i].size[axis]) + i32::from(self.slots[j].size[axis])
                });
                if touches {
                    self.slots[i].results[j] = self.slots[j].class as u8 | LIVE;
                    self.slots[j].results[i] = self.slots[i].class as u8 | LIVE;
                    self.slots[i].count += 1;
                    self.slots[j].count += 1;
                    scene.models[a].attribute |= CONFLICT;
                    scene.models[b].attribute |= CONFLICT;
                }
            }
        }
        Ok(())
    }

    pub fn result(&mut self, scene: &Scene, model: ModelId, index: Option<usize>) -> Option<usize> {
        let id = scene.models[model].conflict?;
        if scene.models[model].attribute & COLLIDE == 0 {
            return None;
        }
        let index = match index {
            Some(index) => index,
            None => {
                let mut count = 0;
                let mut index = 0;
                while index < self.len {
                    let result = self.slots[id].results[index];
                    if result != 0 {
                        count += 1;
                        if count > self.slots[id].count {
                            return None;
                        }
                        if result & CONSUMED == 0 {
                            break;
                        }
                    }
                    index += 1;
                }
                index
            }
        };
        if index >= self.len || self.slots[id].results[index] == 0 {
            return None;
        }
        self.slots[id].results[index] |= CONSUMED;
        self.other = self.slots[index].model;
        self.distance = std::array::from_fn(|i| {
            (self.slots[index].position[i] as i16).wrapping_sub(self.slots[id].position[i] as i16)
        });
        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Matrix;

    #[test]
    fn growing_collision_storage_preserves_ordered_contacts_and_reuses_removed_slots() {
        let mut scene = Scene::default();
        let mut pool = Collision::default();
        let mut models = Vec::new();
        for _ in 0..RETAIL_CAPACITY * 3 {
            let model = scene.add(Some(0), Matrix::default());
            pool.insert(&mut scene, model).unwrap();
            models.push(model);
        }
        pool.compute(&mut scene).unwrap();
        for index in 1..models.len() {
            assert_eq!(pool.result(&scene, models[0], None), Some(index));
        }
        assert_eq!(pool.result(&scene, models[0], None), None);
        pool.remove(&mut scene, models[1]);
        assert_eq!(scene.models[*models.last().unwrap()].conflict, Some(1));
        let slot = pool.insert(&mut scene, models[1]).unwrap();
        assert!(pool.slots[slot].results.iter().all(|&r| r == 0));
        pool.compute(&mut scene).unwrap();
        assert_eq!(pool.result(&scene, models[0], None), Some(1));
        assert_eq!(pool.slots[slot].count, (models.len() - 1) as i32);
    }

    #[test]
    fn ordered_contacts_keep_inactive_results_and_vacated_tail_bytes() {
        let mut scene = Scene::default();
        let mut pool = Collision::default();
        let models: Vec<_> = [0, 20, 40, 60]
            .into_iter()
            .map(|x| {
                let id = scene.add(
                    Some(0),
                    Matrix {
                        translation: Position::new(x, 0, 0),
                        ..Matrix::default()
                    },
                );
                let slot = pool.insert(&mut scene, id).unwrap();
                pool.slots[slot].size = [10; 3];
                id
            })
            .collect();
        pool.slots[1].class = HIT;
        scene.models[models[3]].attribute = CONFLICT;
        pool.slots[3].results[0] = 0xc1;
        pool.slots[3].count = 1;
        pool.compute(&mut scene).unwrap();
        assert_eq!(
            pool.slots[..4].iter().map(|s| s.count).collect::<Vec<_>>(),
            [1, 2, 1, 1]
        );
        assert_eq!(pool.slots[3].results[0], 0xc1);
        assert_eq!(pool.result(&scene, models[1], None), Some(0));
        assert_eq!(pool.distance, [-20, 0, 0]);
        assert_eq!(pool.result(&scene, models[1], None), Some(2));
        assert_eq!(pool.result(&scene, models[1], None), None);
        assert_eq!(pool.result(&scene, models[1], Some(0)), Some(0));
        pool.remove(&mut scene, models[1]);
        assert_eq!(pool.len, 3);
        assert_eq!(scene.models[models[3]].conflict, Some(1));
        assert_eq!(pool.slots[1].results[0], 0xc1);
        assert_eq!(pool.slots[3].results[0], 0xc1);
        assert_eq!(pool.slots[3].model, Some(models[3]));
    }

    #[test]
    fn min_signed_separation_still_overlaps_in_retail_arithmetic() {
        let mut scene = Scene::default();
        let mut pool = Collision::default();
        for x in [0, i32::MIN] {
            let model = scene.add(
                Some(0),
                Matrix {
                    translation: Position::new(x, 0, 0),
                    ..Matrix::default()
                },
            );
            pool.insert(&mut scene, model).unwrap();
        }
        pool.compute(&mut scene).unwrap();
        assert_eq!(pool.slots[0].results[1], LIVE);
    }
}
