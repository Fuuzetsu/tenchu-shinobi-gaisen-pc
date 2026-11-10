use serde::Deserialize;
use tenchu_assets::tenchu2::{layout::Layout, loot::Loot};
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{
        inventory::Inventory,
        world::{Assets, Options, World},
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ACTOR_TAIL_ORACLE from the Japanese actor update"]
fn native_actor_visibility_feet_head_tracking_and_effort_match_source() {
    use serde_json::Value;
    use tenchu_assets::{
        tenchu2::collision::{Cell, Collision},
        terrain::Node,
    };
    use tenchu_core::tenchu2::actor::State;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layout = Layout::available(&disc).unwrap().remove(0);
    let assets = Assets::load(&disc, 0, &layout).unwrap();
    let mut template = World::new(
        &layout,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 1,
            inventory: Inventory::default(),
        },
        &assets,
    )
    .unwrap();
    assert!(template.actors.len() >= 2);
    template.actors.truncate(2);
    template.instances.truncate(2);
    template.terrain = Collision {
        gaisen: None,
        origin: [-3000, -3000],
        dimensions: [1, 1],
        cell_size: 6000,
        cells: vec![Cell {
            minimum: i16::MIN,
            maximum: i16::MAX,
            starts: [0; 4],
            nodes: vec![Node {
                y: 0,
                dy: i16::MAX,
                bounds: [-3000, -3000, 3000, 3000],
                attributes: 0,
                division: u16::MAX,
            }],
        }],
    };
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_ACTOR_TAIL_ORACLE") {
        let mut world = template.clone();
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let flag = |key: &str| row[key].as_bool().unwrap();
        let actor = &mut world.actors[0].actor;
        actor.kind = int("kind") as i16;
        actor.state = State::from_action(int("action") as i16).unwrap();
        actor.motion.action = int("action") as i16;
        actor.motion.loops = -1;
        actor.flags = int("flags") as u32;
        actor.health = int("health") as i16;
        actor.position = super::source_physics::vector(&row["position"]);
        actor.rotation.y = int("yaw") as i16;
        actor.motion.pose.translation.fill(Position::default());
        actor.motion.pose.rotation.fill([0; 3]);
        actor.motion.pose.rotation[0][1] = int("root_yaw") as i16;
        actor.motion.pose.rotation[1][1] = int("torso_yaw") as i16;
        actor.motion.pose.rotation[2] = serde_json::from_value(row["head"].clone()).unwrap();
        actor.velocity = super::source_physics::vector(&row["velocity"]);
        actor.terrain.material = int("material") as u16;
        actor.target = flag("target").then_some(1);
        let target = &mut world.actors[1].actor;
        target.motion.loops = if flag("target_dead") { i16::MIN } else { 0 };
        target.flags = if flag("target_visible") { 0x100000 } else { 0 };
        target.position = super::source_physics::vector(&row["target_position"]);
        world.player = usize::from(!flag("player"));
        world.tracking = flag("tracked").then_some(1);
        world.water.bottom = int("water");
        world.effort = int("effort") as u16;
        world.travel = int("travel");
        world.publish(0, &assets);
        world.camera.view = tenchu_core::math::Matrix {
            rotation: [[0; 3]; 3],
            translation: Position::new(0, 0, if flag("visible") { 1000 } else { 0 }),
        };
        world.animate_actor(0, &assets).unwrap();
        let expected = &row["expected"];
        assert_eq!(
            world.actors[0].actor.flags,
            expected["flags"].as_u64().unwrap() as u32,
            "case {count}"
        );
        assert_eq!(
            serde_json::to_value(world.actors[0].actor.motion.pose.rotation[2]).unwrap(),
            expected["head"],
            "case {count}"
        );
        assert_eq!(
            world.travel,
            expected["travel"].as_i64().unwrap() as i32,
            "case {count}"
        );
        assert_eq!(
            world.effort,
            expected["effort"].as_u64().unwrap() as u16,
            "case {count}"
        );
        assert_eq!(
            world.tracking.is_some(),
            expected["tracked"].as_bool().unwrap(),
            "case {count}"
        );
        let feet: Vec<_> = world.instances[0]
            .feet
            .iter()
            .flat_map(|foot| {
                [
                    foot.position.x,
                    foot.position.y,
                    foot.position.z,
                    foot.floor.unwrap_or(i32::MIN),
                ]
            })
            .collect();
        assert_eq!(
            serde_json::to_value(feet).unwrap(),
            expected["foot_points"],
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 10_000);
}

