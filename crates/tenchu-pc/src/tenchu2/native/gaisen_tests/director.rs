use super::{doors::Sounds, *};
use tenchu_assets::{executable::Executable, script::Command};
use tenchu_core::{
    game::{Game, events::Outcome as SourceOutcome},
    ids::ATTR_SUSPEND,
    tenchu2::{
        actor::{State, Velocity},
        mission::{Outcome, Phase},
        world::gaisen::Director,
    },
};

fn fixture(
    game: &GameAssets<impl ReadAt>,
    disc: &Disc<impl ReadAt>,
    stage: usize,
    character: i16,
    hero: usize,
    layout: usize,
) -> (Director, World, Assets) {
    let source = Game::new(
        MissionAssets::load(game, stage, character, layout).unwrap(),
        &Executable::new(&game.executable).unwrap(),
        character,
        1,
    )
    .unwrap();
    let (assets, layout) = Assets::gaisen_campaign(disc, &source, hero, false)
        .unwrap_or_else(|e| panic!("stage {stage} character {character} hero {hero}: {e}"));
    let mut world = World::new(
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
    let director = Director::new(source, &mut world, &assets).unwrap();
    (director, world, assets)
}

#[test]
#[ignore = "requires both discs; checks complete original casts, script-hidden bosses, native health and both activation modes on every stage/layout/hero"]
fn imported_director_preserves_cast_and_hidden_actors() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    for stage in 0..11 {
        for character in 0..2 {
            for hero in 0..3 {
                for layout in 1..=3 {
                    let (director, mut world, assets) =
                        fixture(&game, &disc, stage, character, hero, layout);
                    assert_eq!(
                        director.native_actor(director.source.player_id),
                        Some(world.player)
                    );
                    let hidden: Vec<_> = director
                        .source
                        .roster
                        .order
                        .iter()
                        .filter_map(|&id| {
                            let actor = director.source.roster.get(id);
                            let Some(index) = director.native_actor(id) else {
                                assert!(matches!(actor.kind, 4..=6 | 0xa6..=0xa8));
                                return None;
                            };
                            assert_eq!(actor.max_health, world.actors[index].actor.max_health);
                            if actor.health == -1 {
                                assert_ne!(actor.attribute & ATTR_SUSPEND, 0);
                                assert!(world.script_hidden.contains(&index));
                                Some((index, postcard::to_allocvec(&world.actors[index]).unwrap()))
                            } else {
                                None
                            }
                        })
                        .collect();
                    assert_eq!(hidden.len(), world.script_hidden.len());
                    for enabled in [false, true, false] {
                        world.keep_all_active = enabled;
                        world
                            .update_activation(world.camera.eye, world.camera.look, 6, &assets)
                            .unwrap();
                        for (index, before) in &hidden {
                            assert_eq!(
                                postcard::to_allocvec(&world.actors[*index]).unwrap(),
                                *before
                            );
                            assert!(
                                world
                                    .collision
                                    .index(world.instances[*index].rig.joints[0])
                                    .is_none()
                            );
                        }
                    }
                    count += 1;
                }
            }
        }
    }
    eprintln!("{count} original stage/layout/hero casts retained script visibility");
}

fn force_next(director: &Director, world: &mut World, assets: &Assets) {
    let Some((event, target)) = director.source.events.active().next() else {
        return;
    };
    let event = &director.source.mission.scripts.events[event];
    assert!(!matches!(event.id, 2 | 3), "unexpected failure route");
    let index = director
        .native_actor(target)
        .expect("unbound gameplay event target");
    let actor = &mut world.actors[index].actor;
    match event.mode {
        0 | 8 => {}
        1 => {
            actor.position = event
                .bounds
                .map(|[low, high]| (i32::from(low) + i32::from(high)) / 2 * 1000)
                .into();
            actor.previous = actor.position;
            actor.velocity = Default::default();
        }
        3 => {
            assert_eq!(event.value, 17);
            assert!(
                !world.script_hidden.contains(&index),
                "boss was never activated"
            );
            actor.health = 0;
            let death = (0xb00..=0xb05)
                .find(|&action| {
                    assets
                        .resources
                        .registry
                        .clip(actor.definition_kind(), action)
                        .is_some()
                })
                .unwrap();
            actor
                .change_motion(death, Velocity::Stop, &assets.resources)
                .unwrap()
                .unwrap();
            actor.motion.frame = 1;
            actor.motion.loops = 0;
        }
        5 => {
            assert!(
                !world.script_hidden.contains(&index),
                "boss was never activated"
            );
            actor.health = event.value;
        }
        mode => panic!("unhandled authored event mode {mode}"),
    }
    world.scene.finish_tick();
    world.publish(index, assets);
    world.collision.compute(|id| world.scene.world(id)).unwrap();
}

