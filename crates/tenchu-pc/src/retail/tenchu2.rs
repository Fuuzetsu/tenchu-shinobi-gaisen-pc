use super::{
    Coverage,
    observation::{Ram, Reader},
};
use crate::source::DiscSource;
use std::path::Path;
use tenchu_assets::{
    Result,
    disc::Disc,
    tenchu2::{Edition, layout::Layout},
};
use tenchu_core::tenchu2::{
    inventory::Inventory,
    world::{Assets, Options, World},
};

fn state(world: &World, ram: &Ram, initial: bool) -> Result<()> {
    macro_rules! equal {
        ($actual:expr, $expected:expr) => {{
            let (actual, expected) = ($actual, $expected);
            if actual != expected {
                return Err(format!(
                    "{}: native {actual:?}, retail {expected:?}",
                    stringify!($actual)
                ));
            }
        }};
    }
    equal!(world.sequence, ram.short(0x8008e9bc) as u16);
    equal!(world.map, ram.short(0x8008e9c2) as usize);
    equal!(world.tick, ram.word(0x8008eaa4));
    equal!(world.scene.epoch, ram.word(0x800cee68));
    equal!(world.random.seed, ram.word(0x800d05e0));
    equal!(world.actors.len(), ram.short(0x8008e7d6) as usize);
    for (index, agent) in world.actors.iter().enumerate() {
        let address = ram.word(0x800c2f58 + index as u32 * 4);
        if !(0x80010000..0x801ffe00).contains(&address) {
            return Err(format!("Invalid actor {index} address {address:#x}"));
        }
        let actor = &agent.actor;
        let check = || -> Result<()> {
            equal!(actor.kind, i16::from(ram.byte(address + 4)));
            equal!(actor.state as u8, ram.byte(address + 5));
            equal!(actor.flags, ram.word(address));
            equal!(actor.health, ram.short(address + 8));
            equal!(actor.max_health, ram.short(address + 10));
            equal!(actor.width, ram.short(address + 12));
            equal!(actor.height, ram.short(address + 14));
            equal!(actor.position, ram.vector(ram.word(address + 0x2c)));
            equal!(actor.previous, ram.vector(address + 0x1c));
            equal!(
                actor.rotation.to_array(),
                ram.short_vector(ram.word(address + 0x30))
            );
            equal!(actor.velocity.to_array(), ram.short_vector(address + 0x34));
            equal!(actor.last_vertical_velocity, ram.short(address + 0x16));
            equal!(actor.motion.action, ram.short(address + 0x78));
            equal!(actor.motion.frame, ram.short(address + 0x7a));
            equal!(actor.motion.loops, ram.short(address + 0x7c));
            equal!(actor.motion.mask, ram.word(address + 0x80));
            let clip = ram.word(address + 0x84);
            equal!(actor.motion.clip, (clip != 0).then(|| ram.short(clip)));
            let model = ram.word(address + 0x60);
            equal!(
                world.scene.models[world.instances[index].rig.root].local,
                ram.matrix(model + 8)
            );
            for (bone, &joint) in world.instances[index].rig.joints.iter().enumerate() {
                let coordinate = model + 0x5c + bone as u32 * 0x58;
                equal!(
                    actor.motion.pose.rotation[bone],
                    ram.short_vector(coordinate + 0x50)
                );
                equal!(world.scene.models[joint].local, ram.matrix(coordinate + 4));
            }
            if !initial {
                equal!(
                    actor.terrain.floor.unwrap_or(i32::MIN),
                    ram.word(address + 0x10c) as i32
                );
                equal!(actor.terrain.height, ram.word(address + 0x110) as i32);
                equal!(actor.terrain.material, ram.short(address + 0x114) as u16);
                equal!(actor.terrain.blocked, ram.short(address + 0x116) as u16);
                equal!(actor.terrain.lower, ram.short(address + 0x118) as u16);
                equal!(actor.terrain.higher, ram.short(address + 0x11a) as u16);
            }
            equal!(actor.pad.held, ram.short(address + 0xf0) as u16);
            equal!(actor.pad.previous, ram.short(address + 0xf2) as u16);
            equal!(actor.pad.time, ram.short(address + 0xf4));
            equal!(
                actor.inventory.counts.as_slice(),
                ram.bytes(address + 0x150, 20)
            );
            if agent.behavior.programs.idle == tenchu_core::tenchu2::ai::Program::Patrol {
                equal!(
                    agent.behavior.patrol.next,
                    ram.short(address + 0xd8) as usize
                );
                equal!(agent.behavior.patrol.wait, ram.short(address + 0xda));
                equal!(agent.behavior.patrol.turning, ram.byte(address + 0x98));
            }
            Ok(())
        };
        check().map_err(|e| format!("actor {index}: {e}"))?;
    }
    equal!(world.camera.eye, ram.vector(0x80094608));
    equal!(world.camera.look, ram.vector(0x80094614));
    equal!(world.camera.roll, ram.word(0x80094620) as i32);
    equal!(world.camera.mode, ram.short(0x8008ebc8));
    equal!(world.camera.previous, ram.short(0x8008ebc0));
    equal!(
        world.camera.direction,
        [ram.short(0x8008ebcc), ram.short(0x8008ebce)]
    );
    equal!(world.camera.snap, ram.short(0x8008ebd0) != 0);
    if !initial {
        equal!(
            world.collision.slots().len(),
            ram.short(0x8008e816) as usize
        );
        for (index, slot) in world.collision.slots().iter().enumerate() {
            let address = 0x800c3038 + index as u32 * 48;
            equal!(slot.position, ram.vector(address + 4));
            equal!(slot.offset.to_array(), ram.short_vector(address + 0x14));
            equal!(slot.size.to_array(), ram.short_vector(address + 0x1c));
            equal!(slot.flags, ram.short(address + 0x1a) as u16);
            let contacts = u64::from_le_bytes(ram.bytes(address + 0x28, 8).try_into().unwrap());
            let expected: Vec<_> = (0..64).filter(|bit| contacts & (1 << bit) != 0).collect();
            equal!(
                serde_json::to_value(slot).map_err(|e| e.to_string())?["contacts"].clone(),
                serde_json::json!(expected)
            );
        }
    }
    Ok(())
}

