use super::Request;
use crate::{
    Result,
    tenchu2::{
        actor::{Actor, Resources, Velocity},
        input::ITEM,
        inventory::{Item, Unavailable},
    },
};
use tenchu_assets::tenchu2::Edition;

mod update;
pub use update::{ItemControlInput, ItemCue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrappleStatus {
    Failed,
    Flying,
    Attached,
    Released,
}

pub struct ItemInput {
    pub held: u16,
    pub player: bool,
    pub scripted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemSound {
    EmptyInventory,
    Unavailable,
}

impl Actor {
    pub fn item_input(
        &mut self,
        request: &mut Request,
        input: &ItemInput,
        resources: &Resources,
        mut sound: impl FnMut(ItemSound) -> Result<()>,
    ) -> Result<()> {
        let pressed = input.held & !self.pad.previous & ITEM != 0;
        if input.scripted && self.inventory.selected != Some(Item::GRAPPLE) {
            if pressed {
                sound(ItemSound::Unavailable)?;
            }
            return Ok(());
        }
        if matches!(request.action, 3 | 0x103)
            && let Some(item) = self.inventory.selected
            && self.inventory.counts[item.index()] != 0
            && let Some(action) = match u8::from(item) {
                4 => Some(0x803),
                5 => Some(0x806),
                _ => None,
            }
        {
            if self.overlay_motion(action, resources)?.is_some() {
                request.action = action;
                let _ = self.inventory.take();
            }
            return Ok(());
        }
        if !pressed {
            return Ok(());
        }
        let item = match self.inventory.take() {
            Ok(item) => item,
            Err(Unavailable::Empty) if input.player => {
                sound(ItemSound::EmptyInventory)?;
                return Ok(());
            }
            Err(_) => return Ok(()),
        };
        let action = match u8::from(item) {
            0 | 4 => 0x802,
            1 | 2 => 0x809,
            5 => 0x806,
            6 | 10 | 18 => 0x805,
            7 | 12 => 0x808,
            8 | 14 => 0x80b,
            9 => 0x80c,
            11 => 0x807,
            13 => 0x80a,
            15 if resources.registry.edition == Edition::Japan => 0x809,
            15 => 0x808,
            16 => 0x80d,
            17 => 0x80e,
            _ => {
                self.inventory.refund();
                return Ok(());
            }
        };
        request.action = action;
        request.velocity = Velocity::Motion;
        self.change_motion(action, request.velocity, resources)?;
        Ok(())
    }
}
