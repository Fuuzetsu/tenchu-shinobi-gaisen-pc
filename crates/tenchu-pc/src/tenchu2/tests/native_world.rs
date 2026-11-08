use tenchu_assets::{
    tenchu2::{
        collision::{Cell, Collision},
        layout::Layout,
    },
    terrain::Node,
};
use tenchu_core::{
    Result,
    math::{Position, Vector3},
    random::Random,
    tenchu2::{
        actor::{State, Velocity},
        effects::{Effect, sprites::Motion},
        input::{ATTACK, ITEM},
        inventory::{Inventory, Item},
        physics::{Depth, HIT, Water},
        rig::WeaponMode,
        world::{
            Assets, Options, World,
            controls::{Event, EventSource, Events},
        },
    },
};

#[derive(Default)]
struct Recorded(Vec<Event>, Vec<tenchu_core::tenchu2::environment::Event>);
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
    fn emit(&mut self, _: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.0.push(event);
        Ok(())
    }
}

fn fixture() -> (World, Assets, usize) {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layout = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.sequence == 9)
        .unwrap();
    let mut assets = Assets::load(&disc, 0, &layout).unwrap();
    assets.level.collision = Collision {
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
    let mut world = World::new(
        &layout,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 123,
            inventory: Inventory::default(),
        },
        &assets,
    )
    .unwrap();
    let guard = world
        .actors
        .iter()
        .position(|a| matches!(a.actor.kind & 0xf0, 0x20..=0x50))
        .unwrap();
    for i in 0..world.actors.len() {
        let actor = &mut world.actors[i].actor;
        if i != 0 && i != guard {
            actor.flags |= 8;
            for &joint in &world.instances[i].rig.joints {
                world.collision.remove(joint);
            }
        } else {
            actor.flags &= !8;
            actor
                .set_body_collision(
                    world.instances[i].rig.joints[0],
                    i,
                    &mut world.collision,
                    true,
                )
                .unwrap();
            actor.position = Position::new(0, 0, if i == 0 { 0 } else { -1000 });
            actor.previous = actor.position;
            actor.rotation = Vector3::default();
            actor
                .change_motion(0, Velocity::Stop, &assets.resources)
                .unwrap();
            actor.motion.frame = 0;
            actor.terrain.height = 0;
            actor.terrain.floor = Some(0);
            actor.terrain.material = 0;
            actor
                .set_weapon(
                    &mut world.instances[i].rig,
                    i,
                    &mut world.collision,
                    &assets.resources,
                    WeaponMode::Sheathe,
                )
                .unwrap();
        }
        world.publish(i, &assets);
    }
    world.update_camera(&assets, true).unwrap();
    world.finish_tick(&assets, 6).unwrap();
    (world, assets, guard)
}

