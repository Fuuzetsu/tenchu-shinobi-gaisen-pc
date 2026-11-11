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
pub(super) struct Recorded(Vec<Event>, Vec<tenchu_core::tenchu2::environment::Event>);
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
impl tenchu_core::tenchu2::ai::behavior::Events for Recorded {
    fn voice(&mut self, _: &tenchu_core::tenchu2::actor::Actor, id: u8) -> Result<()> {
        self.0.push(Event::Voice(id));
        Ok(())
    }
}
impl Events for Recorded {
    fn emit(&mut self, _: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.0.push(event);
        Ok(())
    }
}

pub(in crate::tenchu2) fn fixture() -> (World, Assets, usize) {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layout = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.sequence == 9)
        .unwrap();
    let mut assets = Assets::load(&disc, 0, &layout).unwrap();
    assets.level.collision = Collision {
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
#[ignore = "requires TENCHU2_DISC; special-move cloud emits once and affects nearby actors after expanding"]
fn native_special_cloud_reaches_damage_and_particles() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    let actor = &mut world.actors[guard].actor;
    actor.state = State::Special;
    actor.motion.action = 0x70d;
    actor.motion.clip = Some(0x121);
    actor.motion.frame = 20;
    actor.motion.loops = 0;
    let mut events = Recorded::default();
    world
        .control_actor(guard, &mut assets, &mut events)
        .unwrap();
    assert!(!events.0.iter().any(|e| matches!(e, Event::Throw { .. })));
    let cloud = world
        .items
        .entries
        .iter()
        .find_map(|entry| match &entry.state {
            ItemState::Cloud(cloud) => Some(cloud),
            _ => None,
        })
        .expect("special move did not create its cloud");
    assert_eq!(cloud.owner, Some(guard));
    let actor = &mut world.actors[0].actor;
    actor.position = Position::new(cloud.position.x + 1500, 0, cloud.position.z);
    actor.previous = actor.position;
    let health = actor.health;
    world.publish(0, &assets);
    let mut affected = false;
    for tick in 0..81 {
        world.update_items(&assets, &mut events).unwrap();
        if tick == 0 {
            assert!(world.effects.entries.iter().any(|effect| matches!(effect, Effect::Sprites(s) if matches!(s.motion, Motion::Dispersing))));
        }
        world.finish_tick(&assets, 6).unwrap();
        world.move_actor(0, &assets, &mut events).unwrap();
        world.control_actor(0, &mut assets, &mut events).unwrap();
        if world.actors[0].actor.flags & 0x800 != 0 {
            assert!(tick >= 32, "cloud affected player before expanding");
            affected = true;
        }
        world.update_effects(&assets).unwrap();
    }
    assert!(affected, "expanded cloud never affected the player");
    assert_eq!(world.actors[0].actor.health, health);
    assert!(
        !world
            .items
            .entries
            .iter()
            .any(|entry| matches!(entry.state, ItemState::Cloud(_)))
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; stepping on a smoke trap creates its blinding cloud once"]
fn native_surface_smoke_trap_blinds_the_player() {
    use tenchu_core::tenchu2::{environment::Trigger, items::State as ItemState};
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.flags |= 8;
    for &joint in &world.instances[guard].rig.joints {
        world.collision.remove(joint);
    }
    world.terrain.cells[0].nodes[0].attributes = 0x400;
    world.environment.triggers = vec![Trigger {
        position: Position::new(0, -600, -1500),
        angle: 0,
        kind: 14,
        target: 0,
        clock: 0,
    }];
    let mut events = Recorded::default();
    world.move_actor(0, &assets, &mut events).unwrap();
    assert!(events.1.is_empty(), "trap event escaped the runtime");
    assert_eq!(world.items.entries.len(), 1);
    let entry = &world.items.entries[0];
    let ItemState::Cloud(cloud) = &entry.state else {
        panic!("trap did not create a cloud")
    };
    assert_eq!(cloud.owner, None);
    assert_eq!(cloud.direction, Position::new(0, 0, 10));
    assert_eq!(world.collision.get_mut(entry.model).unwrap().flags, 0x500a);
    let mut blinded = false;
    for _ in 0..81 {
        world.update_items(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
        world.move_actor(0, &assets, &mut events).unwrap();
        world.control_actor(0, &mut assets, &mut events).unwrap();
        blinded |= world.actors[0].actor.flags & 0x200 != 0;
        world.update_effects(&assets).unwrap();
        assert!(world.items.entries.len() <= 1, "trap fired repeatedly");
    }
    assert!(blinded);
    assert!(world.items.entries.is_empty());
}

#[test]
#[ignore = "requires TENCHU2_DISC; source floor triggers create spear/arrow traps that hurt the player"]
fn native_surface_projectile_traps_reach_the_player() {
    use tenchu_core::tenchu2::{
        environment::{Event as SceneryEvent, Trigger},
        items::State as ItemState,
    };
    for kind in 10..=13 {
        let (mut world, mut assets, guard) = fixture();
        world.actors[guard].actor.flags |= 8;
        for &joint in &world.instances[guard].rig.joints {
            world.collision.remove(joint);
        }
        world.terrain.cells[0].nodes[0].attributes = 0x400;
        world.environment.triggers = vec![Trigger {
            position: Position::new(-1800, -900, 0),
            angle: 192,
            kind,
            target: 0,
            clock: 0,
        }];
        let mut events = Recorded::default();
        world.move_actor(0, &assets, &mut events).unwrap();
        assert!(
            !events
                .1
                .iter()
                .any(|e| matches!(e, SceneryEvent::Shoot { .. })),
            "unhandled trap {kind}"
        );
        assert_eq!(world.items.entries.len(), 1);
        assert!(matches!(
            (kind, &world.items.entries[0].state),
            (10 | 11, ItemState::Spear(_)) | (12 | 13, ItemState::Arrow(_))
        ));
        let health = world.actors[0].actor.health;
        for _ in 0..41 {
            world.update_items(&assets, &mut events).unwrap();
            world.finish_tick(&assets, 6).unwrap();
            world.move_actor(0, &assets, &mut events).unwrap();
            world.control_actor(0, &mut assets, &mut events).unwrap();
            world.update_effects(&assets).unwrap();
            assert!(world.items.entries.len() <= 1);
        }
        assert!(
            world.actors[0].actor.health < health,
            "trap {kind} caused no damage"
        );
        if kind < 12 {
            assert!(world.items.entries.is_empty());
        }
    }
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
        world.update_effects(assets).unwrap();
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
    assert!(!world.player_can_orbit());
    assert!(!world.player_can_steer());
    let during = postcard::to_allocvec(&world).unwrap();
    assert!(
        world
            .teleport_player(Position::new(1500, 0, 1500), true, &assets)
            .is_err()
    );
    assert!(during == postcard::to_allocvec(&world).unwrap());
    let mut blood = false;
    let mut visible_blood = false;
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
        let effects = world.update_effects(&assets).unwrap();
        visible_blood |= effects.sprites.iter().any(|s| s.blood.is_some())
            || effects.points.iter().any(|p| p.blood.is_some());
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
    assert!(
        visible_blood,
        "stealth-kill blood never reached the shared renderer"
    );
    assert_eq!(world.statistics.killed, 1);
    assert!(world.controls.stealth.is_some());
    let history = postcard::to_allocvec(&world.controls.stealth).unwrap();
    let random = world.random;
    assert!(
        world.player_can_orbit(),
        "completed kill blocked camera orbit"
    );
    assert!(
        world.player_can_steer(),
        "completed kill blocked relative movement"
    );
    let heading = world.actors[0].actor.rotation.y.wrapping_add(1024);
    world.steer_player(heading, &assets).unwrap();
    assert_eq!(world.actors[0].actor.rotation.y, heading);
    let destination = Position::new(1500, 0, 1500);
    world.teleport_player(destination, true, &assets).unwrap();
    assert_eq!(world.actors[0].actor.position, destination);
    assert_eq!(
        postcard::to_allocvec(&world.controls.stealth).unwrap(),
        history
    );
    assert_eq!(world.random, random);
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
        world.update_effects(&assets).unwrap();
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
    check_campaign_casts(false);
}

#[test]
#[ignore = "requires TENCHU2_DISC; every JP campaign area with the complete cast awake"]
fn native_all_active_campaign_casts_run_together() {
    check_campaign_casts(true);
}

fn check_campaign_casts(keep_all_active: bool) {
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
            world.keep_all_active = keep_all_active;
            let mut events = Recorded::default();
            for tick in 0..180 {
                let buttons = match tick % 90 {
                    0..=29 => tenchu_core::tenchu2::input::UP,
                    30..=39 => ATTACK,
                    60..=69 => tenchu_core::tenchu2::input::JUMP,
                    _ => 0,
                };
                world
                    .step([buttons, 0], &mut assets, &mut events)
                    .unwrap_or_else(|error| {
                        panic!(
                            "sequence {} tick {tick}, all active {keep_all_active}: {error}",
                            layout.sequence
                        )
                    });
                if keep_all_active {
                    assert!(world.actors.iter().all(|a| a.actor.flags & 8 == 0));
                }
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
    let mut ribbon = false;
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
                .wrapping_add(projectile.flight.velocity.map(|v| (v * 4000) >> 12));
            let actor = &mut world.actors[guard].actor;
            actor.position.x = target.x;
            actor.position.z = target.z;
            actor.previous = actor.position;
            world.publish(guard, &assets);
            launched = true;
        }
        world.update_items(&assets, &mut events).unwrap();
        ribbon |= !world.update_effects(&assets).unwrap().ribbons.is_empty();
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
    assert!(ribbon, "throw has no native projectile trail");
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

#[test]
#[ignore = "requires TENCHU2_DISC; actual melee controls emit, snapshot and retire native weapon trails"]
fn native_weapon_trails_follow_attacks_and_snapshot_continuation() {
    use tenchu_core::tenchu2::effects::ribbon::Source;
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.flags |= 8;
    for &joint in &world.instances[guard].rig.joints {
        world.collision.remove(joint);
    }
    let (_, mut restored_assets, _) = fixture();
    let mut restored: Option<World> = None;
    let mut drawn = false;
    for tick in 0..160 {
        let buttons = if matches!(tick, 0 | 35 | 70) {
            ATTACK
        } else {
            0
        };
        let frame = world
            .step([buttons, 0], &mut assets, &mut Recorded::default())
            .unwrap();
        if let Some(copy) = &mut restored {
            let repeated = copy
                .step([buttons, 0], &mut restored_assets, &mut Recorded::default())
                .unwrap();
            assert_eq!(frame.effects.ribbons, repeated.effects.ribbons);
        }
        if !frame.effects.ribbons.is_empty() {
            drawn = true;
            if restored.is_none() {
                restored =
                    Some(postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap());
            }
        }
    }
    assert!(drawn, "ordinary melee controls emitted no weapon trail");
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored.unwrap()).unwrap()
    );
    assert!(!world.effects.entries.iter().any(|effect| matches!(effect, Effect::Ribbon(r) if matches!(r.source, Source::Weapon { actor: 0, .. }))));
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual blowgun controls, poison damage and in-flight snapshot continuation"]
fn native_poison_dart_hits_unaware_and_armed_guards() {
    use tenchu_core::tenchu2::items::State as ItemState;
    for armed in [false, true] {
        let (mut world, mut assets, guard) = fixture();
        world.actors[guard].actor.position = Position::new(10000, 0, -10000);
        world.actors[guard].actor.previous = world.actors[guard].actor.position;
        world.publish(guard, &assets);
        world.finish_tick(&assets, 6).unwrap();
        if armed {
            world.actors[guard]
                .actor
                .set_weapon(
                    &mut world.instances[guard].rig,
                    guard,
                    &mut world.collision,
                    &assets.resources,
                    WeaponMode::Draw,
                )
                .unwrap();
        }
        let health = world.actors[guard].actor.health;
        world.actors[0].actor.inventory.counts[8] = 2;
        world.actors[0].actor.inventory.selected = Some(Item::try_from(8).unwrap());
        world.actors[0].actor.pad.update(ITEM);
        let mut events = Recorded::default();
        world.control_actor(0, &mut assets, &mut events).unwrap();
        let mut restored: Option<World> = None;
        let (_, mut restored_assets, _) = fixture();
        let mut hit = false;
        let mut drawn = false;
        let step = |world: &mut World, assets: &mut Assets, events: &mut Recorded| {
            for index in [0, guard] {
                world.move_actor(index, assets, events).unwrap();
                world.actors[index].actor.pad.update(0);
                world.control_actor(index, assets, events).unwrap();
                world.animate_actor(index, assets).unwrap();
            }
            world.update_items(assets, events).unwrap();
            let frame = world.update_effects(assets).unwrap();
            world.finish_tick(assets, 6).unwrap();
            frame
        };
        for _ in 0..100 {
            let frame = step(&mut world, &mut assets, &mut events);
            if let Some(copy) = &mut restored {
                let repeated = step(copy, &mut restored_assets, &mut Recorded::default());
                assert_eq!(frame.ribbons, repeated.ribbons);
            } else if let Some(ItemState::Dart(dart)) =
                world.items.entries.first().map(|e| &e.state)
            {
                let target = dart
                    .flight
                    .position
                    .wrapping_add(dart.flight.velocity.map(|v| v * 8));
                let actor = &mut world.actors[guard].actor;
                actor.position.x = target.x;
                actor.position.z = target.z;
                actor.previous = actor.position;
                world.publish(guard, &assets);
                restored =
                    Some(postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap());
            }
            drawn |= !frame.ribbons.is_empty();
            hit |= world.actors[guard].actor.flags & 0x100 != 0;
        }
        assert!(
            hit && drawn,
            "dart armed={armed}, hit={hit}, drawn={drawn}, pending={:?}",
            events.0
        );
        assert_eq!(world.actors[0].actor.inventory.counts[8], 1);
        let remaining = world.actors[guard].actor.health;
        assert!(if armed {
            remaining > 0 && remaining < health
        } else {
            remaining == 0
        });
        assert!(world.items.entries.is_empty());
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&restored.unwrap()).unwrap()
        );
        assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; native AI must nock, fire and injure the player through the full world loop"]
fn native_archers_and_gunners_land_their_attacks() {
    use tenchu_core::tenchu2::items::{State as ItemState, arrow::Phase};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let registry = tenchu_assets::tenchu2::registry::Registry::load(&disc).unwrap();
    let (_, flat_assets, _) = fixture();
    for mode in ["bow", "fire", "gun"] {
        let clip = if mode == "gun" { 0x418 } else { 0x205 };
        let layout = layouts
            .iter()
            .find(|l| {
                (mode != "fire" || matches!(l.map, 4..=6 | 17 | 21))
                    && l.actors.iter().any(|a| {
                        registry.clip(a.kind, 0x900) == Some(clip)
                            && a.behavior & 0xf00 == 0x100
                            && (mode != "fire" || a.kind & 0xf0 == 0x40)
                    })
            })
            .unwrap_or_else(|| panic!("No {mode} layout"));
        let mut layout = layout.clone();
        let archer = layout
            .actors
            .iter()
            .find(|a| {
                registry.clip(a.kind, 0x900) == Some(clip)
                    && a.behavior & 0xf00 == 0x100
                    && (mode != "fire" || a.kind & 0xf0 == 0x40)
            })
            .unwrap()
            .clone();
        layout.actors = vec![archer];
        layout.start = [0; 3];
        layout.facing = 2048;
        layout.actors[0].position = [0, 0, 6000];
        layout.actors[0].facing = 0;
        layout.actors[0].route.clear();
        let hero = if layout.sequence >= 185 {
            2
        } else {
            usize::from(layout.sequence >= 101)
        };
        let mut assets = Assets::load(&disc, hero, &layout).unwrap();
        assets.level.collision = flat_assets.level.collision.clone();
        let mut world = World::new(
            &layout,
            Options {
                hero,
                difficulty: 2,
                seed: 123,
                inventory: Inventory::default(),
            },
            &assets,
        )
        .unwrap();
        world.finish_tick(&assets, 6).unwrap();
        let mut events = Recorded::default();
        let mut nocked = false;
        let mut shot = false;
        let mut flame = false;
        let mut embedded = false;
        let mut snapshot = None;
        let mut original = world.clone();
        let mut escalated = world.clone();
        escalated.set_damage_escalation(tenchu_core::ai::escalation::Settings {
            increment: 50,
            maximum: 100,
        });
        escalated.actors[1].actor.encounters.detections = 3;
        world.set_bleeding(tenchu_core::game::bleeding::Settings {
            enabled: true,
            chance: 100,
            seconds: 0,
            damage: 0,
            ..Default::default()
        });
        for _ in 0..2400 {
            world
                .step([0; 2], &mut assets, &mut events)
                .unwrap_or_else(|e| panic!("{mode} tick {}: {e}", world.tick));
            original
                .step([0; 2], &mut assets, &mut Recorded::default())
                .unwrap();
            escalated
                .step([0; 2], &mut assets, &mut Recorded::default())
                .unwrap();
            assert_eq!(
                escalated.random, original.random,
                "{mode}: damage bonus altered RNG"
            );
            assert_eq!(
                100 - escalated.actors[0].actor.health,
                (100 - original.actors[0].actor.health) * 2,
                "{mode}: projectile lost the shooter's damage bonus"
            );
            assert_eq!(
                world.random, original.random,
                "{mode}: bleeding altered source RNG"
            );
            assert_eq!(
                world.actors[0].actor.health,
                original.actors[0].actor.health
            );
            nocked |= world.instances[1].rig.weapons[2].is_some();
            for entry in &world.items.entries {
                match &entry.state {
                    ItemState::Arrow(arrow) => {
                        assert_eq!(arrow.launched_by, Some(1));
                        shot = true;
                        flame |= arrow.fire_model;
                        embedded |= matches!(
                            arrow.phase,
                            Phase::Embedded {
                                parent: Some(_),
                                ..
                            }
                        );
                    }
                    ItemState::Gunshot(_) => shot = true,
                    _ => {}
                }
            }
            if shot && snapshot.is_none() {
                snapshot = Some(postcard::to_allocvec(&world).unwrap());
            }
            if world.actors[0].actor.health < 100 && (mode != "fire" || flame) {
                break;
            }
        }
        assert!(
            shot && world.actors[0].actor.health < 100,
            "{mode}: no hit, nocked={nocked}, shot={shot}, ticks={}, health={}, action={:x}, pending={:?}",
            world.tick,
            world.actors[0].actor.health,
            world.actors[1].actor.motion.action,
            events.0
        );
        assert!(mode == "gun" || nocked, "{mode}: never nocked an arrow");
        if mode != "fire" {
            assert!(
                world.actors[0].actor.bleeding.as_ref().unwrap().active,
                "{mode}: damaging shot did not start bleeding"
            );
        }
        assert!(
            mode != "fire" || flame,
            "fire-arrow chance was never exercised"
        );
        assert!(
            !events
                .0
                .iter()
                .any(|e| matches!(e, Event::NockArrow | Event::Fire(_)))
        );
        eprintln!(
            "{mode}: sequence={}, kind={:x}, tick={}, player health={}, attached={embedded}",
            layout.sequence, world.actors[1].actor.kind, world.tick, world.actors[0].actor.health
        );
        let bytes = snapshot.unwrap();
        let mut original: World = postcard::from_bytes(&bytes).unwrap();
        let mut restored: World = postcard::from_bytes(&bytes).unwrap();
        for _ in 0..200 {
            original
                .step([0; 2], &mut assets, &mut Recorded::default())
                .unwrap();
            restored
                .step([0; 2], &mut assets, &mut Recorded::default())
                .unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&original).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; uses the fire-arrow item through native controls and damages a guard"]
fn native_player_fire_arrow_uses_inventory_and_burns_a_guard() {
    use tenchu_core::tenchu2::items::{State as ItemState, arrow::Phase};
    let (mut world, mut assets, guard) = fixture();
    world.actors[0].actor.inventory.counts[9] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(9).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let before = world.actors[guard].actor.health;
    let mut flaming = false;
    let mut explosion = false;
    let mut lit = false;
    for _ in 0..100 {
        for i in [0, guard] {
            world.move_actor(i, &assets, &mut events).unwrap();
            world.actors[i].actor.pad.update(0);
            world.control_actor(i, &mut assets, &mut events).unwrap();
            world.animate_actor(i, &assets).unwrap();
        }
        world.update_items(&assets, &mut events).unwrap();
        for entry in &world.items.entries {
            if let ItemState::Arrow(arrow) = &entry.state {
                flaming |= arrow.fire_model;
                explosion |= matches!(arrow.phase, Phase::Exploding(_));
            }
        }
        lit |= !world.update_effects(&assets).unwrap().emitters.is_empty();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(world.actors[0].actor.inventory.counts[9], 1);
    assert!(
        flaming && explosion && lit,
        "fire arrow did not launch/explode: {flaming}/{explosion}"
    );
    assert!(
        world.actors[guard].actor.health < before,
        "fire arrow caused no damage"
    );
    assert!(
        events
            .0
            .iter()
            .any(|e| matches!(e, Event::SoundPosition { id: 51, .. }))
    );
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; connected grenade/mine damage, smoke affliction and inventory use"]
fn native_explosives_damage_and_smoke_blinds_through_item_controls() {
    use tenchu_core::tenchu2::items::State as ItemState;
    for item in [6, 7, 10] {
        let (mut world, mut assets, guard) = fixture();
        world.actors[guard].actor.position.z = -1800;
        world.actors[guard].actor.previous = world.actors[guard].actor.position;
        world.publish(guard, &assets);
        world.finish_tick(&assets, 6).unwrap();
        world.actors[0].actor.inventory.counts[item] = 2;
        world.actors[0].actor.inventory.selected = Some(Item::try_from(item as u8).unwrap());
        world.actors[0].actor.pad.update(ITEM);
        let mut events = Recorded::default();
        world.control_actor(0, &mut assets, &mut events).unwrap();
        let health = world.actors[guard].actor.health;
        let mut created = false;
        let mut blinded = false;
        for _ in 0..180 {
            for i in [0, guard] {
                world.move_actor(i, &assets, &mut events).unwrap();
                world.actors[i].actor.pad.update(0);
                world.control_actor(i, &mut assets, &mut events).unwrap();
                world.animate_actor(i, &assets).unwrap();
            }
            world.update_items(&assets, &mut events).unwrap();
            if item == 7
                && !created
                && let Some(position) = world.items.entries.iter().find_map(|entry| {
                    if let ItemState::Mine(mine) = &entry.state {
                        Some(mine.transform.translation)
                    } else {
                        None
                    }
                })
            {
                world.actors[guard].actor.position = position;
                world.actors[guard].actor.previous = position;
                world.publish(guard, &assets);
            }
            created |= world.items.entries.iter().any(|entry| {
                matches!(
                    (item, &entry.state),
                    (6, ItemState::Grenade(_))
                        | (7, ItemState::Mine(_))
                        | (10, ItemState::Smoke(_))
                )
            });
            blinded |= world.actors[guard].actor.flags & 0x200 != 0;
            world.update_effects(&assets).unwrap();
            world.finish_tick(&assets, 6).unwrap();
        }
        assert!(created, "item {item} never appeared");
        assert_eq!(
            world.actors[0].actor.inventory.counts[item], 1,
            "item {item} count"
        );
        let sound = if item == 10 { 52 } else { 51 };
        assert!(
            events
                .0
                .iter()
                .any(|event| matches!(event, Event::SoundPosition { id, .. } if *id == sound)),
            "item {item} did not play sound {sound}"
        );
        if item == 10 {
            assert!(blinded, "smoke failed to blind the guard");
        } else {
            assert!(
                world.actors[guard].actor.health < health,
                "item {item} caused no damage"
            );
        }
        assert!(
            !events
                .0
                .iter()
                .any(|event| matches!(event, Event::UseItem { .. }))
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; invisibility use, guard perception, item restriction and expiry after a snapshot"]
fn native_invisibility_changes_guard_perception_and_expires_after_a_snapshot() {
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.rotation.y = 2048;
    world.publish(guard, &assets);
    world.actors[0].actor.inventory.counts[13] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(13).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut restored: Option<World> = None;
    let (_, mut restored_assets, _) = fixture();
    let mut translucent = false;
    let mut veil = false;
    let mut ribbon = false;
    let step = |world: &mut World, assets: &mut Assets| {
        let mut events = Recorded::default();
        world.move_actor(0, assets, &mut events).unwrap();
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, assets, &mut events).unwrap();
        world.animate_actor(0, assets).unwrap();
        let items = world.update_items(assets, &mut events).unwrap();
        let effects = world.update_effects(assets).unwrap();
        world.finish_tick(assets, 6).unwrap();
        (items, effects)
    };
    for tick in 0..950 {
        let (items, effects) = step(&mut world, &mut assets);
        if let Some(copy) = &mut restored {
            let (other_items, other_effects) = step(copy, &mut restored_assets);
            assert_eq!(items.quads.len(), other_items.quads.len());
            assert_eq!(effects.ribbons, other_effects.ribbons);
        } else if world.script == 2 && world.actors[0].actor.state == State::Normal {
            let mut observed = world.clone();
            observed.actors[guard].slot = -(observed.tick as i16) & 31;
            observed
                .decide_actor(guard, [0; 2], &assets, &mut Recorded::default())
                .unwrap();
            assert_eq!(
                observed.ai.visibility,
                tenchu_core::tenchu2::ai::sight::Visibility::Hidden
            );
            observed.actors[0].actor.inventory.selected = Some(Item::try_from(6).unwrap());
            observed.actors[0].actor.inventory.counts[6] = 2;
            observed.actors[0].actor.pad.update(ITEM);
            let mut blocked = Recorded::default();
            observed
                .control_actor(0, &mut assets, &mut blocked)
                .unwrap();
            assert_eq!(observed.actors[0].actor.inventory.counts[6], 2);
            assert!(blocked.0.iter().any(|event| matches!(
                event,
                Event::ItemSound(tenchu_core::tenchu2::control::ItemSound::Unavailable)
            )));
            restored = Some(postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap());
        }
        translucent |= world.instances[0].translucent;
        veil |= !items.quads.is_empty();
        ribbon |= !effects.ribbons.is_empty();
        if tick == 900 {
            assert_eq!(world.script, 2);
        }
    }
    assert!(translucent && veil && ribbon);
    assert_eq!(world.script, 0);
    assert!(!world.instances[0].translucent);
    assert_eq!(world.actors[0].actor.inventory.counts[13], 1);
    assert!(world.items.entries.is_empty());
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored.unwrap()).unwrap()
    );
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual fire-breath input, expanding blasts, guard damage and expiry"]
fn native_fire_breath_hurts_a_guard_and_expires_without_harming_its_owner() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.position.z = -2400;
    world.actors[guard].actor.previous = world.actors[guard].actor.position;
    world.publish(guard, &assets);
    let health = world.actors[guard].actor.health;
    world.actors[0].actor.inventory.counts[17] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(17).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut casts = 0;
    let mut blasts = 0;
    let mut particles = false;
    let mut lit = false;
    for _ in 0..150 {
        for index in [0, guard] {
            world.move_actor(index, &assets, &mut events).unwrap();
            world.actors[index].actor.pad.update(0);
            world
                .control_actor(index, &mut assets, &mut events)
                .unwrap();
            world.animate_actor(index, &assets).unwrap();
        }
        world.update_items(&assets, &mut events).unwrap();
        casts += world
            .items
            .entries
            .iter()
            .filter(|e| matches!(e.state, ItemState::FireBreath(_)))
            .count();
        blasts += world
            .items
            .entries
            .iter()
            .filter(|e| matches!(e.state, ItemState::FireBlast(_)))
            .count();
        let effects = world.update_effects(&assets).unwrap();
        particles |= !effects.sprites.is_empty();
        lit |= !effects.emitters.is_empty();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(casts > 0 && blasts > 0 && particles && lit);
    assert!(world.actors[guard].actor.health < health);
    assert_eq!(world.actors[0].actor.health, 100);
    assert_eq!(world.actors[0].actor.inventory.counts[17], 1);
    assert!(world.items.entries.is_empty());
    assert_eq!(
        events
            .0
            .iter()
            .filter(|e| matches!(e, Event::SoundPosition { id: 51, .. }))
            .count(),
        1
    );
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual powder use, guard confusion and inventory consumption"]
fn native_confusion_powder_affects_nearby_guards_without_health_damage() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    let health = world.actors[guard].actor.health;
    world.actors[0].actor.inventory.counts[14] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(14).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut created = false;
    let mut confused = false;
    let mut particles = false;
    for _ in 0..90 {
        for index in [0, guard] {
            world.move_actor(index, &assets, &mut events).unwrap();
            world.actors[index].actor.pad.update(0);
            world
                .control_actor(index, &mut assets, &mut events)
                .unwrap();
            world.animate_actor(index, &assets).unwrap();
        }
        created |= world
            .items
            .entries
            .iter()
            .any(|entry| matches!(entry.state, ItemState::Powder(_)));
        confused |= world.actors[guard].actor.flags & 0x400 != 0;
        world.update_items(&assets, &mut events).unwrap();
        let frame = world.update_effects(&assets).unwrap();
        particles |= frame.sprites.iter().any(|sprite| sprite.image == 22)
            && world
                .effects
                .entries
                .iter()
                .any(|effect| matches!(effect, Effect::Points(points) if points.blend == 5));
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(
        created && confused && particles,
        "created={created}, confused={confused}, particles={particles}"
    );
    assert_eq!(world.actors[guard].actor.health, health);
    assert_eq!(world.actors[0].actor.inventory.counts[14], 1);
    assert!(world.items.entries.is_empty());
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual sleeping-bomb use, gas contact and snapshot continuation"]
fn native_sleeping_bomb_puts_guards_to_sleep_without_health_damage() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.position = Position::new(10000, 0, -10000);
    world.actors[guard].actor.previous = world.actors[guard].actor.position;
    world.publish(guard, &assets);
    world.finish_tick(&assets, 6).unwrap();
    let health = world.actors[guard].actor.health;
    world.actors[0].actor.inventory.counts[18] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(18).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut restored: Option<World> = None;
    let (_, mut restored_assets, _) = fixture();
    let mut asleep = false;
    let mut gas = false;
    let step = |world: &mut World, assets: &mut Assets, events: &mut Recorded| {
        for index in [0, guard] {
            world.move_actor(index, assets, events).unwrap();
            world.actors[index].actor.pad.update(0);
            world.control_actor(index, assets, events).unwrap();
            world.animate_actor(index, assets).unwrap();
        }
        world.update_items(assets, events).unwrap();
        let frame = world.update_effects(assets).unwrap();
        world.finish_tick(assets, 6).unwrap();
        frame
    };
    for _ in 0..240 {
        let frame = step(&mut world, &mut assets, &mut events);
        if let Some(copy) = &mut restored {
            let repeated = step(copy, &mut restored_assets, &mut Recorded::default());
            assert_eq!(frame.sprites, repeated.sprites);
        } else if let Some(position) = world.items.entries.iter().find_map(|entry| {
            if let ItemState::SleepBomb(bomb) = &entry.state
                && bomb.age > 0
            {
                Some(bomb.flight.position)
            } else {
                None
            }
        }) {
            world.actors[guard].actor.position = position;
            world.actors[guard].actor.previous = position;
            world.publish(guard, &assets);
            restored = Some(postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap());
        }
        gas |= frame.sprites.iter().any(|sprite| sprite.image == 22);
        asleep |= world.actors[guard].actor.motion.action == 0xa13;
    }
    assert!(
        asleep && gas,
        "asleep={asleep}, gas={gas}, events={:?}",
        events.0
    );
    assert_eq!(world.actors[guard].actor.health, health);
    assert_eq!(world.actors[0].actor.inventory.counts[18], 1);
    assert!(world.items.entries.is_empty());
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored.unwrap()).unwrap()
    );
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; water quenches grenades into recoverable items, surviving snapshot restore"]
fn native_grenade_dud_can_be_retrieved_after_a_snapshot() {
    use tenchu_core::tenchu2::items::{
        State as ItemState, flight::Flight, grenade::Grenade, projectile::Phase,
    };
    let (mut world, assets, guard) = fixture();
    world.actors[guard].actor.flags |= 8;
    for &joint in &world.instances[guard].rig.joints {
        world.collision.remove(joint);
    }
    let grenade = Grenade::launch(
        0,
        Flight {
            position: Position::new(4000, -100, 0),
            gravity: 16,
            submerged: true,
            ..Default::default()
        },
    );
    world
        .items
        .insert(
            ItemState::Grenade(grenade),
            &mut world.scene,
            &mut world.collision,
        )
        .unwrap();
    let mut events = Recorded::default();
    for _ in 0..10 {
        world.update_items(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(
        matches!(world.items.entries[0].state, ItemState::Grenade(ref g) if g.phase == Phase::Resting)
    );
    assert!(
        !events
            .0
            .iter()
            .any(|e| matches!(e, Event::SoundPosition { id: 51, .. }))
    );
    let mut triggered = world.clone();
    let blast = triggered.scene.add(
        None,
        tenchu_core::math::Matrix {
            translation: Position::new(4000, 0, 0),
            ..Default::default()
        },
    );
    let slot = triggered.collision.insert(blast).unwrap();
    slot.flags = 0x5106;
    slot.size = Vector3::new(1000, 1000, 1000);
    triggered.finish_tick(&assets, 6).unwrap();
    triggered.update_items(&assets, &mut events).unwrap();
    assert!(matches!(triggered.items.entries[0].state,
        ItemState::Grenade(ref g) if g.phase == Phase::Flying && g.age == 256));
    assert!(
        events
            .0
            .iter()
            .any(|e| matches!(e, Event::SoundPosition { id: 51, .. }))
    );
    for _ in 0..4 {
        triggered.update_items(&assets, &mut events).unwrap();
        triggered.finish_tick(&assets, 6).unwrap();
    }
    assert!(triggered.items.entries.is_empty());

    let mut restored: World =
        postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap();
    for world in [&mut world, &mut restored] {
        world.actors[0].actor.position = Position::new(4000, 0, 0);
        world.publish(0, &assets);
        world.finish_tick(&assets, 6).unwrap();
        world.update_items(&assets, &mut events).unwrap();
        assert!(world.items.entries.is_empty());
        assert_eq!(world.actors[0].actor.inventory.counts[6], 1);
    }
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored).unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; real item controls scatter four caltrops that can injure guards and their owner after landing"]
fn native_caltrops_arm_on_landing_and_hit_both_sides() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.position.z = -8000;
    world.publish(guard, &assets);
    world.finish_tick(&assets, 6).unwrap();
    world.actors[0].actor.inventory.counts[5] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(5).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut maximum = 0;
    for _ in 0..120 {
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, &assets).unwrap();
        maximum = maximum.max(world.items.entries.len());
        world.update_items(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert_eq!(maximum, 4);
    assert_eq!(world.actors[0].actor.inventory.counts[5], 1);
    assert!(
        events
            .0
            .iter()
            .any(|e| matches!(e, Event::SoundPosition { id: 53, .. }))
    );
    let remaining = world.items.entries.len();
    assert!(remaining >= 2);
    for who in [guard, 0] {
        let position = world
            .items
            .entries
            .iter()
            .find_map(|entry| {
                if let ItemState::Caltrop(c) = &entry.state {
                    assert!(c.settled && c.owner.is_none());
                    Some(c.flight.position)
                } else {
                    None
                }
            })
            .unwrap();
        let health = world.actors[who].actor.health;
        world.actors[who].actor.position = position;
        world.actors[who].actor.previous = position;
        world.publish(who, &assets);
        world.finish_tick(&assets, 6).unwrap();
        world.move_actor(who, &assets, &mut events).unwrap();
        world.actors[who].actor.pad.update(0);
        world.control_actor(who, &mut assets, &mut events).unwrap();
        assert!(
            world.actors[who].actor.health < health,
            "caltrop did not injure actor {who}"
        );
        world.update_items(&assets, &mut events).unwrap();
    }
    assert!(world.items.entries.len() <= remaining - 2);
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual rice use, guard lure/poison, owner recovery and mid-lure snapshot continuation"]
fn native_rice_lures_a_guard_and_preserves_poison_and_pickup_rules() {
    use tenchu_core::tenchu2::{ai::Program, items::State as ItemState};
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.position = Position::new(5000, 0, -4000);
    world.actors[guard].actor.rotation.y = 0;
    world.actors[guard]
        .behavior
        .assign(0x1115, 1, 2, &mut world.random)
        .unwrap();
    world.publish(guard, &assets);
    world.actors[0].actor.inventory.counts[11] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(11).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut attracted = false;
    for _ in 0..100 {
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, &assets).unwrap();
        world.update_items(&assets, &mut events).unwrap();
        world.finish_tick(&assets, 6).unwrap();
        attracted |= world.actors[guard].behavior.bait.is_some();
    }
    assert!(attracted);
    assert_eq!(
        world.actors[guard].actor.target,
        None,
        "rice {:?}; guard {:?} flags {:x}; player {:?}",
        world.items.entries,
        world.actors[guard].actor.position,
        world.actors[guard].actor.flags,
        world.actors[0].actor.position
    );
    assert_eq!(world.actors[0].actor.inventory.counts[11], 1);
    assert_eq!(world.items.entries.len(), 1);
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
    let bytes = postcard::to_allocvec(&world).unwrap();
    let mut restored: World = postcard::from_bytes(&bytes).unwrap();
    for world in [&mut world, &mut restored] {
        let held = world
            .decide_actor(guard, [0; 2], &assets, &mut events)
            .unwrap();
        assert_ne!(held, 0, "guard should approach the rice behind him");
        for _ in 0..10 {
            world.update_items(&assets, &mut events).unwrap();
            world.finish_tick(&assets, 6).unwrap();
        }
    }
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored).unwrap()
    );
    let position = match &world.items.entries[0].state {
        ItemState::Rice(rice) => rice.flight.position,
        _ => panic!("missing rice"),
    };
    // The owner recovers it; an enemy eats it, with the source's map-12 exception.
    for (who, map) in [(0, 1), (guard, 1), (guard, 12)] {
        let mut world = restored.clone();
        world.map = map;
        world.actors[who].actor.position = position;
        world.actors[who].actor.previous = position;
        world.publish(who, &assets);
        world.finish_tick(&assets, 6).unwrap();
        world.update_items(&assets, &mut events).unwrap();
        assert!(world.items.entries.is_empty(), "rice not consumed by {who}");
        assert!(world.actors[guard].behavior.bait.is_none());
        assert_eq!(world.actors[guard].actor.target, Some(0));
        assert_eq!(
            world.actors[guard].behavior.programs.idle,
            Program::Stationary
        );
        if who == 0 {
            assert_eq!(world.actors[0].actor.inventory.counts[11], 2);
            assert_eq!(world.actors[0].actor.flags & 0x300, 0);
        } else {
            assert_eq!(
                world.actors[guard].actor.flags & 0x300,
                if map == 12 { 0x200 } else { 0x100 }
            );
        }
    }
    let mut shared = restored.clone();
    let second = shared
        .actors
        .iter()
        .enumerate()
        .find(|(i, a)| *i != guard && matches!(a.actor.kind & 0xf0, 0x20..=0x50))
        .unwrap()
        .0;
    for who in [guard, second] {
        let actor = &mut shared.actors[who].actor;
        actor.flags &= !8;
        actor.position = position;
        actor.previous = position;
        actor
            .change_motion(0, Velocity::Stop, &assets.resources)
            .unwrap();
        actor
            .set_body_collision(
                shared.instances[who].rig.joints[0],
                who,
                &mut shared.collision,
                true,
            )
            .unwrap();
        shared.publish(who, &assets);
    }
    shared.finish_tick(&assets, 6).unwrap();
    shared.update_items(&assets, &mut events).unwrap();
    assert!(shared.items.entries.is_empty());
    for who in [guard, second] {
        assert_ne!(
            shared.actors[who].actor.flags & 0x100,
            0,
            "simultaneous rice contact {who}"
        );
    }
    let mut alert = restored.clone();
    alert.ai.global.perception.alert.remaining = 40;
    alert
        .decide_actor(guard, [0; 2], &assets, &mut events)
        .unwrap();
    assert!(alert.actors[guard].behavior.bait.is_none());
    assert_eq!(alert.actors[guard].actor.target, Some(0));
    // Looking back at the player while distracted still counts as a detection.
    let position = restored.actors[0].actor.position;
    restored.actors[guard].actor.position = position.wrapping_add(Position::new(0, 0, 1500));
    restored.actors[guard].actor.rotation.y = 0;
    restored.tick = (32i32 - i32::from(restored.actors[guard].slot)) as u32 & 31;
    restored
        .decide_actor(guard, [0; 2], &assets, &mut events)
        .unwrap();
    assert!(restored.actors[guard].behavior.bait.is_none());
    assert_eq!(restored.actors[guard].actor.target, Some(0));
    assert_eq!(restored.ai.detections, 1);
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual teleport use, moving target, paper effect and snapshot continuation"]
fn native_teleport_moves_behind_its_target_after_the_delay() {
    use tenchu_core::tenchu2::{effects::surface::Image, items::State as ItemState};
    let (mut world, mut assets, guard) = fixture();
    world.actors[guard].actor.position.z = -8000;
    world.actors[guard].actor.rotation.y = 1024;
    world.actors[guard].actor.flags |= 0x100000;
    world.publish(guard, &assets);
    world.actors[0].actor.inventory.counts[16] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(16).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut restored: Option<World> = None;
    let (_, mut restored_assets, _) = fixture();
    let mut paper = false;
    let mut teleported = false;
    let step = |world: &mut World, assets: &mut Assets| {
        let mut events = Recorded::default();
        world.move_actor(0, assets, &mut events).unwrap();
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, assets, &mut events).unwrap();
        world.animate_actor(0, assets).unwrap();
        world.update_items(assets, &mut events).unwrap();
        let frame = world.update_effects(assets).unwrap();
        world.finish_tick(assets, 6).unwrap();
        assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
        frame
    };
    for _ in 0..240 {
        let due = world
            .items
            .entries
            .iter()
            .any(|e| matches!(&e.state, ItemState::Teleport(t) if t.age == 95));
        let frame = step(&mut world, &mut assets);
        paper |= frame.quads.iter().any(|q| q.image == Image::Item(64));
        if due {
            assert_eq!(
                world.actors[0].actor.position,
                Position::new(1024, 0, -6000)
            );
            assert_eq!(world.actors[0].actor.rotation.y, 1024);
            assert_eq!(world.camera.mode, 21);
            teleported = true;
        } else if !teleported {
            assert!(
                world.actors[0].actor.position.z > -2000,
                "teleported before the delay"
            );
        }
        if let Some(copy) = &mut restored {
            let other = step(copy, &mut restored_assets);
            assert_eq!(frame.quads.len(), other.quads.len());
        } else if world
            .items
            .entries
            .iter()
            .any(|e| matches!(&e.state, ItemState::Teleport(t) if t.age == 40))
        {
            world.actors[guard].actor.position.z = -6000;
            world.publish(guard, &assets);
            restored = Some(postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap());
        }
    }
    assert!(
        teleported && paper,
        "teleported={teleported}, paper={paper}"
    );
    assert_eq!(world.actors[0].actor.inventory.counts[16], 1);
    assert!(world.items.entries.is_empty());
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored.unwrap()).unwrap()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual coloured-rice use, owner-only pickup, affliction boundary and snapshot continuation"]
fn native_marker_pickup_preserves_owner_and_return_point_rules() {
    use tenchu_core::tenchu2::items::State as ItemState;
    let (mut world, mut assets, guard) = fixture();
    world.controls.unlocked_items[12] = false;
    world.actors[guard].actor.position = Position::new(-10000, 0, -10000);
    world.publish(guard, &assets);
    world.actors[0].actor.inventory.counts[12] = 2;
    world.actors[0].actor.inventory.selected = Some(Item::try_from(12).unwrap());
    world.actors[0].actor.pad.update(ITEM);
    let mut events = Recorded::default();
    world.control_actor(0, &mut assets, &mut events).unwrap();
    let mut marker = None;
    for _ in 0..150 {
        world.move_actor(0, &assets, &mut events).unwrap();
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, &assets).unwrap();
        if marker.is_none()
            && let Some(entry) = world
                .items
                .entries
                .iter()
                .find(|e| matches!(e.state, ItemState::Marker(_)))
        {
            let ItemState::Marker(value) = &entry.state else {
                unreachable!()
            };
            assert_eq!(value.color, 0);
            marker = Some((entry.model, value.position));
            world.actors[0].actor.position = Position::new(10000, 0, 10000);
            world.actors[0].actor.previous = world.actors[0].actor.position;
            world.publish(0, &assets);
        }
        world.update_items(&assets, &mut events).unwrap();
        world.update_effects(&assets).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    let (model, position) = marker.expect("marker never appeared");
    assert_eq!(world.items.markers[0], Some(model));
    assert_eq!(world.actors[0].actor.inventory.counts[12], 1);
    assert!(world.items.entries.iter().any(|e| e.model == model));
    world.actors[guard].actor.position = position;
    world.publish(guard, &assets);
    world.finish_tick(&assets, 6).unwrap();
    world.update_items(&assets, &mut events).unwrap();
    assert!(
        world.items.entries.iter().any(|e| e.model == model),
        "another actor took the marker"
    );
    world.actors[guard].actor.position = Position::new(-10000, 0, -10000);
    world.publish(guard, &assets);
    world.actors[0].actor.position = position;
    world.actors[0].actor.flags |= 0x100;
    world.publish(0, &assets);
    world.finish_tick(&assets, 6).unwrap();
    world.update_items(&assets, &mut events).unwrap();
    assert!(world.items.entries.iter().any(|e| e.model == model));
    assert_eq!(
        world.items.markers[0], None,
        "afflicted owner must still clear the source return point"
    );
    world.actors[0].actor.flags &= !0x100;
    let mut restored: World =
        postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap();
    let mut restored_events = Recorded::default();
    events.0.clear();
    world.update_items(&assets, &mut events).unwrap();
    restored
        .update_items(&assets, &mut restored_events)
        .unwrap();
    assert_eq!(world.actors[0].actor.inventory.counts[12], 2);
    assert!(world.items.entries.is_empty());
    assert!(
        !world.controls.unlocked_items[12],
        "pickup notice must not unlock campaign equipment"
    );
    assert!(events.0.iter().any(|e| matches!(e, Event::Pickup(12))));
    assert!(
        events
            .0
            .iter()
            .any(|e| matches!(e, Event::SoundPosition { id: 7, .. }))
    );
    assert_eq!(
        postcard::to_allocvec(&world).unwrap(),
        postcard::to_allocvec(&restored).unwrap()
    );
    assert_eq!(
        format!("{:?}", events.0),
        format!("{:?}", restored_events.0)
    );
    assert!(!events.0.iter().any(|e| matches!(e, Event::UseItem { .. })));
}

#[test]
#[ignore = "requires TENCHU2_DISC; fatal hit, resurrection marker/home, log, fade, loading hold and snapshot continuation"]
fn native_world_resurrection_returns_control_after_the_camera_transition() {
    use tenchu_core::{math::Matrix, tenchu2::items::State as ItemState};
    for marker in [false, true] {
        let (mut world, mut assets, guard) = fixture();
        let home = Position::new(12000, 0, 6000);
        let destination = if marker {
            Position::new(-9000, 0, 4000)
        } else {
            home
        };
        world.actors[0].behavior.home = home;
        if marker {
            world.items.markers[0] = Some(world.scene.add(
                None,
                Matrix {
                    translation: destination,
                    ..Default::default()
                },
            ));
        }
        let enemy = &mut world.actors[guard].actor;
        enemy
            .set_weapon(
                &mut world.instances[guard].rig,
                guard,
                &mut world.collision,
                &assets.resources,
                WeaponMode::Draw,
            )
            .unwrap();
        enemy
            .change_motion(0x900, Velocity::Motion, &assets.resources)
            .unwrap();
        enemy.attack.index = assets
            .resources
            .weapons
            .attacks
            .iter()
            .position(|a| Some(a.clip) == enemy.motion.clip);
        assert!(enemy.attack.index.is_some());
        world.finish_tick(&assets, 6).unwrap();
        let weapon = world
            .collision
            .slots()
            .iter()
            .position(|s| s.owner == Some(guard) && s.flags & 0x4000 != 0)
            .unwrap();
        let player = &mut world.actors[0].actor;
        player.health = 1;
        player.flags |= HIT | 0x100;
        player.hit = Some(weapon);
        player.inventory.resurrections = 1;
        let mut events = Recorded::default();
        world.control_actor(0, &mut assets, &mut events).unwrap();
        let player = &world.actors[0].actor;
        assert_eq!(player.inventory.resurrections, 0);
        assert_eq!(player.health, player.max_health);
        assert_eq!(player.flags & 0xf00, 0);
        assert_eq!(player.position, destination);
        assert_eq!(player.previous, destination);
        assert_eq!(world.script, 3);
        assert_eq!(world.camera.mode, 22);
        assert!(world.environment.background_only);
        assert!(world.instances[0].translucent);
        assert!(
            events
                .0
                .iter()
                .any(|e| matches!(e, Event::SoundPosition { id: 52, .. }))
        );
        assert!(!events.0.contains(&Event::DeathMusic));
        assert!(
            world
                .items
                .entries
                .iter()
                .any(|e| matches!(e.state, ItemState::Resurrection(_)))
        );
        world.actors[guard].actor.position = Position::new(20000, 0, -20000);
        world.actors[guard]
            .actor
            .change_motion(0, Velocity::Stop, &assets.resources)
            .unwrap();
        world.publish(guard, &assets);
        world.actors[0].actor.flags &= !HIT;
        world.controls.transition_ready = false;
        let mut smoke = false;
        let mut full_fade = false;
        for tick in 0..125 {
            let frame = world.step([0; 2], &mut assets, &mut events).unwrap();
            smoke |=
                world.effects.entries.iter().any(
                    |e| matches!(e, Effect::Sprites(s) if matches!(s.motion, Motion::Dispersing)),
                );
            full_fade |= frame.fade == Some(255);
            assert_eq!(world.script, 3, "ended while loading at {tick}");
        }
        assert!(smoke && full_fade);
        assert!(!world.environment.background_only);
        assert!(!world.instances[0].translucent);
        assert_eq!(world.camera.mode, 0);
        assert_eq!(world.actors[0].actor.position, destination);
        world.controls.transition_ready = true;
        let mut restored: World =
            postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap();
        let mut other_events = Recorded::default();
        events.0.clear();
        let mut fading = Vec::new();
        for _ in 0..60 {
            let frame = world.step([0; 2], &mut assets, &mut events).unwrap();
            let other = restored
                .step([0; 2], &mut assets, &mut other_events)
                .unwrap();
            assert_eq!(frame.fade, other.fade);
            if let Some(fade) = frame.fade {
                fading.push(fade);
            }
        }
        assert!(fading.windows(2).all(|a| a[1] <= a[0]));
        assert_eq!(fading.last(), Some(&0));
        assert_eq!(world.script, 0);
        assert!(
            !world
                .items
                .entries
                .iter()
                .any(|e| matches!(e.state, ItemState::Resurrection(_)))
        );
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        assert_eq!(events.0, other_events.0);
        let before = world.actors[0].actor.position;
        for _ in 0..20 {
            world
                .step(
                    [tenchu_core::tenchu2::input::UP, 0],
                    &mut assets,
                    &mut events,
                )
                .unwrap();
        }
        assert_ne!(
            world.actors[0].actor.position, before,
            "controls did not return"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; actual combat-death completion creates a pool at the corpse joint, never early"]
fn native_death_pool_waits_for_completed_combat_death() {
    let (mut world, mut assets, guard) = fixture();
    world.map = 1;
    let actor = &mut world.actors[guard].actor;
    actor.health = 0;
    actor.state = State::Dead;
    assert!(
        actor
            .change_motion(0xb01, Velocity::Motion, &assets.resources)
            .unwrap()
            .is_some()
    );
    actor.motion.frame = 0;
    actor.motion.loops = 0;
    let mut events = Recorded::default();
    let mut seen = false;
    for _ in 0..180 {
        world.move_actor(guard, &assets, &mut events).unwrap();
        world
            .control_actor(guard, &mut assets, &mut events)
            .unwrap();
        world.animate_actor(guard, &assets).unwrap();
        for effect in &world.effects.entries {
            if let Effect::Pool(pool) = effect {
                assert_eq!(world.actors[guard].actor.motion.loops, i16::MIN);
                let mut scene = world.scene.clone();
                let torso = scene
                    .world(world.instances[guard].rig.joints[1])
                    .unwrap()
                    .translation;
                assert!((pool.position.x - torso.x).abs() < 300);
                assert!((pool.position.z - torso.z).abs() < 300);
                assert_eq!(pool.position.y, 0);
                seen = true;
            }
        }
        world.update_effects(&assets).unwrap();
        world.finish_tick(&assets, 6).unwrap();
    }
    assert!(seen, "combat death never produced a source pool");
}