#[derive(Deserialize)]
struct LootCase {
    kind: i16,
    seed: u32,
    inventory: [u8; 20],
    end_seed: u32,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_LOOT_ORACLE from the Japanese executable"]
fn native_actor_inventories_match_source_rng_and_item_slots() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let loot = Loot::load(&disc).unwrap();
    let mut count = 0;
    for case in super::source_control::cases::<LootCase>("TENCHU2_LOOT_ORACLE") {
        let mut random = Random::new(case.seed);
        let inventory = Inventory::for_actor(case.kind, &loot, &mut random);
        assert_eq!(
            inventory.counts, case.inventory,
            "kind {} / seed {}",
            case.kind, case.seed
        );
        assert_eq!(
            random.seed, case.end_seed,
            "kind {} / seed {}",
            case.kind, case.seed
        );
        count += 1;
    }
    assert_eq!(count, 13_000);
}

#[test]
#[ignore = "requires TENCHU2_DISC pointing at the Japanese disc"]
fn native_layouts_create_the_source_cast_and_live_weapon_rigs() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let mut worlds = 0;
    for hero in 0..3 {
        for map in 0..24 {
            let selected: Vec<_> = layouts
                .iter()
                .filter(|layout| {
                    layout.map == map
                        && match hero {
                            0 => layout.sequence < 101,
                            1 => (101..185).contains(&layout.sequence),
                            _ => layout.sequence >= 185,
                        }
                })
                .collect();
            if selected.is_empty() {
                continue;
            }
            for layout in selected {
                let assets = Assets::load(&disc, hero, layout)
                    .unwrap_or_else(|error| panic!("hero {hero} map {map}: {error}"));
                let mut world = World::new(
                    layout,
                    Options {
                        hero,
                        difficulty: 2,
                        seed: 1234,
                        inventory: Inventory::default(),
                    },
                    &assets,
                )
                .unwrap_or_else(|error| panic!("sequence {}: {error}", layout.sequence));
                let cast = assets.resources.registry.cast(hero, map).unwrap();
                let entries: Vec<_> = layout
                    .actors
                    .iter()
                    .filter(|actor| cast.contains(&actor.kind))
                    .collect();
                assert_eq!(world.actors.len(), entries.len() + 1);
                for (agent, source) in world.actors[1..].iter().zip(&entries) {
                    assert_eq!(agent.actor.kind, source.kind);
                    assert_eq!(agent.slot as usize, source.slot);
                    assert_eq!(agent.actor.position, Position::from(source.position));
                    assert_eq!(agent.actor.rotation.y, source.facing);
                    assert_eq!(agent.behavior.home, agent.actor.position);
                }
                for (agent, instance) in world.actors.iter().zip(&world.instances) {
                    assert_eq!(
                        instance.rig.joints.len(),
                        agent.actor.motion.pose.rotation.len()
                    );
                    let body = world.collision.get_mut(instance.rig.joints[0]);
                    assert_eq!(body.is_some(), agent.actor.flags & 8 == 0);
                    if let Some(body) = body {
                        assert_eq!(body.size.x, agent.actor.width / 2);
                    }
                    for root in instance.rig.weapons.iter().flatten() {
                        world.scene.world(*root).unwrap();
                    }
                }
                world
                    .collision
                    .compute(|model| world.scene.world(model))
                    .unwrap();
                let restored: World =
                    serde_json::from_slice(&serde_json::to_vec(&world).unwrap()).unwrap();
                assert_eq!(
                    serde_json::to_value(&restored).unwrap(),
                    serde_json::to_value(&world).unwrap()
                );
                eprintln!(
                    "JP sequence {}: {} native actors",
                    layout.sequence,
                    world.actors.len()
                );
                worlds += 1;
            }
        }
    }
    assert_eq!(worlds, 56);
}

