use super::{Content, Debug, Item};
use crate::debug::InventorySlot;
use tenchu_assets::Result;
use tenchu_core::tenchu2::{inventory, world::World};

const ITEMS: &[(u8, &str)] = &[
    (0, "Grappling hook"),
    (1, "Healing potion"),
    (2, "Antidote"),
    (4, "Shuriken"),
    (5, "Caltrops"),
    (6, "Grenade"),
    (7, "Mine"),
    (8, "Poison dart"),
    (9, "Fire arrow"),
    (10, "Smoke bomb"),
    (11, "Poisoned rice"),
    (12, "Coloured rice"),
    (13, "Invisibility"),
    (14, "Confusion powder"),
    (15, "Rice ball"),
    (16, "Teleport"),
    (17, "Fire breath"),
    (18, "Sleeping bomb"),
    (20, "Resurrection leaf"),
];

pub(super) enum Edit {
    GodMode(bool),
    Armor(bool),
    Sight(u16),
    Stealth(bool),
    Inventory(inventory::Item, u8),
}

pub(super) fn sync(debug: &mut Debug, content: &Content) {
    debug.inventory.clear();
    debug.inventory_unlimited = false;
    debug.god_mode = false;
    debug.armor = false;
    debug.sight_percent = 100;
    debug.stealth = false;
    if let Content::Gameplay(session) = content
        && !session.cinematic()
    {
        let world = &session.world;
        let actor = &world.actors[world.player].actor;
        debug.god_mode = actor.invulnerable;
        debug.armor = world.controls.armor;
        debug.sight_percent = world.ai.sight_percent;
        debug.stealth = world.ai.ignore_player;
        debug.inventory.extend(ITEMS.iter().map(|&(id, name)| {
            InventorySlot {
                id: usize::from(id),
                name,
                count: actor
                    .inventory
                    .count(inventory::Item::try_from(id).unwrap()),
            }
        }));
    }
}

pub(super) fn change(
    debug: &mut Debug,
    pending: &mut Vec<Edit>,
    item: Item,
    delta: i32,
    reset: bool,
) -> Result<bool> {
    if !matches!(
        item,
        Item::GodMode
            | Item::Armor
            | Item::SightDistance
            | Item::Stealth
            | Item::Inventory(_)
            | Item::GiveItems
            | Item::ClearItems
    ) {
        return Ok(false);
    }
    if debug.inventory.is_empty() {
        return Err("Start a mission before editing the player or equipment.".into());
    }
    match item {
        Item::GodMode => {
            debug.god_mode = !reset && !debug.god_mode;
            pending.push(Edit::GodMode(debug.god_mode));
        }
        Item::Armor => {
            debug.armor = !reset && !debug.armor;
            pending.push(Edit::Armor(debug.armor));
        }
        Item::SightDistance => {
            let slider = item.slider().unwrap();
            debug.sight_percent = if reset {
                100
            } else {
                i32::from(debug.sight_percent)
                    .saturating_add(delta.saturating_mul(slider.step))
                    .clamp(slider.min, slider.max) as u16
            };
            pending.push(Edit::Sight(debug.sight_percent));
        }
        Item::Stealth => {
            debug.stealth = !reset && !debug.stealth;
            pending.push(Edit::Stealth(debug.stealth));
        }
        Item::Inventory(id) => {
            let slot = debug
                .inventory
                .iter_mut()
                .find(|s| s.id == id)
                .ok_or("Invalid native equipment slot")?;
            slot.count = if reset {
                0
            } else {
                i32::from(slot.count).saturating_add(delta).clamp(0, 255) as u8
            };
            pending.push(Edit::Inventory(
                inventory::Item::try_from(id as u8)?,
                slot.count,
            ));
        }
        Item::GiveItems | Item::ClearItems if !reset => {
            for slot in &mut debug.inventory {
                slot.count = if item == Item::ClearItems {
                    0
                } else if slot.id == 0 {
                    1
                } else {
                    10
                };
                pending.push(Edit::Inventory(
                    inventory::Item::try_from(slot.id as u8)?,
                    slot.count,
                ));
            }
        }
        _ => return Err("This action has no default value.".into()),
    }
    Ok(true)
}

pub(super) fn apply(world: &mut World, edits: impl IntoIterator<Item = Edit>) -> Result<()> {
    let actor = &mut world.actors[world.player].actor;
    if !world.mission.input_allowed() || actor.health <= 0 {
        return Err("Player edits require a living player outside scripted transitions.".into());
    }
    for edit in edits {
        match edit {
            Edit::GodMode(enabled) => actor.invulnerable = enabled,
            Edit::Armor(enabled) => world.controls.armor = enabled,
            Edit::Sight(percent) => world.ai.sight_percent = percent,
            Edit::Stealth(enabled) => world.ai.ignore_player = enabled,
            Edit::Inventory(item, count) => actor.inventory.set_amount(item, count),
        }
    }
    Ok(())
}
