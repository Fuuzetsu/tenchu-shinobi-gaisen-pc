use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Item(u8);

impl Item {
    pub const GRAPPLE: Self = Self(0);

    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl TryFrom<u8> for Item {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= 20 {
            Ok(Self(value))
        } else {
            Err("Invalid Tenchu 2 inventory item")
        }
    }
}

impl From<Item> for u8 {
    fn from(value: Item) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub counts: [u8; 20],
    pub selected: Option<Item>,
    pub active: Item,
    pub resurrections: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            counts: [0; 20],
            selected: Some(Item::GRAPPLE),
            active: Item::GRAPPLE,
            resurrections: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    Unselected,
    Empty,
}

impl Inventory {
    pub fn from_amounts(amounts: [u8; 21]) -> Self {
        Self {
            counts: std::array::from_fn(|i| amounts[i]),
            resurrections: amounts[20],
            ..Self::default()
        }
    }

    pub fn amounts(&self) -> [u8; 21] {
        std::array::from_fn(|i| {
            if i == 20 {
                self.resurrections
            } else {
                self.counts[i]
            }
        })
    }

    pub fn for_actor(
        kind: i16,
        loot: &tenchu_assets::tenchu2::loot::Loot,
        random: &mut crate::random::Random,
    ) -> Self {
        let mut inventory = Self::default();
        if matches!(kind & 0xf0, 0x20 | 0x30) {
            inventory.counts[15] = (random.next15() % 3) as u8;
        }
        if let Some((_, amounts)) = loot.actors.iter().find(|(id, _)| *id == kind) {
            for (&item, &amount) in loot.items.iter().zip(amounts) {
                if amount != 0 {
                    inventory.counts[item] = if amount & 0x80 != 0 {
                        amount.wrapping_neg()
                    } else {
                        (random.next15() % (i32::from(amount) * 2 + 1)) as u8
                    };
                }
            }
        }
        inventory
    }

    pub fn count(&self, item: Item) -> u8 {
        if item.index() == 20 {
            self.resurrections
        } else {
            self.counts[item.index()]
        }
    }

    fn count_mut(&mut self, item: Item) -> &mut u8 {
        if item.index() == 20 {
            &mut self.resurrections
        } else {
            &mut self.counts[item.index()]
        }
    }

    pub fn select(&mut self, previous: bool) {
        let Some(selected) = self.selected else {
            return;
        };
        self.selected = (1..=21).find_map(|step| {
            let index = (selected.index() + if previous { 21 - step } else { step }) % 21;
            let item = Item(index as u8);
            (self.count(item) != 0).then_some(item)
        });
    }

    pub fn take(&mut self) -> Result<Item, Unavailable> {
        let item = self.selected.ok_or(Unavailable::Unselected)?;
        let count = self.count_mut(item);
        if *count == 0 {
            return Err(Unavailable::Empty);
        }
        if item != Item::GRAPPLE {
            *count -= 1;
        }
        self.active = item;
        Ok(item)
    }

    pub fn refund(&mut self) {
        let count = self.count_mut(self.active);
        *count = count.wrapping_add(1);
    }
}