fn finish_scene(director: &mut Director, world: &mut World, assets: &Assets) -> usize {
    let tick = world.tick;
    let sequence = director.source.cinematic.sequence;
    let native_before = postcard::to_allocvec(world).unwrap();
    let actors: Vec<_> = world
        .actors
        .iter()
        .map(|a| postcard::to_allocvec(a).unwrap())
        .collect();
    let (start, _) = director
        .source
        .mission
        .scripts
        .cutscenes
        .sequence(sequence)
        .unwrap();
    let affected: BTreeSet<_> = director
        .source
        .mission
        .scripts
        .cutscenes
        .commands
        .iter()
        .skip(start + 1)
        .take_while(|c| !matches!(c, Command::Wait { until: 0 }))
        .filter_map(|c| match c {
            Command::Actor { id, .. } => director.source.roster.find(*id),
            _ => None,
        })
        .filter_map(|id| director.native_actor(id))
        .collect();
    let mut elapsed = 1;
    while director.source.cinematic.active {
        assert!(elapsed < 6000, "stalled cinematic {sequence}");
        assert!(
            director
                .update(world, assets, 0)
                .unwrap_or_else(|e| panic!("sequence {sequence}, frame {elapsed}: {e}"))
        );
        assert_eq!(world.tick, tick);
        if elapsed == 20 && director.source.cinematic.active {
            assert_eq!(postcard::to_allocvec(world).unwrap(), native_before);
        }
        elapsed += 1;
    }
    for (index, before) in actors.into_iter().enumerate() {
        if !affected.contains(&index) {
            assert_eq!(
                postcard::to_allocvec(&world.actors[index]).unwrap(),
                before,
                "scene {sequence} changed unrelated native actor {index}"
            );
        }
    }
    elapsed
}

#[test]
#[ignore = "requires both discs; uses direct triggers/deaths to execute complete original mission scripts with all native heroes, without substituting a kill quota"]
fn imported_director_runs_all_original_success_routes() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut scenes = 0;
    for stage in 0..11 {
        for character in 0..2 {
            for hero in 0..3 {
                eprintln!("stage {stage}, source {character}, native {hero}");
                let (mut director, mut world, mut assets) =
                    fixture(&game, &disc, stage, character, hero, 1);
                world.keep_all_active = hero == 2;
                let mut visited = Vec::new();
                for _ in 0..60 {
                    if director.update(&mut world, &assets, 0).unwrap() {
                        let sequence = director.source.cinematic.sequence;
                        let ticks = finish_scene(&mut director, &mut world, &assets);
                        visited.push(sequence);
                        scenes += 1;
                        eprintln!("  sequence {sequence}: {ticks} original ticks");
                        continue;
                    }
                    if director.source.events.outcome != SourceOutcome::Running {
                        break;
                    }
                    let (event, target) = director.source.events.active().next().unwrap();
                    let event = &director.source.mission.scripts.events[event];
                    if event.mode == 5 {
                        let target = director.native_actor(target).unwrap();
                        world.actors[target].actor.health = event.value + 1;
                        assert!(
                            !director.update(&mut world, &assets, 0).unwrap(),
                            "health event fired above its native threshold"
                        );
                    }
                    force_next(&director, &mut world, &assets);
                }
                assert_eq!(
                    director.source.events.outcome,
                    SourceOutcome::Complete,
                    "unfinished stage {stage}, source {character}, native {hero}: {visited:?}"
                );
                assert!(
                    world.statistics.killed
                        < world.mission.population.guards + world.mission.population.bosses
                );
                world.advance_mission(&assets).unwrap();
                assert!(matches!(
                    world.mission.phase,
                    Phase::Ending {
                        outcome: Outcome::AreaCleared,
                        ..
                    }
                ));
                for _ in 0..120 {
                    world
                        .step([0; 2], &mut assets, &mut Sounds::default())
                        .unwrap();
                    world.advance_mission(&assets).unwrap();
                    if matches!(world.mission.phase, Phase::Finished { .. }) {
                        break;
                    }
                }
                assert!(matches!(
                    world.mission.phase,
                    Phase::Finished {
                        outcome: Outcome::AreaCleared,
                        ..
                    }
                ));
            }
        }
    }
    eprintln!("{scenes} complete source cinematics across 66 imported mission routes");
}

