use super::source_control::{cases, resources, template};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::combat::Rules;
use tenchu_core::{
    random::Random,
    tenchu2::{
        actor::State,
        ai::{
            Decision, Probe, SteeringInput,
            combat::{Context, ItemChoices, Ranged},
            sight::{Sight, Visibility},
        },
        inventory::Item,
    },
};

fn visibility(value: i32) -> Visibility {
    match value {
        -2 => Visibility::Hidden,
        -1 => Visibility::Unnoticed,
        0 => Visibility::Unchanged,
        1 => Visibility::Recognized,
        2 => Visibility::Suspicious,
        _ => panic!("Invalid source visibility"),
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_COMBAT_ORACLE from original combat decisions"]
fn native_combos_items_and_ranged_combat_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = resources();
    resources.weapons = resources.weapons.at_gameplay_rate();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut counts = [0; 3];
    let mut items = 0;
    let mut attacks = 0;
    for (id, row) in cases::<Value>("TENCHU2_COMBAT_ORACLE").enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.attack.index = Some(int("attack") as usize);
        actor.pad.held = int("held") as u16;
        actor.inventory.selected = Some(Item::try_from(int("selected") as u8).unwrap());
        actor.inventory.counts = serde_json::from_value(row["stock"].clone()).unwrap();
        let mut target = template.clone();
        target.state = State::from_action((int("target_state") as i16) << 8).unwrap();
        target.flags = int("target_flags") as u32;
        let mut ranged = Ranged {
            attack_delay: int("delay") as u8,
            thrown_item: boolean("used"),
        };
        let mut sight = Sight {
            distance: int("distance"),
            angle: int("angle") as i16,
            ..Default::default()
        };
        let mut visible = visibility(int("visibility"));
        let mut random = Random::new(int("seed") as u32);
        let mut context = Context {
            actor: &mut actor,
            target: &target,
            sight: &mut sight,
            visibility: &mut visible,
            steering: SteeringInput {
                yaw: int("yaw") as i16,
                turn: int("turn") as i16,
                target_angle: int("angle") as i16,
                alerted: int("cached") & 3 != 0,
                blocked: int("cached") & 0x48000 != 0,
                forced: int("forced") != 0,
                ahead: (int("height") != i32::MIN).then_some(Probe {
                    height: int("height"),
                    material: 0,
                }),
                left: boolean("left"),
                right: boolean("right"),
            },
            facing: int("facing"),
            difficulty: int("difficulty") as usize,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let result = match row["mode"].as_str().unwrap() {
            "combo" => {
                counts[0] += 1;
                Decision {
                    held: context.combo(int("reach"), int("cone") as i16).unwrap(),
                    ..Default::default()
                }
            }
            "items" => {
                counts[1] += 1;
                Decision {
                    held: context
                        .choose_item(ItemChoices {
                            defensive: int("choices") & 1 != 0,
                            offensive: int("choices") & 2 != 0,
                        })
                        .unwrap(),
                    ..Default::default()
                }
            }
            "archer" => {
                counts[2] += 1;
                ranged.update(&mut context).unwrap()
            }
            _ => panic!("Unknown combat routine"),
        };
        let actual = json!({
            "held": result.held,
            "selected": actor.inventory.selected.map(u8::from),
            "delay": ranged.attack_delay, "used": ranged.thrown_item,
            "angle": sight.angle, "yaw": actor.rotation.y,
            "forced": result.forced.map_or(int("forced") as u32, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
            "seed": random.seed, "random": random.calls,
        });
        for (key, value) in actual.as_object().unwrap() {
            assert_eq!(
                value, &row["expected"][key],
                "row {id}, mode {}: {key}",
                row["mode"]
            );
        }
        assert_eq!(
            visible,
            visibility(row["expected"]["visibility"].as_i64().unwrap() as i32),
            "row {id}"
        );
        items += usize::from(result.held == tenchu_core::tenchu2::input::ITEM);
        attacks += usize::from(result.held & tenchu_core::tenchu2::input::ATTACK != 0);
    }
    assert!(counts.iter().all(|&n| n > 10000));
    assert!(items > 500 && attacks > 500);
    eprintln!(
        "{} combat decisions: {items} items, {attacks} attacks",
        counts.iter().sum::<usize>()
    );
}
