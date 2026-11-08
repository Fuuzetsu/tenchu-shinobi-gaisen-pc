use super::{
    source_control::{cases, template},
    source_physics::{probe, probe_values, vector},
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    collision::Rules, commands::Commands, level::Level, math_tables, motions, registry::Registry,
    weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{Request, WallCue, WallInput},
        inventory::Item,
        physics::{Terrain, Water},
        terrain::{Cursor, Options, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_WALL_ORACLE from original wall controllers"]
fn native_wall_alignment_movement_peeking_and_item_throws_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_WALL_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let commands = Commands::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        let kind = row["kind"].as_i64().unwrap() as i16;
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
        for action in commands.crouched.iter().map(|c| c.action).chain([
            0x113, 0x114, 0x210, 0x211, 0x212, 0x213, 0x300, 0x301, 0x302, 0x700,
        ]) {
            wanted.extend(registry.clip(kind, action));
        }
    }
    wanted.extend(registry.clip(0, 0));
    let mut resources = Resources {
        clips: motions::collect(&disc, wanted)
            .unwrap()
            .into_iter()
            .map(|(id, c)| (id, c.at_gameplay_rate(registry.edition)))
            .collect(),
        registry,
        commands,
        weapons: Weapons::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let keys: std::collections::BTreeMap<_, _> = resources
        .clips
        .iter()
        .map(|(&id, c)| (id, [c.root[0], c.bones[0][0], c.bones[2][0]]))
        .collect();
    let template = template(&resources);
    let rules = Rules::load(&disc).unwrap();
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut changed_keys = 0;
    let mut throws = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |k: &str| row[k].as_i64().unwrap() as i32;
        let boolean = |k: &str| row[k].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let clip_id = int("clip") as i16;
        let clip = resources.clips.get_mut(&clip_id).unwrap();
        [clip.root[0], clip.bones[0][0], clip.bones[2][0]] = keys[&clip_id];
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::Wall;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(clip_id);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = u32::MAX - 1;
        actor.motion.pose.rotation[0] = vector::<i16>(&row["root_rot"]).to_array();
        actor.motion.pose.translation[0] = vector(&row["root_pos"]);
        actor.flags = int("flags") as u32;
        actor.position = vector(&row["position"]);
        actor.velocity = vector(&row["velocity"]);
        actor.rotation.y = int("yaw") as i16;
        actor.height = int("height") as i16;
        actor.width = int("width") as i16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.pad.previous = int("previous") as u16;
        actor.pad.history = std::array::from_fn(|i| row["history"][i].as_u64().unwrap() as u16);
        actor.inventory.selected = Item::try_from(int("selected") as u8).ok();
        actor.inventory.active = Item::try_from(int("active") as u8).unwrap();
        actor.inventory.counts = std::array::from_fn(|i| row["counts"][i].as_u64().unwrap() as u8);
        let mut cursor = Cursor::default();
        let mut scratch = probe(&row["scratch"]);
        let mut water = Water::default();
        let mut terrain = Terrain {
            map: map.as_mut().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options::default(),
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut input = WallInput {
            held: int("input") as u16,
            player: boolean("player"),
            allowed: boolean("wall"),
            camera: int("camera") as i16,
        };
        let mut calls = Vec::new();
        actor
            .update_wall(
                &mut request,
                &mut input,
                &mut terrain,
                &mut resources,
                |actor, request, held, _, cue| {
                    match cue {
                        WallCue::PublishRoot => calls.push(json!(["publish"])),
                        WallCue::Recover => {
                            request.action = actor.normal_motion(held, |actor| {
                                calls.push(json!(["root"]));
                                Ok(actor.position)
                            })?;
                            request.velocity = Velocity::Motion;
                        }
                        WallCue::Voice(value) => calls.push(json!(["voice", value])),
                        WallCue::Sound(_) => calls.push(json!(["sound", 6])),
                        WallCue::Throw { item, direction } => {
                            throws += 1;
                            calls.push(json!([
                                "throw",
                                u8::from(item),
                                matches!(u8::from(item), 6 | 18).then_some(direction.to_array())
                            ]));
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();
        let state = match actor.state {
            State::Normal => 0,
            State::Ready => 1,
            State::Crouched => 2,
            State::Wall => 3,
            State::Ledge => 4,
            State::Swimming => 5,
            State::Jumping => 6,
            State::Special => 7,
            State::Item => 8,
            State::Attack => 9,
            State::Hurt => 10,
            State::Dead => 11,
        };
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let clip = &resources.clips[&clip_id];
        let actual = json!({"action":request.action,"mode":match request.velocity{Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},
            "input":input.held,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),"yaw":actor.rotation.y,
            "root_rot":actor.motion.pose.rotation[0],"root_pos":actor.motion.pose.translation[0].to_array(),"mask":actor.motion.mask,
            "motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":state,
            "history":actor.pad.history,"active":u8::from(actor.inventory.active),"counts":actor.inventory.counts,"clip_root":clip.root[0].value,
            "clip_yaws":[clip.bones[0][0].value[1],clip.bones[2][0].value[1]],"scratch":probe_values(terrain.scratch),"current":surface(terrain.cursor.current),
            "last":surface(terrain.cursor.last),"material":terrain.cursor.material,"index":terrain.cursor.index,"calls":calls});
        assert_eq!(
            actual,
            row["expected"],
            "case {id}, action {}",
            int("action")
        );
        changed_keys +=
            usize::from(clip.root[0] != keys[&clip_id][0] || clip.bones[0][0] != keys[&clip_id][1]);
    }
    assert_eq!(rows.len(), 18_400);
    assert!(changed_keys > 100 && throws > 100);
    eprintln!(
        "18400 source wall checks: {changed_keys} motion-key adjustments, {throws} item throws"
    );
}
