use super::{
    source_control::{cases, resources, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::combat::Rules;
use tenchu_core::{
    random::Random,
    tenchu2::{
        actor::{Actor, State},
        ai::{
            Decision, Probe, SteeringInput,
            combat::{Context, Dog, Fighter, Flank, GuardHome, ItemChoices, Melee, Ranged},
            sight::{Sight, Visibility},
        },
        inventory::Item,
    },
};

pub(super) fn visibility(value: i32) -> Visibility {
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
    verify("TENCHU2_COMBAT_ORACLE");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_BLADE_ORACLE from original blade-fighter decisions"]
fn native_blade_fighters_match_source() {
    verify("TENCHU2_BLADE_ORACLE");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_HEAVY_ORACLE from original heavy-weapon decisions"]
fn native_heavy_weapon_fighters_match_source() {
    verify("TENCHU2_HEAVY_ORACLE");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_UNARMED_ORACLE from original unarmed decisions"]
fn native_unarmed_fighters_match_source() {
    verify("TENCHU2_UNARMED_ORACLE");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_DOG_ORACLE from original dog decisions"]
fn native_dog_combat_matches_source() {
    verify("TENCHU2_DOG_ORACLE");
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TACTICS_ORACLE from original combat strategies"]
fn native_combat_strategies_match_source() {
    verify("TENCHU2_TACTICS_ORACLE");
}

fn fighter_state(fighter: &Fighter) -> (u8, u8, tenchu_core::math::Position) {
    match fighter {
        Fighter::Unarmed(m) | Fighter::Blade(m) | Fighter::Heavy(m) => {
            (m.attack_delay, u8::from(m.engaged), m.offset)
        }
        Fighter::Ranged(r) => (r.attack_delay, u8::from(r.thrown_item), Default::default()),
        Fighter::Dog {
            state,
            attack_delay,
        } => (*attack_delay, state.waiting, Default::default()),
    }
}

fn verify(variable: &str) {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut resources = resources();
    resources.weapons = resources.weapons.at_gameplay_rate();
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut counts = [0; 4];
    let mut items = 0;
    let mut attacks = 0;
    for (id, row) in cases::<Value>(variable).enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| row[key].as_bool().unwrap();
        let mode = row["mode"].as_str().unwrap_or(match variable {
            "TENCHU2_HEAVY_ORACLE" => "heavy",
            "TENCHU2_UNARMED_ORACLE" => "unarmed",
            "TENCHU2_DOG_ORACLE" => "dog",
            _ => "blade",
        });
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action((int("state") as i16) << 8).unwrap();
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.turn = int("turn") as i16;
        actor.rotation.y = int("yaw") as i16;
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = row["loops"].as_i64().unwrap_or(0) as i16;
        actor.attack.index = Some(int("attack") as usize);
        actor.pad.held = int("held") as u16;
        actor.inventory.selected = Some(Item::try_from(int("selected") as u8).unwrap());
        actor.inventory.counts = serde_json::from_value(row["stock"].clone()).unwrap();
        let mut target = template.clone();
        target.state = State::from_action((int("target_state") as i16) << 8).unwrap();
        target.flags = int("target_flags") as u32;
        let mut melee = Melee::default();
        if !matches!(mode, "combo" | "items" | "archer") {
            actor.motion.action = int("action") as i16;
            actor.position = vector(&row["position"]);
            actor.target = Some(1);
            actor.pad.previous = int("previous") as u16;
            actor.pad.time = int("padtime") as i16;
            actor.pad.history = serde_json::from_value(row["history"].clone()).unwrap();
            target.position = vector(&row["target_position"]);
            melee = Melee {
                attack_delay: int("delay") as u8,
                engaged: boolean("engaged"),
                offset: vector(&row["offset"]),
            };
        }
        let mut dog = Dog {
            waiting: row["waiting"].as_u64().unwrap_or(0) as u8,
        };
        let mut fighter = if matches!(mode, "fighter" | "home" | "flank") {
            let mut fighter = Fighter::new(actor.kind, &resources).unwrap();
            match &mut fighter {
                Fighter::Unarmed(m) | Fighter::Blade(m) | Fighter::Heavy(m) => *m = melee.clone(),
                Fighter::Ranged(r) => {
                    r.attack_delay = melee.attack_delay;
                    r.thrown_item = melee.engaged;
                }
                Fighter::Dog {
                    state,
                    attack_delay,
                } => {
                    state.waiting = int("phase") as u8;
                    *attack_delay = melee.attack_delay;
                }
            }
            Some(fighter)
        } else {
            None
        };
        let mut roaming = row["roaming"].as_bool().unwrap_or(false);
        let mut return_delay = row["return_delay"].as_u64().unwrap_or(0) as u8;
        let mut rotation_phase = row["rotation_phase"].as_i64().unwrap_or(0) as i16;
        let mut actions = Vec::new();
        let mut apply = |_: &mut Actor, action: i16| -> tenchu_core::Result<()> {
            actions.push(action);
            Ok(())
        };
        let mut ranged = Ranged {
            attack_delay: int("delay") as u8,
            thrown_item: row["used"].as_bool().unwrap_or(false),
        };
        let mut sight = Sight {
            distance: int("distance"),
            angle: int("angle") as i16,
            player_angle: row["bearing"].as_i64().unwrap_or(0) as i16,
            ..Default::default()
        };
        let mut visible = visibility(int("visibility"));
        let mut random = Random::new(int("seed") as u32);
        let mut context = Context {
            actor: &mut actor,
            target: Some((&target).into()),
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
                    material: row["material"].as_u64().unwrap_or(0) as u16,
                }),
                left: boolean("left"),
                right: boolean("right"),
            },
            facing: int("facing"),
            observed_flags: int("cached") as u32,
            player_held: row["player_held"].as_i64().unwrap_or(0) as u16,
            difficulty: int("difficulty") as usize,
            rules: &rules,
            resources: &resources,
            random: &mut random,
        };
        let result = match mode {
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
            "blade" => {
                counts[3] += 1;
                melee.blade(&mut context).unwrap()
            }
            "heavy" => {
                counts[3] += 1;
                melee.heavy(&mut context).unwrap()
            }
            "unarmed" => {
                counts[3] += 1;
                melee.unarmed(&mut context).unwrap()
            }
            "dog" => {
                counts[3] += 1;
                dog.update(&context).unwrap()
            }
            "fighter" => {
                counts[3] += 1;
                fighter
                    .as_mut()
                    .unwrap()
                    .update(&mut context, int("tick"), &mut rotation_phase, &mut apply)
                    .unwrap()
            }
            "home" => {
                counts[3] += 1;
                let mut home = GuardHome {
                    roaming,
                    return_delay,
                    fighter: fighter.take().unwrap(),
                };
                let result = home
                    .update(
                        &mut context,
                        vector(&row["home"]),
                        int("tick"),
                        &mut rotation_phase,
                        &mut apply,
                    )
                    .unwrap();
                roaming = home.roaming;
                return_delay = home.return_delay;
                fighter = Some(home.fighter);
                result
            }
            "flank" => {
                counts[3] += 1;
                let mut flank = Flank {
                    engaged: roaming,
                    fighter: fighter.take().unwrap(),
                };
                let result = flank.update(&mut context, &mut apply).unwrap();
                roaming = flank.engaged;
                fighter = Some(flank.fighter);
                result
            }
            "approach" => {
                counts[3] += 1;
                melee.approach_combat(&mut context, &mut apply).unwrap()
            }
            _ => panic!("Unknown combat routine"),
        };
        let facing = context.facing;
        actions.extend(result.action);
        let (delay, phase, offset) = if let Some(fighter) = &fighter {
            fighter_state(fighter)
        } else if mode == "dog" {
            (melee.attack_delay, dog.waiting, melee.offset)
        } else if matches!(mode, "archer" | "items" | "combo") {
            (
                ranged.attack_delay,
                u8::from(ranged.thrown_item),
                Default::default(),
            )
        } else {
            (melee.attack_delay, u8::from(melee.engaged), melee.offset)
        };
        let actual = json!({
            "held": result.held,
            "selected": actor.inventory.selected.map(u8::from),
            "delay": delay,
            "used": ranged.thrown_item,
            "engaged": phase != 0, "phase": phase, "waiting": dog.waiting, "offset": offset.to_array(),
            "roaming": roaming, "return_delay": return_delay, "rotation_phase": rotation_phase,
            "facing": facing, "actions": actions,
            "pad": [actor.pad.held, actor.pad.previous, actor.pad.time as u16],
            "history": actor.pad.history,
            "visibility": match visible {
                Visibility::Hidden => -2, Visibility::Unnoticed => -1, Visibility::Unchanged => 0,
                Visibility::Recognized => 1, Visibility::Suspicious => 2,
            },
            "angle": sight.angle, "yaw": actor.rotation.y,
            "forced": result.forced.map_or(int("forced") as u32, |f| u32::from(f.held) << 16 | u32::from(f.remaining as u16)),
            "seed": random.seed, "random": random.calls,
        });
        for (key, value) in row["expected"].as_object().unwrap() {
            assert_eq!(&actual[key], value, "row {id}, mode {mode}: {key}");
        }
        items += usize::from(result.held == tenchu_core::tenchu2::input::ITEM);
        attacks += usize::from(result.held & tenchu_core::tenchu2::input::ATTACK != 0);
    }
    if variable != "TENCHU2_COMBAT_ORACLE" {
        assert!(counts[3] > 30000);
    } else {
        assert!(counts[..3].iter().all(|&n| n > 10000));
    }
    assert!(attacks > 500);
    if variable != "TENCHU2_DOG_ORACLE" {
        assert!(items > 100);
    }
    eprintln!(
        "{} combat decisions: {items} items, {attacks} attacks",
        counts.iter().sum::<usize>()
    );
}
