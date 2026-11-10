use super::*;
use tenchu_core::tenchu2::{
    actor::State as ActorState,
    hud::{Image, Primitive},
    inventory::Item,
    items::State,
};

#[derive(Default)]
struct Recorded(Vec<Event>);

impl environment::Events for Recorded {
    fn emit(&mut self, _: environment::Event, _: &mut Random) -> Result<()> {
        Ok(())
    }
}

impl Events for Recorded {
    fn emit(&mut self, _: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.0.push(event);
        Ok(())
    }
}

#[test]
#[ignore = "requires both discs; compares every layout's pickups with the original T1 world and advances all native hero/layout starts"]
fn gaisen_pickups_preserve_all_layout_placements() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let executable = tenchu_assets::executable::Executable::new(&game.executable).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    for stage in 0..11 {
        for layout in 1..=3 {
            for hero in 0..3 {
                let character = i16::from(hero == 1);
                let source = MissionAssets::load(&game, stage, character, layout).unwrap();
                let (mut assets, native_layout) =
                    Assets::gaisen(&disc, &source, hero, false).unwrap();
                let original =
                    tenchu_core::game::Game::new(source, &executable, character, 1).unwrap();
                let expected: Vec<_> = original
                    .items
                    .slots
                    .iter()
                    .filter(|item| {
                        matches!(item.behavior, tenchu_core::items::Behavior::Pickup { .. })
                    })
                    .map(|item| {
                        let kind = match item.kind {
                            tenchu_core::items::Kind::Medicine => 1,
                            tenchu_core::items::Kind::Grenade => 6,
                            tenchu_core::items::Kind::SmokeBomb => 10,
                            other => panic!("unmapped pickup {other:?}"),
                        };
                        (kind, original.scene.models[item.model].local.translation)
                    })
                    .collect();
                assert_eq!(
                    assets.gaisen_pickups, expected,
                    "stage {stage} layout {layout} hero {hero}"
                );
                let mut world = World::new(
                    &native_layout,
                    Options {
                        hero,
                        difficulty: 2,
                        seed: 1,
                        inventory: Default::default(),
                    },
                    &assets,
                )
                .unwrap();
                let actual: Vec<_> = world
                    .items
                    .entries
                    .iter()
                    .map(|entry| {
                        let State::Pickup(pickup) = &entry.state else {
                            panic!("unexpected starting item")
                        };
                        assert!(assets.items.model(pickup.item).is_some());
                        (pickup.item, pickup.flight.position)
                    })
                    .collect();
                assert_eq!(actual, expected);
                count += actual.len();
                for _ in 0..120 {
                    world
                        .step([0; 2], &mut assets, &mut Recorded::default())
                        .unwrap_or_else(|e| {
                            panic!("stage {stage} layout {layout} hero {hero}: {e}")
                        });
                }
                assert!(
                    world.actors[world.player].actor.health > 0,
                    "stage {stage} layout {layout} hero {hero}"
                );
            }
        }
    }
    assert_eq!(count, 381);
    eprintln!(
        "127 original placements match for all three native heroes; 99 stage/layout/hero starts advanced for 120 ticks"
    );
}

