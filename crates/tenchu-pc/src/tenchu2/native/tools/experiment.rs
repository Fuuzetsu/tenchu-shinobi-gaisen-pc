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
    Bandages(u16),
    Bandage,
    Behavior(usize, tenchu_core::ai::profiles::Overrides),
}

pub(super) fn sync(debug: &mut Debug, content: &Content) {
    debug.inventory.clear();
    debug.inventory_unlimited = false;
    debug.god_mode = false;
    debug.armor = false;
    debug.sight_percent = 100;
    debug.stealth = false;
    debug.bandages = None;
    debug.actor_behavior = None;
    debug.actor_combat = None;
    if let Content::Gameplay(session) = content
        && !session.cinematic()
    {
        let world = &session.world;
        let actor = &world.actors[world.player].actor;
        debug.god_mode = actor.invulnerable;
        debug.armor = world.controls.armor;
        debug.sight_percent = world.ai.sight_percent;
        debug.stealth = world.ai.ignore_player;
        debug.bandages = actor.bleeding.as_ref().map(|state| state.bandages);
        if world.ai.profiles.enabled
            && world.mission.input_allowed()
            && world.script == 0
            && actor.health > 0
            && world.profile_eligible(debug.selected_actor)
        {
            debug.actor_behavior = Some(
                world.actors[debug.selected_actor]
                    .behavior
                    .profile
                    .overrides,
            );
            debug.actor_combat = world
                .profile_selection(debug.selected_actor)
                .map(|s| s.combat);
        }
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
    if matches!(item, Item::ActorCombat | Item::ActorIdle) {
        use crate::settings::edit::cycle;
        use tenchu_core::ai::profiles::{Combat, Idle};
        let overrides = debug.actor_behavior.as_mut().ok_or(
            "Enable behavior profiles and select an ordinary living guard outside a scripted scene",
        )?;
        if item == Item::ActorCombat {
            overrides.combat = if reset {
                None
            } else {
                cycle(
                    overrides.combat,
                    &[None]
                        .into_iter()
                        .chain(Combat::ALL.map(Some))
                        .collect::<Vec<_>>(),
                    delta,
                )
            };
        } else {
            let choices: Vec<_> = [None].into_iter().chain(Idle::ALL.map(Some)).collect();
            overrides.idle = if reset {
                None
            } else {
                cycle(overrides.idle, &choices, delta)
            };
        }
        pending.push(Edit::Behavior(debug.selected_actor, *overrides));
        debug.actors_edited = true;
        return Ok(true);
    }
    if !matches!(
        item,
        Item::GodMode
            | Item::Armor
            | Item::SightDistance
            | Item::Stealth
            | Item::Inventory(_)
            | Item::GiveItems
            | Item::ClearItems
            | Item::Bandages
            | Item::UseBandage
    ) {
        return Ok(false);
    }
    if debug.inventory.is_empty() {
        return Err("Start a mission before editing the player or equipment.".into());
    }
    match item {
        Item::Bandages => {
            let count = debug
                .bandages
                .as_mut()
                .ok_or("Enable Player bleeding first")?;
            *count = if reset {
                0
            } else {
                i32::from(*count).saturating_add(delta).clamp(0, 99) as u16
            };
            pending.push(Edit::Bandages(*count));
        }
        Item::UseBandage if !reset => {
            let count = debug.bandages.ok_or("Enable Player bleeding first")?;
            if count == 0 {
                return Err("No bandages left".into());
            }
            pending.push(Edit::Bandage);
        }
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
    if !world.mission.input_allowed() || world.actors[world.player].actor.health <= 0 {
        return Err("Player edits require a living player outside scripted transitions.".into());
    }
    for edit in edits {
        if let Edit::Behavior(index, overrides) = edit {
            world.set_actor_behavior(index, overrides)?;
            continue;
        }
        let actor = &mut world.actors[world.player].actor;
        match edit {
            Edit::GodMode(enabled) => actor.invulnerable = enabled,
            Edit::Armor(enabled) => world.controls.armor = enabled,
            Edit::Sight(percent) => world.ai.sight_percent = percent,
            Edit::Stealth(enabled) => world.ai.ignore_player = enabled,
            Edit::Inventory(item, count) => actor.inventory.set_amount(item, count),
            Edit::Bandages(count) => {
                actor
                    .bleeding
                    .as_mut()
                    .ok_or("Enable Player bleeding first")?
                    .bandages = count
            }
            Edit::Bandage => {
                let state = actor
                    .bleeding
                    .as_mut()
                    .ok_or("Enable Player bleeding first")?;
                if !state.active {
                    return Err("No bleeding to treat".into());
                }
                if state.bandages == 0 {
                    return Err("No bandages left".into());
                }
                state.request_bandage();
            }
            Edit::Behavior(..) => unreachable!(),
        }
    }
    Ok(())
}
