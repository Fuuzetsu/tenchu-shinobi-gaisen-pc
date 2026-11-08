use super::{
    source_control::{cases, template},
    source_physics::vector,
};
use serde_json::{Value, json};
use tenchu_assets::tenchu2::{
    commands::Commands, math_tables, motions, registry::Registry, weapons::Weapons,
};
use tenchu_core::{
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::Scene,
    tenchu2::{
        actor::{Resources, State, Velocity},
        collision::Collision,
        control::{AttackContext, CombatCue, Request},
        rig::Rig,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ATTACK_ORACLE from original enclosing attack controllers"]
fn native_attack_contacts_parries_steering_and_recovery_match_source() {
    let rows: Vec<Value> = cases("TENCHU2_ATTACK_ORACLE").collect();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let mut wanted: std::collections::BTreeSet<_> = rows
        .iter()
        .flat_map(|row| {
            [
                row["clip"].as_i64().unwrap() as i16,
                row["expected"]["clip"].as_i64().unwrap() as i16,
            ]
        })
        .collect();
    wanted.extend(registry.clip(0, 0));
    let resources = Resources {
        clips: motions::collect(&disc, wanted)
            .unwrap()
            .into_iter()
            .map(|(id, clip)| (id, clip.at_gameplay_rate(registry.edition)))
            .collect(),
        registry,
        weapons: Weapons::load(&disc).unwrap().at_gameplay_rate(),
        commands: Commands::load(&disc).unwrap(),
        math: Math {
            tables: math_tables(&disc).unwrap(),
        },
    };
    let template = template(&resources);
    let mut seen = std::collections::BTreeSet::new();
    let mut parries = 0;
    for (id, row) in rows.iter().enumerate() {
        let int = |name: &str| row[name].as_i64().unwrap() as i32;
        let mut actor = template.clone();
        actor.kind = int("kind") as i16;
        actor
            .motion
            .select(
                int("action") as i16,
                actor.kind,
                &resources.registry,
                &resources.clips,
            )
            .unwrap()
            .unwrap();
        actor.state = State::Attack;
        actor.motion.frame = int("frame") as i16;
        actor.motion.loops = int("loops") as i16;
        actor.motion.pose.rotation[0][1] = int("root_yaw") as i16;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.skill = int("skill") as i16;
        actor.height = 1800;
        actor.turn = int("turn") as i16;
        actor.terrain.height = int("ground");
        actor.terrain.material = int("material") as u16;
        actor.rotation.y = int("yaw") as i16;
        actor.position = Position::new(0, -1000, 0);
        actor.velocity = vector(&row["velocity"]);
        actor.attack.index = (int("attack") as usize != resources.weapons.attacks.len())
            .then_some(int("attack") as usize);
        actor.attack.trails = int("trails") as u32;
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut scene = Scene::default();
        let rig = Rig::bind(&actor, &mut scene, &resources.math);
        let mut collision = Collision::default();
        let models: Vec<_> = (0..8)
            .map(|i| if i < 4 { rig.joints[i] } else { 100 + i })
            .collect();
        for (index, slot) in row["collision"].as_array().unwrap().iter().enumerate() {
            let entry = collision.insert(models[index]).unwrap();
            entry.owner = Some(slot["owner"].as_u64().unwrap() as usize);
            entry.flags = slot["flags"].as_u64().unwrap() as u16;
            entry.position = vector(&slot["position"]);
            entry.size = Vector3::new(60, 60, 60);
        }
        collision
            .compute(|joint| {
                let index = models.iter().position(|&id| id == joint).unwrap();
                Ok(Matrix {
                    translation: vector(&row["collision"][index]["position"]),
                    ..Matrix::default()
                })
            })
            .unwrap();
        let mut request = Request {
            action: actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut calls = Vec::new();
        let mut others = [0; 4];
        let player = row["player"].as_bool().unwrap();
        let preparation = actor
            .prepare_attack(
                &mut request,
                &resources,
                &mut random,
                player,
                row["enabled"].as_bool().unwrap(),
            )
            .unwrap();
        let status = preparation.status;
        if status.died {
            calls.push(json!(["status", 0]));
            if player {
                calls.extend([json!(["status", 1]), json!(["status", 2])]);
            } else {
                calls.push(json!(["voice", 15]));
            }
        }
        if status.show_health {
            calls.push(json!(["status", 3]));
        }
        if status.staggered {
            calls.push(json!(["rumble", 4, 1]));
        }
        if !preparation.interrupted {
            actor
                .steer_attack(
                    request,
                    (!row["target"].is_null()).then(|| vector(&row["target"])),
                    &resources,
                )
                .unwrap();
            request.action = int("next") as i16;
            actor
                .finish_attack(
                    &mut request,
                    int("input") as u16,
                    &resources,
                    &mut AttackContext {
                        rig: &rig,
                        scene: &mut scene,
                        collision: &mut collision,
                        random: &mut random,
                    },
                    |_, _, cue| {
                        calls.push(match cue {
                            CombatCue::Voice(id) => json!(["voice", id]),
                            CombatCue::Rumble { kind, strength } => {
                                json!(["rumble", kind, strength])
                            }
                            CombatCue::EndTrail(slot) => json!(["end", slot]),
                            CombatCue::Trail { slot, part } => json!(["trail", slot, part]),
                            CombatCue::Clash { opponent, position } => {
                                others[opponent - 1] = -5;
                                parries += 1;
                                json!(["clash", opponent, position.to_array()])
                            }
                        });
                        Ok(())
                    },
                )
                .unwrap();
        }
        let actual = json!({"flags":actor.flags,"health":actor.health,"action":request.action,"mode":match request.velocity {Velocity::Stop=>-1,Velocity::Keep=>0,Velocity::Motion=>1},"motion":actor.motion.action,"state":(actor.motion.action>>8),"frame":actor.motion.frame,"loops":actor.motion.loops,"clip":actor.motion.clip,"attack":actor.attack.index.unwrap_or(resources.weapons.attacks.len()),"yaw":actor.rotation.y,"root_yaw":actor.motion.pose.rotation[0][1],"velocity":actor.velocity.to_array(),"mask":actor.motion.mask,"trails":actor.attack.trails,"collision":collision.slots().iter().map(|s|s.flags).collect::<Vec<_>>(),"others":others,"seed":random.seed,"random":random.calls,"interrupted":preparation.interrupted,"retarget":preparation.retarget,"calls":calls});
        let mut expected = row["expected"].clone();
        expected.as_object_mut().unwrap().remove("roots");
        assert_eq!(
            actual,
            expected,
            "case {id}, kind {}, action {}, frame {}",
            actor.kind,
            int("action"),
            int("frame")
        );
        seen.insert(actor.kind);
    }
    assert!(seen.len() >= 98);
    assert!(parries > 100);
    eprintln!(
        "{} native attack updates over {} actor definitions, {parries} parries match source",
        rows.len(),
        seen.len()
    );
}
