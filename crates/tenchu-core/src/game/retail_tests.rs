use super::*;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const RAM_SIZE: usize = 0x200000;
const RAM_OFFSET: usize = 96;
const CLOCK: u32 = 0x800976b0;
const SEED: u32 = 0x800c79e0;
const PLAYER: u32 = 0x80097c60;

struct Trace {
    bytes: Vec<u8>,
    ranges: Vec<(u32, usize)>,
    records: usize,
    stride: usize,
    fields: usize,
}

struct Ram<'a> {
    bytes: Vec<u8>,
    ranges: Option<&'a [(u32, usize)]>,
}

impl Ram<'_> {
    fn bytes(&self, address: u32, length: usize) -> &[u8] {
        if let Some(ranges) = self.ranges {
            assert!(
                ranges.iter().any(|&(start, size)| {
                    address >= start && address as u64 + length as u64 <= start as u64 + size as u64
                }),
                "uncaptured reference field {address:08x}+{length}"
            );
        }
        let offset = (address & 0x1fffff) as usize;
        &self.bytes[offset..offset + length]
    }

    fn short(&self, address: u32) -> i16 {
        i16::from_le_bytes(self.bytes(address, 2).try_into().unwrap())
    }

    fn word(&self, address: u32) -> u32 {
        u32::from_le_bytes(self.bytes(address, 4).try_into().unwrap())
    }

    fn vector(&self, address: u32) -> [i32; 3] {
        std::array::from_fn(|i| self.word(address + i as u32 * 4) as i32)
    }

    fn short_vector(&self, address: u32) -> [i16; 3] {
        std::array::from_fn(|i| self.short(address + i as u32 * 2))
    }

    fn matrix(&self, address: u32) -> crate::math::Matrix {
        crate::math::Matrix {
            rotation: std::array::from_fn(|i| {
                std::array::from_fn(|j| self.short(address + (i * 3 + j) as u32 * 2))
            }),
            translation: self.vector(address + 20),
        }
    }
}

impl Trace {
    fn load(path: &Path, hash: &str) -> Self {
        let bytes = fs::read(path).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash);
        assert!(matches!(&bytes[..8], b"ONKTR001" | b"ONKTR002"));
        assert_eq!(&bytes[16..24], b"ONKRAM01");
        let end = bytes.len() - 32;
        assert_eq!(Sha256::digest(&bytes[..end]).as_slice(), &bytes[end..]);
        let word = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let fields = word(8);
        let count = word(12);
        let mut offset = RAM_OFFSET + RAM_SIZE;
        let mut ranges = Vec::new();
        let mut stride = 4;
        for _ in 0..count {
            let start = word(offset) as u32;
            let length = word(offset + 4);
            ranges.push((start, length));
            stride += length;
            offset += 8;
        }
        let records = offset + fields * if &bytes[..8] == b"ONKTR002" { 2 } else { 1 };
        assert_eq!(records + fields * stride, end);
        Self {
            bytes,
            ranges,
            records,
            stride,
            fields,
        }
    }

    fn root(&self) -> Ram<'_> {
        Ram {
            bytes: self.bytes[RAM_OFFSET..RAM_OFFSET + RAM_SIZE].to_vec(),
            ranges: None,
        }
    }

    fn frame(&self, field: usize) -> Ram<'_> {
        assert!(field < self.fields);
        let mut ram = self.root();
        ram.ranges = Some(&self.ranges);
        let mut offset = self.records + field * self.stride + 4;
        for &(address, length) in &self.ranges {
            let start = (address & 0x1fffff) as usize;
            ram.bytes[start..start + length].copy_from_slice(&self.bytes[offset..offset + length]);
            offset += length;
        }
        ram
    }

    fn completed_updates(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut previous = None;
        for field in 0..self.fields {
            let clock = self.frame(field).word(CLOCK);
            if previous == Some(clock) {
                *result.last_mut().unwrap() = field;
            } else {
                if let Some(before) = previous {
                    assert_eq!(clock, before + 1);
                }
                result.push(field);
            }
            previous = Some(clock);
        }
        // A final vblank sample can be inside the still-running update.
        result.pop();
        result
    }
}