pub fn check(disc: &Disc<DiscSource>, path: &Path, coverage: Coverage) -> Result<()> {
    if Edition::detect(disc)? != Edition::Japan {
        return Err("The native T2 retail comparison requires the Japanese disc".into());
    }
    if coverage == Coverage::Prefix(0) {
        return Err("The update limit must be positive".into());
    }
    if coverage == Coverage::Complete {
        return Err("T2 retail checking currently covers bounded gameplay; use --scenario or an update limit".into());
    }
    let mut reader = Reader::tenchu2(path)?;
    let mut game: Option<(World, Assets)> = None;
    let mut pending = false;
    let mut display_flags = None;
    let mut partial_update = false;
    let mut checked = 0;
    let mut elapsed_counts = [0u64; 4];
    let mut events = Events;
    let mut seed = None;
    while let Some(row) = reader.next()? {
        if row.pc == 0x80064d3c && row.snapshot {
            seed = Some(row.seed);
        }
        if game.is_none() && row.pc == 0x80065314 && row.clock == 0 && row.snapshot {
            let ram = &reader.ram;
            let hero = ram.short(0x8008e9be) as usize;
            let layout = Layout::load(disc, row.stage as usize)?;
            let assets = Assets::load(disc, hero, &layout)?;
            let player = ram.word(0x8008e9c4);
            let inventory = Inventory {
                counts: ram.bytes(player + 0x150, 20).try_into().unwrap(),
                ..Default::default()
            };
            let mut world = World::new(
                &layout,
                Options {
                    hero,
                    difficulty: ram.short(0x8008e82e) as u8,
                    seed: seed.ok_or("Missing native T2 actor initialization boundary")?,
                    inventory,
                },
                &assets,
            )?;
            // Loading and the preceding cinematic also consume the shared stream.
            // As in the T1 checker, take its value once at the gameplay boundary.
            world.random = tenchu_core::random::Random::new(row.seed);
            world.scene.epoch = ram.word(0x800cee68);
            for (index, agent) in world.actors.iter_mut().enumerate() {
                let address = ram.word(0x800c2f58 + index as u32 * 4);
                let floor = ram.word(address + 0x10c) as i32;
                agent.actor.terrain.floor = (floor != i32::MIN).then_some(floor);
            }
            state(&world, ram, true)
                .map_err(|e| format!("Field {} initialization: {e}", row.field))?;
            eprintln!(
                "T2 sequence {} initialized at field {} seed {:08x}",
                row.stage, row.field, row.seed
            );
            game = Some((world, assets));
        }
        let Some((world, assets)) = &mut game else {
            continue;
        };
        if row.pc == 0x80065314 {
            partial_update = true;
        }
        if row.pc == 0x80017280 && row.caller == 0x8006570c {
            if pending || !row.snapshot {
                return Err("Invalid T2 update boundary".into());
            }
            world.begin_recorded_update([row.pad as u16, 0], assets, &mut events)?;
            state(world, &reader.ram, false).map_err(|e| {
                format!(
                    "Field {} clock {} before display: {e}",
                    row.field, row.clock
                )
            })?;
            pending = true;
            display_flags = None;
        } else if row.pc == 0x800173b8 && pending {
            if display_flags.replace(row.skip).is_some() || !matches!(row.skip, 0 | 1 | 5 | 6) {
                return Err("Unsupported T2 display decision".into());
            }
        } else if row.pc == 0x8006571c && pending {
            let elapsed = display_flags
                .take()
                .map_or(0, |flags| if flags & 4 != 0 { 3 } else { 1 });
            if row.clock != world.tick.wrapping_add(elapsed) {
                return Err("T2 display decision disagrees with its clock advance".into());
            }
            let budget = reader.ram.short(0x8008e7d4) as u8;
            world.finish_recorded_update(assets, budget, elapsed as u8)?;
            state(world, &reader.ram, false).map_err(|e| {
                format!("Field {} clock {} after display: {e}", row.field, row.clock)
            })?;
            world.advance_mission(assets)?;
            pending = false;
            partial_update = false;
            checked += 1;
            elapsed_counts[elapsed as usize] += 1;
            if matches!(coverage, Coverage::Prefix(limit) if checked >= limit) {
                break;
            }
        }
    }
    if checked == 0 {
        return Err("No native T2 gameplay updates validated".into());
    }
    println!(
        "{}",
        serde_json::json!({
            "matched_updates": checked,
            "display_clock_advances": elapsed_counts,
            "pending_display_boundary": pending,
            "trailing_partial_update": partial_update,
            "state_corrections_after_initialization": 0,
        })
    );
    Ok(())
}

struct Events;
impl tenchu_core::tenchu2::environment::Events for Events {
    fn emit(
        &mut self,
        _: tenchu_core::tenchu2::environment::Event,
        _: &mut tenchu_core::random::Random,
    ) -> Result<()> {
        Ok(())
    }
}
impl tenchu_core::tenchu2::ai::behavior::Events for Events {
    fn voice(&mut self, _: &tenchu_core::tenchu2::actor::Actor, _: u8) -> Result<()> {
        Ok(())
    }
}
impl tenchu_core::tenchu2::world::controls::Events for Events {
    fn emit(
        &mut self,
        _: tenchu_core::tenchu2::world::controls::EventSource<'_>,
        _: tenchu_core::tenchu2::world::controls::Event,
        _: &mut tenchu_core::random::Random,
    ) -> Result<()> {
        Ok(())
    }
}
