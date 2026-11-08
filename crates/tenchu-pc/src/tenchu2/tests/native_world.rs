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
        control::GrappleStatus,
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
struct Recorded(Vec<Event>);
impl Events for Recorded {
    fn emit(&mut self, _: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.0.push(event);
        Ok(())
    }
    fn grapple(&self) -> Result<(GrappleStatus, Position)> {
        Err("No grapple in this fixture".into())
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
    world.finish_tick().unwrap();
    (world, assets, guard)
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
    for _ in 0..240 {
        for i in [0, guard] {
            world.move_actor(i, &assets, |_| 0).unwrap();
            world.actors[i].actor.pad.update(0);
            world
                .control_actor(i, &mut assets, &mut events)
                .unwrap_or_else(|e| panic!("actor {i} tick {}: {e}", world.tick));
            world.animate_actor(i, true, &assets).unwrap();
        }
        world.finish_tick().unwrap();
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
    assert!(events.0.iter().any(|e| matches!(e, Event::Blood { .. })));
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
    world.finish_tick().unwrap();
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
    assert!(events.0.iter().any(|e| matches!(e, Event::Sparks { .. })));

    let player = &mut world.actors[0].actor;
    player
        .change_motion(0x100, Velocity::Stop, &assets.resources)
        .unwrap();
    player.health = 1;
    player.inventory.selected = Some(Item::try_from(1).unwrap());
    player.inventory.counts[1] = 1;
    player.pad.update(ITEM);
    world.control_actor(0, &mut assets, &mut events).unwrap();
    for _ in 0..180 {
        world.actors[0].actor.pad.update(0);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        world.animate_actor(0, true, &assets).unwrap();
        world.finish_tick().unwrap();
    }
    assert_eq!(
        world.actors[0].actor.health,
        world.actors[0].actor.max_health
    );
    assert_eq!(world.actors[0].actor.inventory.counts[1], 0);
    assert!(events.0.contains(&Event::Medicine(0)));

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
        world.animate_actor(0, true, &assets).unwrap();
        world.finish_tick().unwrap();
    }
    assert_eq!(world.actors[0].actor.health, 0);
    assert_eq!(world.actors[0].actor.state, State::Dead);
}
