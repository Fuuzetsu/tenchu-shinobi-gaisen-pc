use serde_json::{Value, json};
use tenchu_assets::tenchu2::{effects::Rules, level::Level};
use tenchu_core::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        effects::Effects,
        items::{
            self, Context, Event, State, flight::Flight, grenade::Grenade, mine::Mine, smoke::Smoke,
        },
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_EXPLOSIVE_ORACLE; original JP grenade, mine and smoke routines"]
fn native_explosives_match_original_launch_bounces_fuses_and_contacts() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let rules = Rules::load(&disc).unwrap();
    let mut maps = std::collections::BTreeMap::new();
    let mut count = [0; 3];
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_EXPLOSIVE_ORACLE").enumerate()
    {
        let id = row["map"].as_u64().unwrap() as usize;
        let map = maps
            .entry(id)
            .or_insert_with(|| Level::load(&disc, id).unwrap().collision);
        let kind = row["kind"].as_str().unwrap();
        let position = |key: &str| -> Position {
            serde_json::from_value::<[i32; 3]>(row[key].clone())
                .unwrap()
                .into()
        };
        let origin = position("origin");
        let direction = position("direction");
        let yaw = row["yaw"].as_i64().unwrap() as i16;
        let action = row["action"].as_i64().unwrap() as i16;
        let mut cursor = Cursor::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut cues = Vec::new();
        let launched = if kind == "Mine" {
            json!(
                Mine::place(0, yaw, origin, map, &mut cursor, &math, |cue| {
                    if let Event::Sound { position, id } = cue {
                        cues.push(json!(["sound", position.to_array(), id]));
                    }
                    Ok(())
                })
                .unwrap()
            )
        } else if kind == "Smoke" && !matches!(action, 0x310 | 0x311) {
            json!(Smoke::place(0, yaw, origin, map, &mut cursor, &math).unwrap())
        } else {
            let wall = matches!(action, 0x310 | 0x311);
            let (origin, direction) = if wall {
                items::wall_throw(origin, position("actor"), direction, action)
            } else {
                (origin, direction)
            };
            let flight = Flight::thrown(
                origin,
                position("actor"),
                direction,
                if wall { 150 } else { 300 },
                map,
                &mut cursor,
                &math,
            )
            .unwrap();
            if kind == "Grenade" {
                json!(Grenade::launch(0, flight))
            } else {
                json!(Smoke::thrown(0, flight))
            }
        };
        assert_eq!(launched, row["launch"], "launch {case} {kind}");
        assert_eq!(
            json!(cues),
            row["launch_cues"],
            "launch events {case} {kind}"
        );
        count[match kind {
            "Grenade" => 0,
            "Mine" => 1,
            _ => 2,
        }] += 1;
        if launched.is_null() {
            continue;
        }
        let mut state: State =
            serde_json::from_value(json!({kind: row["before"].clone()})).unwrap();
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let other_index = row["other_index"].as_u64().unwrap() as usize;
        let flags = row["other_flags"].as_u64().unwrap() as u16;
        let mut model = 0;
        for i in 0..3 {
            let translation = if i == 1 {
                serde_json::from_value::<Matrix>(row["before"]["transform"].clone())
                    .unwrap()
                    .translation
            } else if i == other_index && flags != 0 {
                position("other_position")
            } else {
                Position::new(1_000_000, 1_000_000, 1_000_000)
            };
            let root = scene.add(
                None,
                Matrix {
                    translation,
                    ..Default::default()
                },
            );
            let slot = collision.insert(root).unwrap();
            if i == 1 {
                model = root;
            } else {
                slot.flags = if i == other_index { flags } else { 0 };
                slot.size = Vector3::new(4000, 4000, 4000);
                slot.owner = Some(1);
            }
        }
        match &state {
            State::Grenade(g) => g.register(model, &mut collision),
            State::Mine(m) => m.register(model, &mut collision),
            State::Smoke(s) => s.register(model, &mut collision),
            _ => unreachable!(),
        }
        .unwrap();
        collision.get_mut(model).unwrap().flags = row["initial_flags"].as_u64().unwrap() as u16;
        collision.compute(|model| scene.world(model)).unwrap();
        cues.clear();
        let mut effects = Effects::default();
        let mut emit = |cue, random: &mut Random| {
            match cue {
                Event::Splash(p) => cues.push(json!(["splash", p.to_array()])),
                Event::Sound { position, id } => {
                    cues.push(json!(["sound", position.to_array(), id]))
                }
                Event::Smoke { position, kind } => {
                    cues.push(json!(["smoke", position.to_array(), kind]))
                }
                Event::Explosion {
                    position,
                    kind,
                    contact,
                } => {
                    cues.push(json!(["explode", position.to_array(), kind]));
                    cues.push(json!([
                        "alert",
                        position.to_array(),
                        if contact { 2 } else { 3 }
                    ]));
                }
                Event::Fuse(position) => {
                    cues.push(json!(["fuse", position.to_array()]));
                    effects.fuse(position, &rules, random)?;
                }
                other => panic!("Unexpected explosive event {other:?}"),
            }
            Ok(())
        };
        let mut context = Context {
            map,
            cursor: &mut cursor,
            math: &math,
            scene: &mut scene,
            collision: &mut collision,
            player: 0,
        };
        let alive = match &mut state {
            State::Grenade(g) => g.advance(model, &mut context, &mut random, &mut emit),
            State::Mine(m) => m.advance(model, &mut context, |cue| emit(cue, &mut random)),
            State::Smoke(s) => s.advance(model, &mut context, |cue| emit(cue, &mut random)),
            _ => unreachable!(),
        }
        .unwrap_or_else(|e| panic!("case {case} {kind}: {e}"));
        if !alive {
            collision.remove(model);
        }
        assert_eq!(json!(alive), row["alive"], "alive {case} {kind}");
        assert_eq!(json!(cues), row["cues"], "events {case} {kind}");
        let after = json!(state);
        assert_eq!(after[kind], row["after"], "state {case} {kind}");
        assert_eq!(json!(random.seed), row["after_seed"], "RNG {case} {kind}");
        let slot = collision.slot(1).unwrap();
        assert_eq!(json!(slot.flags), row["flags"], "flags {case} {kind}");
        assert_eq!(
            json!(slot.size.to_array()),
            row["size"],
            "size {case} {kind}"
        );
        assert_eq!(json!(slot.owner), row["owner"], "owner {case} {kind}");
    }
    assert!(count.iter().all(|&n| n > 800));
    eprintln!("Original explosive cases: {count:?}");
}
