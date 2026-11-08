use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::{Resources, State, Velocity},
        control::{AttackCue, Projectile, Request},
        rig::Grip,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SEQUENCE_ORACLE from the original attack timelines"]
fn native_attack_sequences_preserve_combos_grips_projectiles_and_landings() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let resources = Resources {
        clips: motions::collect(&disc, registry.clip(0, 0).into_iter().collect()).unwrap(),
        registry,
        weapons: Weapons::load(&disc).unwrap().at_gameplay_rate(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        }
        .into(),
    };
    let template = template(&resources);
    let mut checked = 0;
    let mut clips = std::collections::BTreeSet::new();
    for (index, row) in cases::<Value>("TENCHU2_SEQUENCE_ORACLE").enumerate() {
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.state = State::Attack;
        actor.motion.action = int("action") as i16;
        actor.motion.clip = Some(int("clip") as i16);
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.height = 1800;
        actor.rotation.y = int("yaw") as i16;
        actor.velocity = vector(&row["velocity"]);
        actor.pad.previous = int("previous") as u16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.attack.index = (int("attack") as usize != resources.weapons.attacks.len())
            .then_some(int("attack") as usize);
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut calls = Vec::new();
        let mut arrow_slot = 0x12345678u32;
        let flow = actor
            .attack_sequence(
                &mut request,
                int("input") as u16,
                &resources,
                |actor, _, cue| {
                    match cue {
                        AttackCue::Grip { slot, spare, grip } => calls.push(json!([
                            "grip",
                            slot,
                            spare,
                            match grip {
                                Grip::Swap => 0,
                                Grip::Release => 1,
                                Grip::Hold => 2,
                            }
                        ])),
                        AttackCue::NockArrow => {
                            arrow_slot = 0x12345690;
                            calls.push(json!(["voice", 3]));
                        }
                        AttackCue::Fire(Projectile::Arrow) => {
                            arrow_slot = 0;
                            calls.extend([json!(["voice", 2]), json!(["arrow"])]);
                        }
                        AttackCue::Fire(Projectile::Gunshot) => {
                            arrow_slot = 0;
                            calls.push(json!(["gun"]));
                        }
                        AttackCue::Fire(Projectile::ForwardGunshot) => calls.push(json!(["gun"])),
                        AttackCue::Fire(Projectile::Ice | Projectile::IceFan) => {
                            for _ in 0..if cue == AttackCue::Fire(Projectile::IceFan) {
                                4
                            } else {
                                1
                            } {
                                calls.push(json!(["ice"]));
                            }
                            calls.push(json!(["voice", 0x21]));
                        }
                        AttackCue::Land => {
                            calls.push(json!(["rumble", 2, 1]));
                            if actor.terrain.material & 0xf00 == 0x200 {
                                calls.extend([json!(["water", 0x400]), json!(["voice", 0x1c])]);
                                if row["player"].as_bool().unwrap() {
                                    calls.push(json!(["alert", 1]));
                                }
                            } else {
                                calls.extend([json!(["dust", 0]), json!(["voice", 0x17])]);
                            }
                        }
                        AttackCue::FallDead => calls.push(json!(["special"])),
                    }
                    Ok(())
                },
            )
            .unwrap();
        let actual = json!({"action":request.action,"mode":match request.velocity { Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1 },"motion":actor.motion.action,"loops":actor.motion.loops,"yaw":actor.rotation.y,"root_yaw":actor.motion.pose.rotation[0][1],"velocity":actor.velocity.to_array(),"arrow_slot":arrow_slot,"calls":calls,"finished":flow.is_break()});
        assert_eq!(
            actual,
            row["expected"],
            "case {index}: action {}, clip {}, frame {}",
            int("action"),
            int("clip"),
            int("frame")
        );
        clips.insert(int("clip"));
        checked += 1;
    }
    assert!(clips.len() >= 220);
    assert!(checked >= 30_000);
}