fn starting_state(game: &mut Game, root: &Ram<'_>) {
    // The retail root follows SKIPFRAME_AFTER_LOAD: its first update advanced
    // motion counters without animating the player. Import that initial pose
    // once; every subsequent update uses the port's ordinary no-skip loop.
    assert_eq!(game.clock, 1);
    assert_eq!(game.random.calls, 0);
    let address = root.word(PLAYER);
    let model = root.word(address + 0x58);
    let parts = root.word(model + 0x68);
    let actor = game.roster.get_mut(game.player_id);
    let mut coordinates = vec![(actor.root_model, model)];
    for (part, &id) in actor.parts.iter().enumerate() {
        let ptr = root.word(parts + part as u32 * 4);
        coordinates.push((id, ptr));
        actor.motion.pose.translation[part] = root.vector(ptr + 24);
        actor.motion.pose.rotation[part] = root.short_vector(ptr + 0x50);
    }
    for (index, weapon) in actor.weapons.iter().enumerate() {
        if let Some(weapon) = weapon {
            coordinates.push((weapon.model, root.word(address + 0x94 + index as u32 * 4)));
        }
    }
    for (id, ptr) in coordinates {
        let coordinate = &mut game.scene.models[id];
        coordinate.local = root.matrix(ptr + 4);
        coordinate.cached = root.matrix(ptr + 0x24);
        coordinate.flags = if root.word(ptr) == 0 {
            0
        } else {
            game.scene.epoch - 1
        };
    }
    // ProcMiscSound stores the pre-loading GameClock before StartStageSequence
    // resets it. The PC launcher starts at zero; this recording starts at 644.
    for (index, object) in game.misc.objects.iter_mut().enumerate() {
        if matches!(object.kind, tenchu_assets::misc::Kind::Sound { .. }) {
            let address = 0x800c0958 + index as u32 * 36;
            assert_eq!(object.position, root.vector(address + 4));
            assert_eq!(root.word(address + 24), 644);
            object.count = 644;
        }
    }
}