#[test]
#[ignore = "requires both discs; native pickup eligibility, animation, inventory, notice, sound and snapshot continuation for all three imported item types and heroes"]
fn gaisen_pickups_use_native_collection_and_hud() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for hero in 0..3 {
        let source = MissionAssets::load(&game, 0, i16::from(hero == 1), 3).unwrap();
        let (mut assets, mut layout) = Assets::gaisen(&disc, &source, hero, false).unwrap();
        layout.actors.clear();
        let initial = World::new(
            &layout,
            Options {
                hero,
                difficulty: 2,
                seed: 1,
                inventory: Default::default(),
            },
            &assets,
        )
        .unwrap();
        assert_eq!(assets.gaisen_pickups.len(), 3);
        for (index, &(item, position)) in assets.gaisen_pickups.iter().enumerate() {
            for amount in [0u8, 254, 255] {
                let mut world = initial.clone();
                let model = world.items.entries[index].model;
                let actor = &mut world.actors[world.player].actor;
                actor.position = position;
                actor.previous = position;
                actor.inventory.counts[usize::from(item)] = amount;
                actor.state = ActorState::Item;
                world.scene.finish_tick();
                world.publish(world.player, &assets);
                world.collision.compute(|m| world.scene.world(m)).unwrap();
                let mut events = Recorded::default();
                world.update_items(&assets, &mut events).unwrap();
                assert!(
                    world.items.entries.iter().any(|e| e.model == model),
                    "busy hero collected item {item}"
                );
                assert!(events.0.is_empty());
                world.actors[world.player].actor.state = ActorState::Normal;
                let snapshot = postcard::to_allocvec(&world).unwrap();
                world.update_items(&assets, &mut events).unwrap();
                let actor = &world.actors[world.player].actor;
                assert_eq!(
                    actor.inventory.counts[usize::from(item)],
                    amount.wrapping_add(1),
                    "hero {hero} item {item}: {:?}; colliders {:?}",
                    world.items.entries.iter().find(|e| e.model == model),
                    world.collision.slots()
                );
                assert_eq!(actor.motion.action, 0x800);
                if amount != 255 {
                    assert_eq!(
                        actor.inventory.selected,
                        Some(Item::try_from(item).unwrap())
                    );
                }
                assert!(!world.items.entries.iter().any(|e| e.model == model));
                assert!(events.0.contains(&Event::Pickup(usize::from(item))));
                assert!(events.0.contains(&Event::SoundPosition { position, id: 7 }));
                let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
                let mut restored_events = Recorded::default();
                restored
                    .update_items(&assets, &mut restored_events)
                    .unwrap();
                assert_eq!(
                    postcard::to_allocvec(&world).unwrap(),
                    postcard::to_allocvec(&restored).unwrap()
                );
                assert_eq!(events.0, restored_events.0);
                let mut notice = false;
                for _ in 0..4 {
                    let actor = &world.actors[world.player].actor;
                    world
                        .controls
                        .hud
                        .step(
                            actor.into(),
                            actor.breath,
                            &actor.inventory,
                            world.ai.global.radar,
                            |_| None,
                        )
                        .unwrap();
                    notice |= world.controls.hud.frame.iter().any(|p| matches!(p,
                        Primitive::Sprite(s) if s.image == Image::Item(usize::from(item)) && s.position[1] == 58
                    ));
                }
                assert_eq!(
                    notice,
                    amount != 255,
                    "hero {hero} item {item} amount {amount}: native HUD hides the carousel and notices when the inventory becomes empty"
                );
            }
        }
        // Let an ordinary tick consume a placement contact, including the actor update.
        let mut world = initial;
        let position = assets.gaisen_pickups[0].1;
        let item = assets.gaisen_pickups[0].0;
        world.actors[world.player].actor.position = position;
        world.actors[world.player].actor.previous = position;
        world.scene.finish_tick();
        world.publish(world.player, &assets);
        world.collision.compute(|m| world.scene.world(m)).unwrap();
        let mut events = Recorded::default();
        for _ in 0..4 {
            world.step([0; 2], &mut assets, &mut events).unwrap();
        }
        assert!(
            events.0.contains(&Event::Pickup(usize::from(item))),
            "hero {hero}: full tick did not collect"
        );
    }
}

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; renders each imported pickup at its authored floor position"]
fn gaisen_pickups_render_at_original_positions() {
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let source = MissionAssets::load(&game, 0, 0, 3).unwrap();
    let mut session = Session::gaisen(
        &disc,
        &source,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 1,
            inventory: Default::default(),
        },
        false,
    )
    .unwrap();
    let initial = session.world.clone();
    let settings = crate::settings::Settings {
        show_hud: false,
        ..Default::default()
    };
    for &(item, position) in &session.assets.gaisen_pickups.clone() {
        session.world = initial.clone();
        let mut camera = crate::debug::camera::Camera {
            position: glam::Vec3::from_array(position.to_array().map(|v| v as f32))
                - glam::Vec3::Y * 60.0,
            yaw: 0.5,
            pitch: -0.7,
            fov: 45.0,
        };
        camera.position -= camera.forward() * 1800.0;
        session.world.camera.view = camera.source_view();
        session.world.camera.eye = camera.position.to_array().map(|v| v as i32).into();
        session.world.camera.look = position;
        session.world.actors[session.world.player].actor.position = position;
        session.frame.scenery = session
            .world
            .update_environment(&session.assets, &mut Recorded::default())
            .unwrap();
        render::capture(
            &session,
            &settings,
            &output.join(format!("pickup-{item}-after.png")),
        )
        .unwrap();
        session.world.items.entries.clear();
        render::capture(
            &session,
            &settings,
            &output.join(format!("pickup-{item}-before.png")),
        )
        .unwrap();
        assert!(
            std::fs::read(output.join(format!("pickup-{item}-after.png"))).unwrap()
                != std::fs::read(output.join(format!("pickup-{item}-before.png"))).unwrap(),
            "item {item}: pickup is invisible at its original floor position"
        );
    }
}
