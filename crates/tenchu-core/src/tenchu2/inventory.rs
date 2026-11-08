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
        if value < 20 {
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
    pub fn take(&mut self) -> Result<Item, Unavailable> {
        let item = self.selected.ok_or(Unavailable::Unselected)?;
        let count = &mut self.counts[item.index()];
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
        let count = &mut self.counts[self.active.index()];
        *count = count.wrapping_add(1);
    }
}