fn compare(game: &Game, ram: &Ram<'_>, root: &Ram<'_>) {
    assert_eq!(game.clock, ram.word(CLOCK) as i32, "clock");
    assert_eq!(
        ram.short(CLOCK + 4),
        0,
        "fixture must not request console frame skips"
    );
    assert_eq!(game.roster.order.len(), ram.short(0x80097724) as usize);
    assert_eq!(
        game.camera.smoothing_state(),
        std::array::from_fn::<_, 2, _>(|i| std::array::from_fn::<_, 6, _>(
            |j| ram.short(0x80089f10 + i as u32 * 16 + j as u32 * 2)
        )),
        "camera history at {}",
        game.clock
    );
    assert_eq!(
        game.camera.position,
        ram.vector(0x800b8080),
        "camera position at {}",
        game.clock
    );
    assert_eq!(
        game.camera.reference,
        ram.vector(0x800b808c),
        "camera reference at {}",
        game.clock
    );
    assert_eq!(
        game.camera.target,
        ram.vector(0x80089ef0),
        "camera target at {}",
        game.clock
    );
    assert_eq!(
        game.camera.view,
        ram.matrix(0x800c6520),
        "view matrix at {}",
        game.clock
    );
    assert_eq!(
        game.activation.raw_budget,
        ram.short(0x80097f40),
        "raw budget at {}",
        game.clock
    );
    assert_eq!(
        game.activation.budget,
        ram.short(0x80097f44),
        "budget at {}",
        game.clock
    );
    assert_eq!(
        game.camera.mode.source_id(),
        ram.word(0x80089f04),
        "camera mode at {}",
        game.clock
    );
    assert_eq!(
        game.camera.direction,
        [ram.short(0x80089f08), ram.short(0x80089f0a)],
        "camera angles at {}",
        game.clock
    );
    assert_eq!(game.intelligence.action_halt, ram.short(0x800979b8));
    assert_eq!(game.intelligence.found, ram.short(0x80097c6e));
    assert_eq!(
        game.events.time,
        ram.word(0x80097c64) as i32,
        "event timer at {}",
        game.clock
    );
    assert_eq!(game.collision.len, ram.short(0x800976ec) as usize);
    for (index, slot) in game.collision.slots[..game.collision.len]
        .iter()
        .enumerate()
    {
        let ptr = 0x800bc108 + index as u32 * 0x78;
        assert_eq!(
            slot.position,
            ram.vector(ptr + 4),
            "collision {index} position at {}",
            game.clock
        );
        assert_eq!(
            slot.offset,
            ram.short_vector(ptr + 0x14),
            "collision {index} offset at {}",
            game.clock
        );
        assert_eq!(
            slot.size,
            ram.short_vector(ptr + 0x1c),
            "collision {index} size at {}",
            game.clock
        );
        assert_eq!(
            slot.count,
            ram.short(ptr + 0x1a),
            "collision {index} count at {}",
            game.clock
        );
        assert_eq!(
            slot.class,
            ram.short(ptr + 0x22),
            "collision {index} class at {}",
            game.clock
        );
        assert_eq!(
            slot.results.as_slice(),
            ram.bytes(ptr + 0x28, 80),
            "collision {index} results at {}",
            game.clock
        );
    }
    for (index, &id) in game.roster.order.iter().enumerate() {
        let address = ram.word(0x800be7b8 + index as u32 * 4);
        let actor = game.roster.get(id);
        let motion = ram.word(address + 0x5c);
        macro_rules! equal {
            ($field:expr, $expected:expr) => {
                assert_eq!(
                    $field,
                    $expected,
                    "{}: actor {index} at clock {}",
                    stringify!($field),
                    game.clock
                );
            };
        }
        if actor.attribute & crate::ids::ATTR_SUSPEND == 0 {
            let model = root.word(address + 0x58);
            let parts = root.word(model + 0x68);
            for (part, id) in std::iter::once(actor.root_model)
                .chain(actor.parts.iter().copied())
                .enumerate()
            {
                let ptr = if part == 0 {
                    model
                } else {
                    root.word(parts + (part as u32 - 1) * 4)
                };
                let native = &game.scene.models[id];
                assert_eq!(
                    native.local,
                    ram.matrix(ptr + 4),
                    "local matrix actor {index} part {part} clock {}",
                    game.clock
                );
                assert_eq!(
                    native.flags == 0,
                    ram.word(ptr) == 0,
                    "dirty matrix actor {index} part {part} clock {}",
                    game.clock
                );
                if ram.word(ptr) != 0 {
                    assert_eq!(
                        native.cached,
                        ram.matrix(ptr + 0x24),
                        "cached matrix actor {index} part {part} clock {}",
                        game.clock
                    );
                }
            }
        }
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
        equal!(actor.map.vector, ram.bytes(address + 0x2c, 1)[0]);
        equal!(actor.map.angle_low, ram.bytes(address + 0x2e, 1)[0]);
        equal!(actor.map.angle_high, ram.bytes(address + 0x2f, 1)[0]);
        let terrain = root.word(0x800976e4);
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
    }
    if game.clock > 1 {
        let page = ram.short(0x800976a4) as u32;
        let bytes = ram.word(0x800c6578) - (0x80098040 + page * 65536);
        assert_eq!(game.drawing.bytes, bytes, "drawing bytes at {}", game.clock);
        assert_eq!(
            game.drawing.sampled_bytes,
            ram.word(0x800976b8),
            "sampled drawing bytes at {}",
            game.clock
        );
    }
    assert_eq!(game.random.seed, ram.word(SEED), "RNG at {}", game.clock);
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_CASTLE_CORPUS; continuous retail comparison with initial loading-pose and ambient-schedule alignment"]
fn castle_start_matches_retail_without_state_refreshes() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("TENCHU_CASTLE_CORPUS").expect("Set TENCHU_CASTLE_CORPUS"),
    );
    for (name, ticks, hash) in [
        (
            "neutral-world",
            75,
            "672464425bf1ca320289af4bddf8a38ce8c2183689a70ceb875321ec133de795",
        ),
        (
            "walk-expanded2",
            100,
            "6e27ad49e09f42e2343dd3242ed79f3342c9a4dc9ae3bc7e409ca867cba29b2c",
        ),
        (
            "jump-expanded2",
            100,
            "33ddd7368e6209938bfc41feab690d22050200b27df3091f8427bdea7cacde0b",
        ),
        (
            "crouch",
            100,
            "9c1cf25e9222a7dcbb0042382fe686962811f7b73432ab1190ed8d2a5501d803",
        ),
        (
            "grapple",
            100,
            "53d16d64d43fe7d1532e1d8db44c9c4d6a163d3bd7879d59a43c4520c0c41985",
        ),
        (
            "walk-detect-expanded2",
            155,
            "b3c20164e7e07f0348293709fe1035f1a8c1060ecab1ab604dc608cfeb69b9c1",
        ),
        (
            "wall-grapple-long",
            4000,
            "9486c05b11149c82acd093231bf4ae5d6e89e22f6586499e3a64b84331555859",
        ),
    ] {
        eprintln!("{name}");
        let trace = Trace::load(&directory.join(format!("{name}.trace")), hash);
        let root = trace.root();
        assert_eq!(root.word(CLOCK), 1);
        assert_eq!(root.word(0x80097ac4), 6);
        assert_eq!(root.bytes(0x80010058, 1), [1]);
        assert_eq!(root.bytes(0x80010006, 1), [0]);
        let player = root.word(PLAYER);
        let mut game = crate::oracle::game(6, 1);
        assert_eq!(game.random.calls, 0);
        game.random.seed = root.word(SEED);
        game.roster
            .get_mut(game.player_id)
            .inventory
            .copy_from_slice(root.bytes(player + 0xb4, 26));
        game.events =
            events::Events::new(&game.mission.scripts.events, &game.roster, game.player_id);
        game.step(0).unwrap();
        starting_state(&mut game, &root);
        compare(&game, &root, &root);
        let updates = trace.completed_updates();
        let mut checked = 0;
        for field in updates {
            let reference = trace.frame(field);
            if reference.word(CLOCK) == 1 {
                continue;
            }
            // The recorded game's latched controller word aligns input to logic
            // updates; emulator field timing is not a PC gameplay input.
            let buttons = reference.short(player + 0x10) as u16;
            game.step(buttons).unwrap();
            compare(&game, &reference, &root);
            checked += 1;
            if checked == ticks {
                break;
            }
        }
        assert_eq!(checked, ticks);
        eprintln!("{name}: {checked} consecutive updates");
    }
}
