use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    collision::Rules, commands::Commands, level::Level, math_tables, motions, registry::Registry,
    weapons::Weapons,
};
use tenchu_core::{
    math::{Math, Position},
    random::Random,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{DamageAttacker, DamageCue, DamageInput, DamageTarget, Hit, Request},
        inventory::Item,
        physics::{Depth, Terrain, Water},
        terrain::{Cursor, Options, Probe, Surface},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_DAMAGE_ORACLE from original combat code"]
fn native_weapon_projectile_damage_and_resurrection_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_DAMAGE_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted = std::collections::BTreeSet::new();
    for row in &rows {
        wanted.extend([
            row["clip"].as_i64().unwrap() as i16,
            row["expected"]["clip"].as_i64().unwrap() as i16,
        ]);
        for action in [0x916, 0xb00] {
            wanted.extend(registry.clip(row["kind"].as_i64().unwrap() as i16, action));
        }
    }
    wanted.extend(registry.clip(0, 0));
    let resources = Resources {
        clips: motions::collect(&disc, wanted)
            .unwrap()
            .into_iter()
            .map(|(id, c)| (id, c.at_gameplay_rate(registry.edition)))
            .collect(),
        registry,
        commands: Commands::load(&disc).unwrap(),
        weapons: Weapons::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        }
        .into(),
    };
    let rules = Rules::load(&disc).unwrap();
    let template = template(&resources);
    let mut map_id = usize::MAX;
    let mut map = None;
    let mut parries = 0;
    let mut deaths = 0;
    let mut revivals = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let flag = |key: &str| row[key].as_bool().unwrap();
        if map_id != int("map") as usize {
            map_id = int("map") as usize;
            map = Some(Level::load(&disc, map_id).unwrap().collision);
        }
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor.state = State::from_action(int("action") as i16).unwrap();
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.mask = 0x12345;
        actor.motion.pose.rotation[0] = [10, int("root_yaw") as i16, 20];
        actor.motion.pose.translation[0] = Position::new(111, 222, 333);
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.max_health = int("max_health") as i16;
        actor.height = int("height") as i16;
        actor.skill = int("skill") as i16;
        actor.position = vector(&row["position"]);
        actor.rotation = vector(&row["rotation"]);
        actor.velocity = vector(&row["velocity"]);
        actor.target = Some(8);
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.inventory.counts = std::array::from_fn(|i| row["counts"][i].as_u64().unwrap() as u8);
        actor.inventory.resurrections = int("resurrections") as u8;
        actor.inventory.active = Item::try_from(int("active") as u8).unwrap();
        let mut input = DamageInput {
            actor: 5,
            player: flag("player"),
            controls_enabled: flag("controls"),
            held: int("input") as u16,
            map: int("map") as i16,
            difficulty: int("difficulty") as i16,
            armor: flag("armor"),
            drain_attack: flag("drain"),
            drain_uses: 0,
            story_triggered: flag("story"),
            story_damage: 0,
            camera: int("camera") as i16,
            focus: None,
            death_music: flag("music"),
            hit: if int("hit") < 0 {
                Hit::Weapon
            } else {
                Hit::Object(int("hit") as u8)
            },
            contact: vector(&row["contact"]),
            body: vector(&row["body"]),
            contact_delta: vector(&row["delta"]),
            hit_disabled: false,
            damage_bonus: 0,
            attacker: flag("owner").then(|| DamageAttacker {
                actor: 8,
                player: flag("attacker_player"),
                position: vector(&row["attacker_position"]),
                power: int("power") as u8,
                health: int("attacker_health") as i16,
                attacking: flag("attacking"),
                recoil: 0,
            }),
        };
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut cursor = Cursor::default();
        let mut scratch = Probe::default();
        let mut water = Water {
            depth: match int("water") {
                0 => Some(Depth::Shallow),
                2 => Some(Depth::Swimming),
                3 => Some(Depth::Submerged),
                _ => None,
            },
            surface: 0,
            bottom: 0,
        };
        let mut terrain = Terrain {
            map: map.as_mut().unwrap(),
            rules: &rules,
            cursor: &mut cursor,
            scratch: &mut scratch,
            water: &mut water,
            options: Options::default(),
        };
        let mut random = Random::new(int("seed") as u32);
        let mut calls = Vec::new();
        let target = |target| match target {
            DamageTarget::Victim => "victim",
            DamageTarget::Attacker => "attacker",
        };
        actor
            .apply_damage(
                &mut request,
                &mut input,
                &mut terrain,
                &resources,
                &mut random,
                |actor, _request, input, _random, cue| {
                    match cue {
                        DamageCue::DropHeldItem => {
                            calls.push(json!(["drop", u8::from(actor.inventory.active)]))
                        }
                        DamageCue::HideHeldItem => calls.push(json!(["hide"])),
                        DamageCue::ReleaseGrip { first, second } => {
                            calls.push(json!(["grip", first, second, 1]))
                        }
                        DamageCue::FinishContacts => calls.push(json!(["contacts", 1])),
                        DamageCue::Root => {
                            calls.push(json!(["root"]));
                            actor.commit_root(vector(&row["published"]));
                        }
                        DamageCue::ContactDelta => calls.push(json!(["contact"])),
                        DamageCue::AdvanceMotion => {
                            calls.push(json!(["advance", 0]));
                            if actor.motion.loops >= 0 {
                                actor.motion.frame += 1;
                            }
                        }
                        DamageCue::Sparks { position, parried } => {
                            calls.push(json!(["sparks", position.to_array(), i32::from(!parried)]));
                            parries += usize::from(parried);
                        }
                        DamageCue::Voice { target: who, id } => {
                            calls.push(json!(["voice", target(who), id]))
                        }
                        DamageCue::Rumble {
                            target: who,
                            kind,
                            strength,
                        } => calls.push(json!(["rumble", target(who), kind, strength])),
                        DamageCue::Burn => calls.push(json!(["burn", 0x78000a])),
                        DamageCue::Sound(id) => calls.push(json!(["sound", id])),
                        DamageCue::Alert => calls.push(json!(["alert"])),
                        DamageCue::KillCredit => {
                            calls
                                .push(json!(["kill", input.attacker.as_ref().map(|_| "attacker")]));
                            deaths += 1;
                        }
                        DamageCue::Heal => {
                            calls.push(json!(["heal", 0]));
                            revivals += 1;
                        }
                        DamageCue::Resurrection => {
                            calls.push(json!([
                                "revive",
                                input.attacker.as_ref().map(|_| "attacker")
                            ]));
                            revivals += 1;
                        }
                        DamageCue::ShowHealth => calls.push(json!(["health"])),
                        DamageCue::DeathMusic => calls.push(json!(["music", actor.kind * 2 + 14])),
                        DamageCue::Blood { joint, angle } => {
                            calls.push(json!(["point", joint]));
                            calls.push(json!(["blood", row["point"], angle]));
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();
        let surface = |value: Option<Surface>| value.map(|s| [s.cell, s.node]);
        let actual = json!({"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"health":actor.health,"flags":actor.flags,"position":actor.position.to_array(),"velocity":actor.velocity.to_array(),"rotation":actor.rotation.to_array(),"root_rotation":actor.motion.pose.rotation[0],"mask":actor.motion.mask,"motion":actor.motion.action,"clip":actor.motion.clip,"frame":actor.motion.frame,"loops":actor.motion.loops,"state":actor.state as u8,"seed":random.seed,"random":random.calls,"target":actor.target,"focus":input.focus,"camera":input.camera,"hit_disabled":input.hit_disabled,"attacker_health":input.attacker.as_ref().map_or(int("attacker_health") as i16, |a| a.health),"attacker_recoil":input.attacker.as_ref().map_or(0, |a| a.recoil),"counts":actor.inventory.counts,"resurrections":actor.inventory.resurrections,"story":input.story_triggered,"story_damage":input.story_damage,"drain_uses":input.drain_uses,"current":surface(terrain.cursor.current),"last":surface(terrain.cursor.last),"material":terrain.cursor.material,"index":terrain.cursor.index,"calls":calls});
        assert_eq!(actual, row["expected"], "case {id}: {row}");
    }
    assert_eq!(rows.len(), 41_400);
    assert!(parries > 100 && deaths > 100 && revivals > 100);
    eprintln!(
        "41400 damage source updates; {parries} parries, {deaths} lethal hits, {revivals} resurrections"
    );
}
