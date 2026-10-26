use super::observation::Ram;
use tenchu_assets::Result;
use tenchu_core::game::Game;

pub(super) fn state(game: &Game, ram: &Ram) -> Result<()> {
    let mut differences = Vec::new();
    macro_rules! equal {
        ($native:expr, $reference:expr) => {
            if ($native) != ($reference) && differences.len() < 30 {
                differences.push(format!(
                    "{}: native {:?}, retail {:?}",
                    stringify!($native),
                    $native,
                    $reference
                ));
            }
        };
    }
    equal!(game.clock, ram.word(0x800976b0) as i32);
    equal!(game.random.seed, ram.word(0x800c79e0));
    equal!(game.roster.order.len(), ram.short(0x80097724) as usize);
    equal!(game.statistics.criticals, ram.short(0x80097c6c));
    equal!(game.statistics.murders, ram.short(0x80097c70));
    equal!(game.statistics.friend_hits, ram.short(0x80097c72));
    equal!(game.statistics.bosses, ram.short(0x80097c74));
    equal!(game.statistics.enemies, ram.short(0x80097c76));
    equal!(game.statistics.citizens, ram.short(0x80097c78));
    if game.clock > 0 {
        equal!(
            game.camera.smoothing_state(),
            std::array::from_fn::<_, 2, _>(|i| std::array::from_fn::<_, 6, _>(
                |j| ram.short(0x80089f10 + i as u32 * 16 + j as u32 * 2)
            ))
        );
        equal!(game.camera.position, ram.vector(0x800b8080));
        equal!(game.camera.reference, ram.vector(0x800b808c));
        equal!(game.camera.target, ram.vector(0x80089ef0));
        equal!(game.camera.view, ram.matrix(0x800c6520));
        equal!(game.camera.mode.source_id(), ram.word(0x80089f04));
        equal!(
            game.camera.direction,
            [ram.short(0x80089f08), ram.short(0x80089f0a)]
        );
        equal!(game.activation.raw_budget, ram.short(0x80097f40));
        equal!(game.activation.budget, ram.short(0x80097f44));
        equal!(game.intelligence.action_halt, ram.short(0x800979b8));
        equal!(game.intelligence.found, ram.short(0x80097c6e));
        equal!(game.events.time, ram.word(0x80097c64) as i32);
        let page = ram.short(0x800976a4) as u32;
        equal!(
            game.drawing.bytes,
            ram.word(0x800c6578).wrapping_sub(0x80098040 + page * 65536)
        );
        equal!(game.drawing.sampled_bytes, ram.word(0x800976b8));
        equal!(game.collision.len, ram.short(0x800976ec) as usize);
        for (index, slot) in game.collision.slots[..game.collision.len]
            .iter()
            .enumerate()
        {
            let ptr = 0x800bc108 + index as u32 * 0x78;
            equal!(slot.position, ram.vector(ptr + 4));
            equal!(slot.offset, ram.short_vector(ptr + 0x14));
            equal!(slot.size, ram.short_vector(ptr + 0x1c));
            equal!(slot.count, ram.short(ptr + 0x1a));
            equal!(slot.class, ram.short(ptr + 0x22));
            equal!(slot.results.as_slice(), ram.bytes(ptr + 0x28, 80));
        }
    }
    for (index, &id) in game.roster.order.iter().enumerate() {
        let address = ram.word(0x800be7b8 + index as u32 * 4);
        let actor = game.roster.get(id);
        let motion = ram.word(address + 0x5c);
        let before = differences.len();
        equal!(actor.kind, ram.short(address));
        equal!(actor.status, ram.short(address + 2));
        equal!(actor.attribute, ram.short(address + 4) as u16);
        equal!(actor.health, ram.short(address + 8));
        equal!(actor.position, ram.vector(ram.word(address + 0x38)));
        equal!(actor.rotation, ram.short_vector(ram.word(address + 0x3c)));
        equal!(actor.velocity, ram.short_vector(address + 0x40));
        equal!(actor.previous, ram.vector(address + 0x48));
        equal!(actor.motion.action, ram.short(motion));
        equal!(actor.motion.count, ram.short(motion + 2));
        equal!(actor.motion.loops, ram.short(motion + 4));
        equal!(actor.map.level, ram.word(address + 0x20) as i32);
        equal!(actor.map.height, ram.word(address + 0x24) as i32);
        equal!(actor.map.attribute, ram.short(address + 0x28) as u16);
        equal!(actor.map.vector, ram.byte(address + 0x2c));
        equal!(actor.map.angle_low, ram.byte(address + 0x2e));
        equal!(actor.map.angle_high, ram.byte(address + 0x2f));
        let terrain = if actor.kind == 0x85 && ram.word(address + 0x30) >= ram.word(0x800976e8) {
            ram.word(0x800976e8)
        } else {
            ram.word(0x800976e4)
        };
        equal!(
            actor.map.area,
            ram.word(address + 0x30).wrapping_sub(terrain) as usize
        );
        equal!(
            actor.map.row,
            (ram.word(address + 0x34).wrapping_sub(terrain) / 16) as usize
        );
        equal!(actor.pad.held, ram.short(address + 0x10) as u16);
        equal!(actor.pad.previous, ram.short(address + 0x12) as u16);
        equal!(actor.pad.trigger, ram.short(address + 0x14) as u16);
        equal!(actor.pad.time, ram.short(address + 0x16));
        equal!(
            actor.pad.stream,
            std::array::from_fn::<_, 4, _>(|i| ram.short(address + 0x18 + i as u32 * 2) as u16)
        );
        equal!(actor.pad.hold_counter, ram.word(address + 0xb0));
        equal!(actor.inventory.as_slice(), ram.bytes(address + 0xb4, 26));
        equal!(
            actor.brain.chase,
            [
                ram.word(address + 0x80) as i32,
                ram.word(address + 0x84) as i32
            ]
        );
        equal!(
            [
                actor.brain.mode,
                actor.brain.flag,
                actor.brain.count,
                actor.brain.timer
            ]
            .as_slice(),
            ram.bytes(address + 0x88, 4)
        );
        if game.clock > 0 && actor.attribute & tenchu_core::ids::ATTR_SUSPEND == 0 {
            let model = ram.word(address + 0x58);
            let parts = ram.word(model + 0x68);
            for (part, id) in std::iter::once(actor.root_model)
                .chain(actor.parts.iter().copied())
                .enumerate()
            {
                let ptr = if part == 0 {
                    model
                } else {
                    ram.word(parts + (part as u32 - 1) * 4)
                };
                let coordinate = &game.scene.models[id];
                equal!(coordinate.local, ram.matrix(ptr + 4));
                equal!(coordinate.flags == 0, ram.word(ptr) == 0);
                if ram.word(ptr) != 0 {
                    equal!(coordinate.cached, ram.matrix(ptr + 0x24));
                }
            }
        }
        for difference in &mut differences[before..] {
            *difference = format!("actor {index} (kind {:x}): {difference}", actor.kind);
        }
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("\n"))
    }
}
