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
        control::Request,
        physics::{Terrain, Water},
        terrain::{Cursor, Options, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_JUMP_ORACLE from original airborne controllers"]
fn native_air_control_wall_jumps_and_landings_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let idle = registry.clip(0, 0).unwrap();
    let mut resources = Resources {
        clips: motions::collect(&disc, [idle, registry.clip(0, 0x605).unwrap()].into()).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let template = template(&resources);
    let rules = Rules::load(&disc).unwrap();
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut checked = 0;
    for (index, row) in cases::<Value>("TENCHU2_JUMP_ORACLE").enumerate() {
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let bool = |name: &str| row[name].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        resources.clips.get_mut(&idle).unwrap().duration = int("duration") as i16;
        let mut actor = template.clone();
        actor.state = State::Jumping;
        actor.motion.action = int("action") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.position = vector(&row["position"]);
        actor.velocity = vector(&row["velocity"]);
        actor.height = int("height") as i16;
        actor.width = int("width") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.flags = int("flags") as u32;
        actor.pad.previous = int("previous") as u16;
        actor.terrain.height = int("ground");
        let mut scratch = probe(&row["scratch"]);
        let mut cursor = Cursor::default();
        let mut water = Water::default();
        let mut terrain = Terrain {
            map: map.as_ref().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options {
                avoid_water: bool("avoid"),
                ignore_steps: bool("ignore"),
            },
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut attacks = 0;
        let effects = actor
            .update_jump(
                &mut request,
                int("input") as u16,
                &mut terrain,
                &resources,
                |_, _| {
                    attacks += 1;
                    Ok(())
                },
            )
            .unwrap();
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({
            "position": actor.position.to_array(), "velocity": actor.velocity.to_array(),
            "yaw": actor.rotation.y, "root_yaw": actor.motion.pose.rotation[0][1],
            "action": request.action, "mode": match request.velocity { Velocity::Stop => -1, Velocity::Keep => 0, Velocity::Motion => 1 },
            "motion": actor.motion.action, "frame": actor.motion.frame, "loops": actor.motion.loops,
            "scratch": probe_values(terrain.scratch), "current": surface(terrain.cursor.current), "last": surface(terrain.cursor.last),
            "material": terrain.cursor.material, "index": terrain.cursor.index, "ignore": terrain.options.ignore_steps,
            "change": u8::from(actor.motion.clip != Some(idle)), "attacks": attacks,
            "voices": effects.voices, "wall_jump": effects.wall_jump,
        });
        assert_eq!(actual, row["expected"], "case {index}");
        checked += 1;
    }
    assert_eq!(checked, 13_800);
}
