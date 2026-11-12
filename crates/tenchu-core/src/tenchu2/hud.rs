use super::{actor::Actor, ai::status::Ki, inventory::Inventory};
use crate::Result;
use crate::tenchu2::actor::flags::POISONED;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Image {
    Item(usize),
    Selection,
    PlayerFrame,
    PlayerFill,
    EnemyFrame,
    EnemyFill,
    Ki(usize),
    Digit(u8),
    Aim,
    Bandage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprite {
    pub image: Image,
    pub position: [i16; 2],
    pub width: Option<u16>,
    pub scale: i16,
    pub rotation: i32,
    pub color: [u8; 3],
}

impl Sprite {
    fn new(image: Image, position: [i16; 2]) -> Self {
        Self {
            image,
            position,
            width: None,
            scale: 4096,
            rotation: 0,
            color: [128; 3],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primitive {
    Sprite(Sprite),
    Breath(u16),
}

#[derive(Clone, Copy)]
pub struct Health {
    pub current: i16,
    pub maximum: i16,
    pub poisoned: bool,
}

impl From<&Actor> for Health {
    fn from(actor: &Actor) -> Self {
        Self {
            current: actor.health,
            maximum: actor.max_health,
            poisoned: actor.flags & POISONED != 0,
        }
    }
}

impl Health {
    fn width(self, full: i32, minimum: i32) -> Result<u16> {
        let fraction = (i32::from(self.current) * full)
            .checked_div(i32::from(self.maximum))
            .ok_or("Zero maximum health in Tenchu 2 HUD")?;
        Ok((fraction + minimum) as u16)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Enemy {
    actor: usize,
    remaining: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pickup {
    item: usize,
    remaining: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hud {
    pub frame: Vec<Primitive>,
    pulse: u8,
    enemies: [Option<Enemy>; 5],
    pickups: [Option<Pickup>; 4],
    offset: usize,
    rotation: i32,
    reticle_rotation: i32,
    item_colors: [u8; 22],
    bandages: u16,
    bandage_selected: bool,
}

impl Default for Hud {
    fn default() -> Self {
        Self {
            frame: Vec::new(),
            pulse: 0,
            enemies: Default::default(),
            pickups: Default::default(),
            offset: 0,
            rotation: 0,
            reticle_rotation: 0,
            item_colors: [128; 22],
            bandages: 0,
            bandage_selected: false,
        }
    }
}

impl Hud {
    pub fn bandage_selected(&self) -> bool {
        self.bandage_selected
    }

    pub fn set_bandages(&mut self, count: u16, inventory: &Inventory) {
        self.bandages = count.min(99);
        if count == 0 {
            self.bandage_selected = false;
        } else if inventory.amounts().iter().all(|&count| count == 0) {
            self.bandage_selected = true;
        }
    }

    fn counts(&self, inventory: &Inventory) -> [u8; 22] {
        let amounts = inventory.amounts();
        std::array::from_fn(|index| {
            if index == 21 {
                self.bandages as u8
            } else {
                amounts[index]
            }
        })
    }

    fn selected(&self, inventory: &Inventory) -> Option<usize> {
        if self.bandage_selected {
            Some(21)
        } else {
            inventory.selected.map(|i| i.index())
        }
    }

    pub(crate) fn hide_health(&mut self, actor: usize) {
        for slot in &mut self.enemies {
            if slot.as_ref().is_some_and(|enemy| enemy.actor == actor) {
                *slot = None;
            }
        }
    }

    pub fn aim(&mut self, reticle: Option<super::items::aim::Reticle>) {
        let Some(reticle) = reticle else { return };
        if reticle.ready {
            self.reticle_rotation = self.reticle_rotation.wrapping_add(0x4000);
        }
        let mut sprite = Sprite::new(Image::Aim, [0, 0]);
        sprite.scale = reticle.scale;
        sprite.rotation = self.reticle_rotation;
        sprite.color = [if reticle.ready { 255 } else { 64 }; 3];
        self.frame.insert(0, Primitive::Sprite(sprite));
    }

    pub fn show_health(&mut self, actor: usize, kind: i16) {
        if self
            .enemies
            .iter()
            .flatten()
            .any(|slot| slot.actor == actor)
        {
            return;
        }
        if let Some(slot) = self.enemies.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(Enemy {
                actor,
                remaining: (kind & 0xe0 != 0xe0).then_some(300),
            });
        }
    }

    pub fn pickup(&mut self, item: usize) {
        if item >= 21 {
            return;
        }
        let index = self
            .pickups
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                self.pickups.rotate_left(1);
                self.pickups.len() - 1
            });
        self.pickups[index] = Some(Pickup {
            item,
            remaining: 60,
        });
    }

    pub fn select(&mut self, inventory: &mut Inventory, previous: bool) {
        let Some(old) = self.selected(inventory) else {
            return;
        };
        let counts = self.counts(inventory);
        let selected = (1..=22).find_map(|step| {
            let index = (old + if previous { 22 - step } else { step }) % 22;
            (counts[index] != 0).then_some(index)
        });
        self.bandage_selected = selected == Some(21);
        if !self.bandage_selected {
            inventory.selected =
                selected.map(|i| super::inventory::Item::try_from(i as u8).unwrap());
        }
        let Some(selected) = selected else {
            return;
        };
        let last = counts.iter().filter(|&&n| n != 0).count() - 1;
        if previous {
            self.offset = if self.offset == 0 && last < 5 {
                last
            } else {
                self.offset.saturating_sub(1)
            };
        } else if counts[old] != 0 {
            if last < 5 {
                self.offset = if self.offset < last {
                    self.offset + 1
                } else {
                    0
                };
            } else if self.offset < 4 {
                self.offset += 1;
            }
        }
        self.offset = self.offset.min(last);
        self.item_colors[old] = 96;
        self.item_colors[selected] = 128;
    }

    pub fn step(
        &mut self,
        player: Health,
        breath: u8,
        inventory: &Inventory,
        ki: Ki,
        mut health: impl FnMut(usize) -> Option<Health>,
    ) -> Result<()> {
        self.frame.clear();
        self.inventory(inventory);
        self.number(i32::from(player.current), [-64, 96]);
        let mut fill = Sprite::new(Image::PlayerFill, [-87, 88]);
        let width = player.width(57, 0)?;
        let color: u8 = if width <= 13 && self.pulse & 1 != 0 {
            224
        } else {
            128
        };
        fill.width = Some(width);
        fill.color = if player.poisoned {
            [0, color.wrapping_add(31), 0]
        } else {
            [color; 3]
        };
        self.frame.push(Primitive::Sprite(fill));
        if breath != 100 {
            self.frame
                .push(Primitive::Breath(u16::from(breath) * 51 / 100));
        }
        if ki != Ki::Clear {
            let color = if self.pulse < 16 {
                self.pulse.wrapping_mul(8).wrapping_add(32)
            } else {
                31u8.wrapping_sub(self.pulse.wrapping_mul(8))
            };
            let (image, advance) = match ki {
                Ki::Nearby(_) => (0, 1),
                Ki::Spotted => (1, 2),
                Ki::Suspicious => (2, 4),
                Ki::Searching => (3, 4),
                Ki::Clear => unreachable!(),
            };
            let mut sprite = Sprite::new(Image::Ki(image), [-130, 85]);
            sprite.color = [color; 3];
            if let Ki::Nearby(value) = ki {
                self.number(i32::from(value), [-110, 97]);
                sprite.scale = (i32::from(value) * 3072 / 100 + 1024) as i16;
            }
            self.frame.push(Primitive::Sprite(sprite));
            self.pulse += advance;
        }
        self.frame.push(Primitive::Sprite(Sprite::new(
            Image::PlayerFrame,
            [-100, 82],
        )));
        if self.pulse > 30 {
            self.pulse = 0;
        }
        for index in 0..self.enemies.len() {
            let Some(slot) = &self.enemies[index] else {
                continue;
            };
            let Some(health) = health(slot.actor) else {
                self.enemies[index] = None;
                continue;
            };
            let x = index as i16 * 60;
            self.number(i32::from(health.current), [x - 96, -87]);
            let width = health.width(50, 3)?;
            let color: u8 = if width < 15 && self.pulse & 1 != 0 {
                32
            } else {
                128
            };
            let mut fill = Sprite::new(Image::EnemyFill, [x - 120, -92]);
            fill.width = Some(width);
            fill.color = if health.poisoned {
                [32, color.wrapping_add(63), 32]
            } else {
                [color; 3]
            };
            self.frame.push(Primitive::Sprite(fill));
            self.frame.push(Primitive::Sprite(Sprite::new(
                Image::EnemyFrame,
                [x - 120, -92],
            )));
            if let Some(remaining) = &mut self.enemies[index].as_mut().unwrap().remaining {
                *remaining -= 1;
                if *remaining == 0 {
                    self.enemies[index] = None;
                }
            }
        }
        Ok(())
    }

    fn number(&mut self, value: i32, [mut x, y]: [i16; 2]) {
        let mut value = value.unsigned_abs();
        loop {
            let image = Image::Digit((value % 10) as u8);
            self.frame
                .push(Primitive::Sprite(Sprite::new(image, [x, y])));
            let mut shadow = Sprite::new(image, [x + 1, y + 1]);
            shadow.color = [0; 3];
            self.frame.push(Primitive::Sprite(shadow));
            x -= 5;
            value /= 10;
            if value == 0 {
                break;
            }
        }
    }

    fn inventory(&mut self, inventory: &Inventory) {
        let Some(selected) = self.selected(inventory) else {
            return;
        };
        let counts = self.counts(inventory);
        let mut start = selected;
        if self.offset != 0 {
            let Some(found) = (0..22)
                .rev()
                .map(|i| (start + i) % 22)
                .filter(|&i| counts[i] != 0)
                .nth(self.offset - 1)
            else {
                return;
            };
            start = found;
        }
        for (slot, item) in (0..22)
            .map(|i| (start + i) % 22)
            .filter(|&i| counts[i] != 0)
            .take(5)
            .enumerate()
        {
            let x = 128 - slot as i16 * 24;
            if item != 0 {
                self.number(i32::from(counts[item]), [x + 8, 94]);
            }
            let mut sprite = Sprite::new(
                if item == 21 {
                    Image::Bandage
                } else {
                    Image::Item(item)
                },
                [x, 86],
            );
            sprite.color = [self.item_colors[item]; 3];
            self.frame.push(Primitive::Sprite(sprite));
            if item == selected {
                self.offset = slot;
                self.rotation = self.rotation.wrapping_sub(0x4000);
                let mut sprite = Sprite::new(Image::Selection, [x, 86]);
                sprite.rotation = self.rotation;
                self.frame.push(Primitive::Sprite(sprite));
            }
        }
        let mut x = 128;
        for notice in self.pickups.iter_mut().flatten() {
            if notice.remaining & 2 != 0 {
                let mut sprite = Sprite::new(Image::Item(notice.item), [x, 58]);
                sprite.color = [self.item_colors[notice.item]; 3];
                self.frame.push(Primitive::Sprite(sprite));
            }
            x -= 24;
            notice.remaining = notice.remaining.wrapping_sub(1);
        }
        while self.pickups[0].as_ref().is_some_and(|p| p.remaining == 0) {
            self.pickups.rotate_left(1);
            self.pickups[3] = None;
        }
    }
}