#[test]
#[ignore = "requires TENCHU2_DISC; exercises connected JP player controls and snapshot continuation"]
fn native_player_controls_continue_through_movement_drawing_and_combat() {
    use tenchu_core::{
        Result,
        tenchu2::{
            input::{ATTACK, CROUCH, JUMP, RIGHT, UP},
            world::controls::{Event, EventSource, Events},
        },
    };
    #[derive(Default)]
    struct Recorded(
        Vec<(usize, i16, i16, Event)>,
        Vec<tenchu_core::tenchu2::environment::Event>,
    );
    impl tenchu_core::tenchu2::environment::Events for Recorded {
        fn emit(
            &mut self,
            event: tenchu_core::tenchu2::environment::Event,
            _: &mut Random,
        ) -> Result<()> {
            self.1.push(event);
            Ok(())
        }
    }
    impl Events for Recorded {
        fn emit(&mut self, source: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
            self.0.push((
                source.index,
                source.actor.motion.action,
                source.actor.motion.frame,
                event,
            ));
            Ok(())
        }
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let inputs: Vec<_> = [
        (UP, 10),
        (0, 20),
        (CROUCH, 10),
        (CROUCH | UP, 10),
        (0, 20),
        (RIGHT, 10),
        (JUMP, 1),
        (0, 90),
        (ATTACK, 1),
        (0, 90),
        (ATTACK, 1),
        (0, 90),
        (ATTACK, 1),
        (0, 90),
    ]
    .into_iter()
    .flat_map(|(held, count)| std::iter::repeat_n(held, count))
    .collect();
    let mut states = std::collections::BTreeSet::new();
    for hero in 0..3 {
        let layout = layouts
            .iter()
            .find(|l| match hero {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap();
        let mut assets = Assets::load(&disc, hero, layout).unwrap();
        let mut world = World::new(
            layout,
            Options {
                hero,
                difficulty: 2,
                seed: 1,
                inventory: Inventory::default(),
            },
            &assets,
        )
        .unwrap();
        for instance in world.instances.iter().skip(1) {
            for &joint in &instance.rig.joints {
                world.collision.remove(joint);
            }
        }
        world.actors.truncate(1);
        world.instances.truncate(1);
        world.finish_tick(&assets, 6).unwrap();
        let start = world.actors[0].actor.position;
        let mut events = Recorded::default();
        let step = |world: &mut World, assets: &mut Assets, events: &mut Recorded, held| {
            world.update_camera(assets, false).unwrap();
            world.move_actor(0, assets, events).unwrap();
            world.actors[0].actor.pad.update(held);
            world
                .control_actor(0, assets, events)
                .unwrap_or_else(|e| panic!("hero {hero} tick {}: {e}", world.tick));
            world.animate_actor(0, assets).unwrap();
            world.update_items(assets, events).unwrap();
            world.update_effects(assets).unwrap();
            world.finish_tick(assets, 6).unwrap();
        };
        for &held in &inputs[..180] {
            step(&mut world, &mut assets, &mut events, held);
            states.insert(world.actors[0].actor.motion.action >> 8);
        }
        assert_ne!(world.actors[0].actor.position, start);
        let saved = serde_json::to_vec(&world).unwrap();
        let mut resumed: World = serde_json::from_slice(&saved).unwrap();
        let mut resumed_assets = Assets::load(&disc, hero, layout).unwrap();
        let mut resumed_events = Recorded::default();
        events.0.clear();
        events.1.clear();
        for &held in &inputs[180..] {
            step(&mut world, &mut assets, &mut events, held);
            step(&mut resumed, &mut resumed_assets, &mut resumed_events, held);
            states.insert(world.actors[0].actor.motion.action >> 8);
        }
        assert_eq!(
            serde_json::to_value(&world).unwrap(),
            serde_json::to_value(&resumed).unwrap()
        );
        assert_eq!(events.0, resumed_events.0);
        assert_eq!(events.1, resumed_events.1);
        assert!(world.actors[0].actor.weapon_drawn());
        assert!(
            events
                .0
                .iter()
                .any(|(_, _, _, event)| matches!(event, Event::Combat(_)))
        );
    }
    for state in [0, 1, 2, 6, 7, 9] {
        assert!(states.contains(&state), "unvisited state {state}");
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_ACTIVATION_ORACLE from JP 8001e318"]
fn native_activation_matches_source_actor_order_and_camera_boundaries() {
    use serde_json::{Value, json};
    use tenchu_core::tenchu2::actor::Velocity;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layout = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|layout| layout.sequence == 9)
        .unwrap();
    let assets = Assets::load(&disc, 0, &layout).unwrap();
    let mut template = World::new(
        &layout,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 1,
            inventory: Inventory::default(),
        },
        &assets,
    )
    .unwrap();
    assert!(template.actors.len() >= 12);
    template.actors.truncate(12);
    template.instances.truncate(12);
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_ACTIVATION_ORACLE") {
        let mut world = template.clone();
        world.collision = Default::default();
        world.view_distance = row["view"].as_i64().unwrap() as i32;
        world.controls.camera_actor = row["focus"].as_u64().unwrap() as usize;
        world.tick = row["tick"].as_u64().unwrap() as u32;
        for (index, input) in row["actors"].as_array().unwrap().iter().enumerate() {
            let agent = &mut world.actors[index];
            let actor = &mut agent.actor;
            actor.kind = 0;
            actor
                .change_motion(
                    input["action"].as_i64().unwrap() as i16,
                    Velocity::Stop,
                    &assets.resources,
                )
                .unwrap();
            actor.kind = input["kind"].as_i64().unwrap() as i16;
            actor.flags = input["flags"].as_u64().unwrap() as u32;
            actor.health = input["health"].as_i64().unwrap() as i16;
            actor.position = super::source_physics::vector(&input["position"]);
            actor.previous = actor.position;
            actor.motion.loops = input["loops"].as_i64().unwrap() as i16;
            agent.behavior.home = super::source_physics::vector(&input["home"]);
            agent.behavior.patrol.turning = input["turn"].as_i64().unwrap() as u8;
            agent.behavior.patrol.next = input["next"].as_i64().unwrap() as usize;
            actor
                .set_body_collision(
                    world.instances[index].rig.joints[0],
                    index,
                    &mut world.collision,
                    actor.flags & 8 == 0,
                )
                .unwrap();
        }
        world
            .update_activation(
                super::source_physics::vector(&row["eye"]),
                super::source_physics::vector(&row["look"]),
                row["budget"].as_u64().unwrap() as u8,
                &assets,
            )
            .unwrap();
        for (index, expected) in row["expected"].as_array().unwrap().iter().enumerate() {
            let agent = &world.actors[index];
            let actor = &agent.actor;
            let mut actual = json!({
                "flags": actor.flags, "position": actor.position.to_array(),
                "previous": actor.previous.to_array(), "home": agent.behavior.home.to_array(),
                "action": actor.motion.action, "turn": agent.behavior.patrol.turning,
                "next": agent.behavior.patrol.next, "body": null,
            });
            if !expected["body"].is_null() {
                actual["body"] = json!(u8::from(
                    world
                        .collision
                        .index(world.instances[index].rig.joints[0])
                        .is_some()
                ));
            }
            assert_eq!(actual, *expected, "case {count}, actor {index}");
        }
        count += 1;
    }
    assert_eq!(count, 1000);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; checks alternate scenery, interactive objects and trap links"]
fn japanese_scene_variants_keep_all_object_records() {
    use tenchu_assets::tenchu2::level::{
        Level,
        objects::{StaticKind, Variant},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut visited = std::collections::BTreeSet::new();
    let mut dynamic = 0;
    let mut effects = 0;
    let mut triggers = 0;
    for layout in Layout::available(&disc).unwrap() {
        for hidden in [false, true] {
            let key = (
                layout.map,
                format!(
                    "{:?}",
                    Variant::japanese(layout.map, layout.sequence, hidden)
                ),
            );
            if !visited.insert(key) {
                continue;
            }
            let level = Level::japanese_sequence(&disc, &layout, hidden)
                .unwrap_or_else(|e| panic!("sequence {} hidden {hidden}: {e}", layout.sequence));
            let scene = &level.scene;
            assert_eq!(scene.bounds.len(), level.objects.len());
            dynamic += scene.dynamic.len();
            effects += scene
                .statics
                .iter()
                .filter(|p| matches!(p.kind, StaticKind::Effect(_)))
                .count();
            triggers += scene.triggers.len();
            for object in &scene.dynamic {
                assert!(!level.objects[object.model.unwrap()].geometry.is_empty());
                if let Some(trigger) = object.trigger {
                    assert!(matches!(scene.triggers[trigger].kind, 0..=5));
                }
            }
        }
    }
    assert!(visited.len() > 35);
    assert!(dynamic > 1500 && triggers > 200 && effects > 100);
    eprintln!(
        "{} JP scenery variants: {dynamic} interactive objects, {triggers} triggers, {effects} static effects",
        visited.len()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; loads every area's background, including JP's alternate story palettes"]
fn all_area_background_resources_load() {
    use tenchu_assets::tenchu2::{Edition, campaign::Campaign, level::Level, story::Script};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut skies = 0;
    let mut landscapes = 0;
    for area in Level::available(&disc).unwrap() {
        let level = Level::load(&disc, area).unwrap_or_else(|e| panic!("area {area}: {e}"));
        skies += level.background.sky.len();
        landscapes += usize::from(level.background.landscape.is_some());
    }
    assert!(skies > 20 && landscapes > 5);
    if Edition::detect(&disc).unwrap() == Edition::Japan {
        let campaign = Campaign::load(&disc).unwrap();
        for sequence in [16, 17, 26, 117, 186] {
            let script = Script::load(&disc, campaign.sequences[sequence].resource).unwrap();
            let area = script.area.unwrap();
            let base = Level::load(&disc, area).unwrap();
            let alternate = Level::japanese_scene(&disc, area, sequence, true).unwrap();
            assert!(!alternate.background.sky.is_empty());
            assert_ne!(base.vram.words, alternate.vram.words, "sequence {sequence}");
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; checks the alternate boss-arena collision resources"]
fn japanese_boss_arenas_use_their_sequence_collision() {
    use tenchu_assets::tenchu2::level::Level;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, count) in [(179, 166), (216, 340), (245, 159)] {
        let layout = Layout::load(&disc, sequence).unwrap();
        for hidden in [false, true] {
            let level = Level::japanese_sequence(&disc, &layout, hidden).unwrap();
            assert_eq!(
                level
                    .collision
                    .cells
                    .iter()
                    .map(|c| c.nodes.len())
                    .sum::<usize>(),
                count,
                "sequence {sequence}, hidden {hidden}"
            );
        }
        let base = Level::load(&disc, layout.map).unwrap();
        assert!(
            base.collision
                .cells
                .iter()
                .map(|c| c.nodes.len())
                .sum::<usize>()
                > 2800
        );
    }
}