#[test]
#[ignore = "requires TENCHU2_DISC; exercises native hook controls through attachment, pulling and release"]
fn native_world_grapple_pulls_the_player_and_survives_a_snapshot() {
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.flags |= 8;
    for &joint in &world.instances[guard].rig.joints {
        world.collision.remove(joint);
    }
    let ground = world.terrain.cells[0].nodes[0];
    let mut roof = ground;
    roof.y = -300;
    roof.bounds[3] = -500;
    world.terrain.cells[0].nodes = vec![roof, ground];
    world.cursor = Default::default();
    world.camera.mode = 20;
    world.items.aim = tenchu_core::tenchu2::items::aim::Aim {
        direction: Position::new(0, -2000, -5500),
        target: Position::new(0, -2970, -5500),
        reach: Some(5500),
        distance: 5850,
        phase: 1,
    };
    let player = &mut world.actors[0].actor;
    player
        .change_motion(0x803, Velocity::Stop, &assets.resources)
        .unwrap();
    player.motion.frame = 4;
    player.inventory.active = Item::GRAPPLE;
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    assert!(world.items.grapple().is_some());
    assert_eq!(world.camera.mode, 19);
    let mut attached = false;
    let mut pulled = false;
    let mut restored = None;
    let (_, mut restored_assets, _) = fixture();
    let step = |world: &mut World, assets: &mut Assets| {
        let mut events = Recorded::default();
        world.move_actor(0, assets, &mut events).unwrap();
        world.control_actor(0, assets, &mut events).unwrap();
        world.animate_actor(0, assets).unwrap();
        world.update_items(assets, &mut events).unwrap();
        world.update_effects().unwrap();
        world.finish_tick(assets, 6).unwrap();
    };
    for tick in 0..120 {
        if tick == 5 {
            restored = Some(
                postcard::from_bytes::<World>(&postcard::to_allocvec(&world).unwrap()).unwrap(),
            );
        }
        step(&mut world, &mut assets);
        if let Some(restored) = &mut restored {
            step(restored, &mut restored_assets);
        }
        attached |= world.actors[0].actor.motion.action == 0x804;
        pulled |= world.actors[0].actor.position.z < -3000;
    }
    assert!(attached && pulled);
    assert!(world.items.grapple().is_none());
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored.unwrap()).unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; verifies paired kills reach a searchable physical corpse"]
fn native_stealth_kill_completes_both_actors_and_restores_corpse_collision() {
    let (mut world, mut assets, guard) = fixture();
    let mut events = Recorded::default();
    world.actors[0].actor.pad.update(ATTACK);
    world.control_actor(0, &mut assets, &mut events).unwrap();
    assert_eq!(world.actors[guard].actor.health, 0);
    assert_eq!(world.actors[guard].actor.state, State::Dead);
    assert_eq!(world.statistics.stealth_kills, 1);
    assert!(
        world
            .collision
            .index(world.instances[guard].rig.joints[0])
            .is_none()
    );
    let mut blood = false;
    for _ in 0..240 {
        for i in [0, guard] {
            world.move_actor(i, &assets, &mut events).unwrap();
            world.actors[i].actor.pad.update(0);
            world
                .control_actor(i, &mut assets, &mut events)
                .unwrap_or_else(|e| panic!("actor {i} tick {}: {e}", world.tick));
            world.animate_actor(i, &assets).unwrap();
        }
        blood |= world
            .effects
            .entries
            .iter()
            .any(|e| matches!(e, Effect::Blood(_)));
        world.update_items(&assets, &mut events).unwrap();
        world.update_effects().unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(matches!(
        world.actors[0].actor.state,
        State::Normal | State::Ready
    ));
    assert_eq!(world.actors[guard].actor.motion.loops, i16::MIN);
    assert!(
        world
            .collision
            .index(world.instances[guard].rig.joints[0])
            .is_some()
    );
    assert!(blood, "stealth kill did not create native blood");
    assert_eq!(world.statistics.killed, 1);
}

#[test]
#[ignore = "requires TENCHU2_DISC; exercises weapon hit, healing item and drowning through outer controls"]
fn native_damage_medicine_and_drowning_use_the_shared_world() {
    let (mut world, mut assets, guard) = fixture();
    let mut events = Recorded::default();
    let player = &mut world.actors[0].actor;
    player
        .set_weapon(
            &mut world.instances[0].rig,
            0,
            &mut world.collision,
            &assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    player
        .change_motion(0x900, Velocity::Motion, &assets.resources)
        .unwrap();
    player.attack.index = assets
        .resources
        .weapons
        .attacks
        .iter()
        .position(|a| Some(a.clip) == player.motion.clip);
    assert!(player.attack.index.is_some());
    let target = &mut world.actors[guard].actor;
    target
        .set_weapon(
            &mut world.instances[guard].rig,
            guard,
            &mut world.collision,
            &assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    target.flags |= 2;
    world.finish_tick(&assets, 6).unwrap();
    let weapon = world
        .collision
        .slots()
        .iter()
        .position(|s| s.owner == Some(0) && s.flags & 0x4000 != 0)
        .unwrap();
    let target = &mut world.actors[guard].actor;
    let health = target.health;
    target.flags |= HIT;
    target.hit = Some(weapon);
    world
        .control_actor(guard, &mut assets, &mut events)
        .unwrap();
    assert!(world.actors[guard].actor.health < health);
    assert!(
        world.actors[0].actor.motion.loops < 0,
        "attacker hit pause was not propagated"
    );
    assert!(
        world
            .effects
            .entries
            .iter()
            .any(|e| matches!(e, Effect::Sprites(s) if matches!(s.frames[0].image, 16 | 18)))
    );

    let player = &mut world.actors[0].actor;
    player
        .change_motion(0x100, Velocity::Stop, &assets.resources)
        .unwrap();
    player.health = 1;
    player.inventory.selected = Some(Item::try_from(1).unwrap());
    player.inventory.counts[1] = 1;
    player.pad.update(ITEM);
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut healing = false;
    for _ in 0..180 {
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, &assets).unwrap();
        healing |= world
            .effects
            .entries
            .iter()
            .any(|e| matches!(e, Effect::Sprites(s) if matches!(s.motion, Motion::Healing)));
        world.update_items(&assets, &mut events).unwrap();
        world.update_effects().unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(
        world.actors[0].actor.health,
        world.actors[0].actor.max_health
    );
    assert_eq!(world.actors[0].actor.inventory.counts[1], 0);
    assert!(healing, "medicine did not create native healing particles");

    let player = &mut world.actors[0].actor;
    player.health = 2;
    player.breath = 0;
    player
        .change_motion(0x530, Velocity::Stop, &assets.resources)
        .unwrap();
    world.water = Water {
        depth: Some(Depth::Submerged),
        surface: -4000,
        bottom: 0,
    };
    for _ in 0..9 {
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, &assets).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(world.actors[0].actor.health, 0);
    assert_eq!(world.actors[0].actor.state, State::Dead);
}

#[test]
#[ignore = "requires TENCHU2_DISC; continuous native actors with recorded effect boundaries"]
fn native_campaign_casts_run_physics_ai_controllers_and_animation_together() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let mut checked = 0;
    for hero in 0..3 {
        for layout in layouts.iter().filter(|l| match hero {
            0 => l.sequence < 101,
            1 => (101..185).contains(&l.sequence),
            _ => l.sequence >= 185,
        }) {
            let mut assets = Assets::load(&disc, hero, layout).unwrap();
            let mut world = World::new(
                layout,
                Options {
                    hero,
                    difficulty: 2,
                    seed: 4,
                    inventory: Inventory::default(),
                },
                &assets,
            )
            .unwrap();
            let mut events = Recorded::default();
            for tick in 0..180 {
                let buttons = match tick % 90 {
                    0..=29 => tenchu_core::tenchu2::input::UP,
                    30..=39 => ATTACK,
                    60..=69 => tenchu_core::tenchu2::input::JUMP,
                    _ => 0,
                };
                world.update_camera(&assets, false).unwrap();
                world
                    .update_actors([buttons, 0], &mut assets, &mut events)
                    .unwrap_or_else(|error| {
                        panic!("sequence {} tick {tick}: {error}", layout.sequence)
                    });
                world
                    .update_environment(&assets, &mut events)
                    .unwrap_or_else(|error| {
                        panic!("sequence {} tick {tick}: {error}", layout.sequence)
                    });
                world.update_items(&assets, &mut events).unwrap();
                world.update_effects().unwrap();
                world.finish_tick(&assets, 6).unwrap();
            }
            eprintln!(
                "JP hero {hero} sequence {}: 180 continuous actor ticks",
                layout.sequence
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 56);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; exercises door contact, floor destruction and restored mutable terrain"]
fn native_objects_share_actor_contacts_and_snapshot_the_changed_floor() {
    use tenchu_assets::tenchu2::level::objects::Bounds;
    use tenchu_core::{
        math::Matrix,
        tenchu2::{
            collision::BODY,
            environment::{Environment, Object, Trigger},
        },
    };
    let (mut world, mut assets, _) = fixture();
    let coordinate = world.scene.add(None, Matrix::default());
    world.environment = Environment {
        static_phases: vec![0; assets.level.scene.statics.len()],
        objects: vec![Object {
            cell: assets
                .level
                .grid
                .cells
                .iter()
                .position(Option::is_some)
                .unwrap(),
            coordinate,
            position: Position::default(),
            yaw: 0,
            roll: 0,
            model: Some(0),
            kind: 0,
            link: 0,
            clock: 0,
            actor: None,
            fall_yaw: None,
        }],
        triggers: vec![Trigger {
            position: Position::default(),
            angle: 0,
            kind: 0,
            target: 0,
            clock: 0,
        }],
        ..Environment::default()
    };
    assets.level.scene.bounds = vec![Bounds {
        offset: [0; 3],
        top: -1000,
        half_width: 500,
        half_depth: 200,
    }];
    world
        .environment
        .enable_collision(
            0,
            &assets.level.scene.bounds,
            &mut world.collision,
            &assets.resources.math,
        )
        .unwrap();
    let body = world.scene.add(None, Matrix::default());
    let slot = world.collision.insert(body).unwrap();
    slot.flags = BODY;
    slot.size = Vector3::new(100, 100, 100);
    slot.owner = Some(world.player);
    world.finish_tick(&assets, 6).unwrap();
    let mut events = Recorded::default();
    for _ in 0..26 {
        world.update_environment(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(
        world.environment.objects[0].position,
        Position::new(-1040, 0, 0)
    );
    assert_eq!(world.environment.triggers[0].clock, 0);
    assert!(world.collision.index(coordinate).is_none());

    let object = &mut world.environment.objects[0];
    object.position = Position::default();
    object.kind = 7;
    object.cell = usize::MAX;
    object.link = 0;
    object.clock = 0;
    object.actor = None;
    world.environment.triggers[0].kind = 7;
    world.terrain.cells[0].nodes[0].attributes = 0x400;
    let player = &mut world.actors[world.player].actor;
    player.position = Position::default();
    player.previous = Position::default();
    player.velocity = Vector3::default();
    let mut snapshots = Vec::new();
    for tick in 0..100 {
        world
            .move_actor(world.player, &assets, &mut events)
            .unwrap();
        world.update_environment(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
        if tick == 20 {
            snapshots.push(postcard::to_allocvec(&world).unwrap());
        }
    }
    assert!(world.environment.objects[0].model.is_none());
    let node = &world.terrain.cells[0].nodes[0];
    assert_eq!((node.y, node.attributes, node.division), (0, 0x4401, 0));
    assert_eq!(assets.level.collision.cells[0].nodes[0].attributes, 0);
    let mut restored: World = postcard::from_bytes(&snapshots[0]).unwrap();
    for _ in 21..100 {
        restored
            .move_actor(restored.player, &assets, &mut events)
            .unwrap();
        restored.update_environment(&assets, &mut events).unwrap();
        restored.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(
        postcard::to_allocvec(&restored).unwrap(),
        postcard::to_allocvec(&world).unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; exercises throwing, damaging a guard, retrieving a shuriken and bounded item reuse"]
fn native_shuriken_hits_and_returns_to_inventory() {
    use tenchu_core::tenchu2::items::{
        State as ItemState,
        projectile::{Kind, Projectile},
    };
    let (mut world, mut assets, guard) = fixture();
    world.actors[0].actor.inventory.counts[4] = 3;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(4).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let health = world.actors[guard].actor.health;
    let mut launched = false;
    let mut hit = false;
    for _ in 0..100 {
        for i in [0, guard] {
            world.move_actor(i, &assets, &mut events).unwrap();
            world.actors[i].actor.pad.update(0);
            world.control_actor(i, &mut assets, &mut events).unwrap();
            world.animate_actor(i, &assets).unwrap();
        }
        if !launched && let Some(entry) = world.items.entries.first() {
            let ItemState::Projectile(projectile) = &entry.state else {
                panic!("wrong item");
            };
            let target = projectile
                .flight
                .position
                .wrapping_add(projectile.flight.velocity.map(|v| (v * 1500) >> 12));
            let actor = &mut world.actors[guard].actor;
            actor.position.x = target.x;
            actor.position.z = target.z;
            actor.previous = actor.position;
            world.publish(guard, &assets);
            launched = true;
        }
        world.update_items(&assets, &mut events).unwrap();
        world.update_effects().unwrap();
        world.finish_tick(&assets, 6).unwrap();
        hit |= world.actors[guard].actor.health < health;
    }
    assert!(
        launched && hit,
        "no native shuriken hit: launched={launched}, health={health} -> {}, pending={:?}",
        world.actors[guard].actor.health,
        events.0
    );
    assert_eq!(world.actors[0].actor.inventory.counts[4], 2);
    assert!(world.items.entries.is_empty());
    // A missed throw becomes a physical pickup rather than disappearing on the floor.
    world.actors[guard].actor.flags |= 8;
    let origin = Position::new(4000, -20, 4000);
    let shot = Projectile::launch(
        Kind::Shuriken,
        0,
        origin,
        Position::new(0, 4096, 0),
        &assets.resources.math,
    );
    world
        .items
        .insert(
            ItemState::Projectile(shot),
            &mut world.scene,
            &mut world.collision,
        )
        .unwrap();
    world.update_items(&assets, &mut events).unwrap();
    world.finish_tick(&assets, 6).unwrap();
    let player = &mut world.actors[0].actor;
    player.position = Position::new(4000, 0, 4000);
    player.previous = player.position;
    player
        .change_motion(0, Velocity::Stop, &assets.resources)
        .unwrap();
    world.publish(0, &assets);
    world.finish_tick(&assets, 6).unwrap();
    world.update_items(&assets, &mut events).unwrap();
    assert!(
        world.items.entries.is_empty(),
        "resting shuriken was not picked up"
    );
    assert_eq!(world.actors[0].actor.inventory.counts[4], 3);
    let initial_models = world.scene.models.len();
    for i in 0..240 {
        let shot = Projectile::launch(
            Kind::Shuriken,
            0,
            Position::new(i * 100, -1500, 0),
            Position::new(0, 0, -4096),
            &assets.resources.math,
        );
        world
            .items
            .insert(
                ItemState::Projectile(shot),
                &mut world.scene,
                &mut world.collision,
            )
            .unwrap();
        world
            .collision
            .compute(|model| world.scene.world(model))
            .unwrap();
    }
    assert_eq!(world.items.entries.len(), 30);
    assert!(
        world.scene.models.len() <= initial_models + 30,
        "projectile coordinates grow without bound"
    );
    let snapshot = postcard::to_allocvec(&world).unwrap();
    let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
    for _ in 0..20 {
        world.update_items(&assets, &mut events).unwrap();
        restored
            .update_items(&assets, &mut Recorded::default())
            .unwrap();
        world.finish_tick(&assets, 6).unwrap();
        restored.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored).unwrap()
    );
}
