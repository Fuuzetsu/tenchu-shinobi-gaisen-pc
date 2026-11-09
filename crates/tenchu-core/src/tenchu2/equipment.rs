use super::{input, progress::Stock};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::equipment::Rules;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub item: usize,
    pub count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Confirm,
    Cancel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Equipment {
    pub stock: Stock,
    pub slots: Vec<Slot>,
    pub selected: usize,
    pub description: usize,
    pub remaining: u8,
    pub fade: i16,
    pub outcome: Option<Outcome>,
    pub finished: bool,
    pub tick: u32,
    pub cursors: [usize; 3],
    previous: u16,
    history: [u16; 26],
}

pub fn training(map: u16) -> Option<[u8; 21]> {
    if map > 1 {
        return None;
    }
    let mut inventory = [0; 21];
    inventory[0] = 1;
    if map == 1 {
        inventory[1] = 1;
        inventory[4] = 5;
    }
    Some(inventory)
}

impl Equipment {
    pub fn new(stock: Stock, double_capacity: bool) -> Self {
        Self {
            stock,
            slots: vec![Slot { item: 0, count: 1 }],
            selected: 0,
            description: 3,
            remaining: if double_capacity { 24 } else { 12 },
            fade: 240,
            outcome: None,
            finished: false,
            tick: 0,
            cursors: [0; 3],
            previous: 0,
            history: [0; 26],
        }
    }

    pub fn inventory(&self) -> [u8; 21] {
        let mut inventory = [0; 21];
        if self.outcome == Some(Outcome::Confirm) {
            for slot in &self.slots {
                if slot.item != 19 {
                    inventory[slot.item] = slot.count;
                }
            }
        }
        inventory
    }

    pub fn alternate(&self) -> bool {
        self.outcome == Some(Outcome::Confirm) && self.slots.iter().any(|s| s.item == 19)
    }

    pub fn step(&mut self, mut buttons: u16, rules: &Rules) -> Vec<u8> {
        if self.finished {
            return Vec::new();
        }
        let mut sounds = Vec::new();
        if buttons != self.previous {
            self.history.copy_within(..25, 1);
            self.history[0] = buttons;
            if self.history.starts_with(&rules.unlock) {
                sounds.push(2);
                for item in &mut self.stock.items {
                    item.get_or_insert(0);
                }
            }
            if self.history.starts_with(&rules.replenish) {
                sounds.push(2);
                for count in self.stock.items.iter_mut().flatten() {
                    if *count < 99 {
                        *count += 1;
                    }
                }
            }
        }
        if self.fade == 0 {
            if buttons == 9 {
                self.outcome = Some(Outcome::Cancel);
                self.fade = -8;
            } else {
                if self.previous == 0 {
                    let direction = match buttons {
                        input::UP => Some(0),
                        input::DOWN => Some(1),
                        input::LEFT => Some(2),
                        input::RIGHT => Some(3),
                        _ => None,
                    };
                    if let Some(next) =
                        direction.and_then(|d| rules.cells[self.selected].neighbours[d])
                        && next != self.selected
                    {
                        self.selected = next;
                        self.description = next + 3;
                        sounds.push(5);
                    }
                    if buttons == 8 {
                        sounds.push(2);
                        self.outcome = Some(Outcome::Confirm);
                        self.fade = -8;
                        buttons = 0;
                    }
                }
                let item = rules.cells[self.selected].item;
                if buttons & !self.previous & 0x2000 != 0 {
                    let sound = self.add(item);
                    sounds.push(sound);
                    if sound == 7 && self.description < 3 {
                        sounds.push(6);
                    }
                } else if buttons & !self.previous & 0x4000 != 0 {
                    self.remove(item);
                    sounds.push(6);
                    self.description = self.selected + 3;
                }
            }
        }
        self.cursors.copy_within(..2, 1);
        self.cursors[0] = self.selected;
        if self.fade > 0 {
            self.fade -= 8;
        } else if self.fade < 0 {
            self.fade -= 8;
            if self.fade <= -240 {
                self.finished = true;
                if self.outcome == Some(Outcome::Cancel) {
                    for slot in &self.slots {
                        let count = self.stock.items[slot.item]
                            .unwrap_or(255)
                            .wrapping_add(slot.count);
                        self.stock.items[slot.item] = (count != 255).then_some(count);
                    }
                }
            }
        }
        self.previous = buttons;
        self.tick = self.tick.wrapping_add(1);
        sounds
    }

    fn add(&mut self, item: usize) -> u8 {
        if self.stock.items[item].is_none_or(|count| count == 0) {
            self.description = self.selected + 3;
            return 6;
        }
        if self.remaining == 0 {
            self.description = 2;
            return 6;
        }
        let count = if let Some(slot) = self.slots.iter_mut().find(|s| s.item == item) {
            let limit = match item {
                8 | 12 => 5,
                9 | 13 | 18 => 3,
                16 => 2,
                17 | 19 | 20 => 1,
                _ => 255,
            };
            if slot.count == limit {
                self.description = 0;
                return 6;
            }
            &mut slot.count
        } else {
            if self.slots.len() == 6 {
                self.description = 1;
                return 6;
            }
            self.slots.push(Slot { item, count: 0 });
            &mut self.slots.last_mut().unwrap().count
        };
        *count += 1;
        *self.stock.items[item].as_mut().unwrap() -= 1;
        self.remaining -= 1;
        // A successful add leaves any previous capacity message selected.
        7
    }

    fn remove(&mut self, item: usize) {
        if let Some(index) = self.slots.iter().position(|s| s.item == item) {
            let count = self.stock.items[item].unwrap_or(255).wrapping_add(1);
            self.stock.items[item] = (count != 255).then_some(count);
            self.remaining += 1;
            self.slots[index].count -= 1;
            if self.slots[index].count == 0 {
                self.slots.remove(index);
            }
        }
    }
}
