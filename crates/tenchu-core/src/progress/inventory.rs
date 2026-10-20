use super::{INFINITE, LOCKED};
use tenchu_assets::shop::{ITEMS, Shop};

pub struct Loadout {
    pub stock: [u8; ITEMS],
    pub selected: [u8; ITEMS],
    pub taken: i16,
    pub kinds: i16,
    pub capacity: i16,
    training: bool,
}

impl Loadout {
    pub const KIND_LIMIT: i16 = 5;

    pub fn new(mut stock: [u8; ITEMS], shop: &Shop, training: bool) -> Self {
        let mut selected = [0; ITEMS];
        selected[0] = INFINITE;
        if training {
            selected[1] = 5;
        } else {
            for (count, &limit) in stock.iter_mut().zip(&shop.limits) {
                if *count != LOCKED && *count > limit {
                    *count = limit;
                }
            }
        }
        Self {
            stock,
            selected,
            taken: 0,
            kinds: 1,
            capacity: 15,
            training,
        }
    }

    pub fn take(&mut self, item: usize) -> bool {
        if self.training
            || item == 0
            || item >= ITEMS
            || self.stock[item] == 0
            || self.stock[item] == LOCKED
            || self.taken >= self.capacity
        {
            return false;
        }
        let count = self.selected[item];
        if count == 0 {
            self.kinds += 1;
        }
        if self.kinds > Self::KIND_LIMIT {
            self.kinds -= 1;
            return false;
        }
        if item == 19 && count != 0 {
            return false;
        }
        self.selected[item] = count.wrapping_add(1);
        self.stock[item] = self.stock[item].wrapping_sub(1);
        self.taken += 1;
        true
    }

    pub fn put_back(&mut self, item: usize) -> bool {
        if self.training || item == 0 || item >= ITEMS || self.selected[item] == 0 {
            return false;
        }
        if self.selected[item] == INFINITE {
            self.selected[item] = 0;
            self.stock[item] = 1;
            self.kinds -= 1;
        } else {
            self.selected[item] -= 1;
            self.stock[item] = self.stock[item].wrapping_add(1);
            if self.selected[item] == 0 {
                self.kinds -= 1;
            }
        }
        self.taken -= 1;
        true
    }

    pub fn finish(mut self) -> ([u8; ITEMS], [u8; ITEMS]) {
        if !self.training {
            if self.selected[18] != 0 {
                self.selected[18] = INFINITE;
            }
            for item in 0..9 {
                if self.stock[item] == 0 {
                    self.stock[item] = LOCKED;
                }
            }
        }
        (self.stock, self.selected)
    }
}