#[test]
#[ignore = "requires both discs; verifies native gameplay isolation, death-state/attack gates, and mid-scene snapshot continuation"]
fn imported_director_preserves_native_ticks_and_scene_continuations() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for hero in 0..3 {
        let (mut director, mut world, mut assets) = fixture(&game, &disc, 0, 0, hero, 1);
        let mut native = world.clone();
        for _ in 0..60 {
            assert!(!director.update(&mut world, &assets, 0).unwrap());
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            native
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&native).unwrap()
        );
        for agent in &mut world.actors {
            agent.actor.motion.frame =
                assets.resources.clips[&agent.actor.motion.clip.unwrap()].duration;
        }
        force_next(&director, &mut world, &assets);
        assert!(director.update(&mut world, &assets, 0).unwrap());
        for _ in 0..99 {
            director.update(&mut world, &assets, 0).unwrap();
        }
        let mut restored: Director =
            postcard::from_bytes(&postcard::to_allocvec(&director).unwrap()).unwrap();
        let mut restored_world: World =
            postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap();
        while director.source.cinematic.active {
            assert_eq!(
                director.update(&mut world, &assets, 0).unwrap(),
                restored.update(&mut restored_world, &assets, 0).unwrap()
            );
            assert_eq!(director.source.random.seed, restored.source.random.seed);
            assert_eq!(director.source.music, restored.source.music);
        }
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&restored_world).unwrap()
        );
        assert_eq!(
            postcard::to_allocvec(&director).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        assert!(!director.update(&mut world, &assets, 0).unwrap());
        let (_, id) = director.source.events.active().next().unwrap();
        assert_eq!(director.source.roster.get(id).kind, 0x81);
        let boss = director.native_actor(id).unwrap();
        world.actors[boss].actor.health = 0;
        world.actors[boss].actor.state = State::Hurt;
        assert!(
            !director.update(&mut world, &assets, 0).unwrap(),
            "zero HP alone must not trigger a death-status event"
        );
        world.actors[boss].actor.state = State::Dead;
        world.actors[boss].actor.motion.loops = 0;
        world.actors[boss].actor.motion.frame = 1;
        world.actors[world.player].actor.state = State::Attack;
        assert!(
            !director.update(&mut world, &assets, 0).unwrap(),
            "the authored gate waits for the player's attack"
        );
        world.actors[world.player].actor.state = State::Ready;
        assert!(director.update(&mut world, &assets, 0).unwrap());
        assert_eq!(director.source.cinematic.sequence, 220);
    }
}

#[test]
#[ignore = "requires both discs; preserves original death waits before native mission failure on every stage and hero"]
fn imported_director_death_waits_precede_native_failure() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for stage in 0..11 {
        for character in 0..2 {
            for hero in 0..3 {
                let (mut director, mut world, assets) =
                    fixture(&game, &disc, stage, character, hero, 1);
                let player = &mut world.actors[world.player].actor;
                player.health = 0;
                player.state = State::Dead;
                player.motion.loops = i16::MIN;
                player.motion.frame = 0;
                player.velocity = Default::default();
                let mut resolved = false;
                for tick in 0..102 {
                    assert!(!director.update(&mut world, &assets, 0).unwrap());
                    world.advance_mission(&assets).unwrap();
                    if director.source.events.outcome == SourceOutcome::GameOver {
                        assert_eq!(
                            tick, 100,
                            "stage {stage}, source {character}, native {hero}"
                        );
                        assert!(matches!(
                            world.mission.phase,
                            Phase::Ending {
                                outcome: Outcome::Dead,
                                ..
                            }
                        ));
                        resolved = true;
                        break;
                    }
                    assert_eq!(world.mission.phase, Phase::Playing { pending: None });
                }
                assert!(resolved, "stage {stage}, source {character}, native {hero}");
            }
        }
    }
}

#[test]
#[ignore = "requires both discs; verifies original boss-specific player death cinematics before native failure, for both protagonists and all native heroes"]
fn imported_director_finishes_authored_failure_cinematics() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for character in 0..2 {
        for hero in 0..3 {
            for boss in [0x81, 0x80] {
                let (mut director, mut world, assets) =
                    fixture(&game, &disc, 0, character, hero, 1);
                let mut reached = false;
                for _ in 0..30 {
                    if director.update(&mut world, &assets, 0).unwrap() {
                        finish_scene(&mut director, &mut world, &assets);
                        continue;
                    }
                    let (event, target) = director.source.events.active().next().unwrap();
                    if director.source.mission.scripts.events[event].mode == 3
                        && director.source.roster.get(target).kind == boss
                    {
                        reached = true;
                        break;
                    }
                    force_next(&director, &mut world, &assets);
                }
                assert!(reached);
                let player = &mut world.actors[world.player].actor;
                let death = (0xb00..=0xb05)
                    .find(|&action| {
                        assets
                            .resources
                            .registry
                            .clip(player.definition_kind(), action)
                            .is_some()
                    })
                    .unwrap();
                player.health = 0;
                player
                    .change_motion(death, Velocity::Stop, &assets.resources)
                    .unwrap()
                    .unwrap();
                player.motion.frame = 31;
                player.motion.loops = 0;
                assert!(director.update(&mut world, &assets, 0).unwrap());
                assert_eq!(
                    director.source.cinematic.sequence,
                    if boss == 0x81 { 250 } else { 152 }
                );
                finish_scene(&mut director, &mut world, &assets);
                assert_eq!(world.mission.phase, Phase::Playing { pending: None });
                assert!(!director.update(&mut world, &assets, 0).unwrap());
                assert_eq!(director.source.events.outcome, SourceOutcome::GameOver);
                world.advance_mission(&assets).unwrap();
                assert!(matches!(
                    world.mission.phase,
                    Phase::Ending {
                        outcome: Outcome::Dead,
                        ..
                    }
                ));
            }
        }
    }
}
