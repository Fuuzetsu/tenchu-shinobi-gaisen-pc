use super::*;
use std::collections::BTreeMap;
use tenchu_assets::{
    executable::{BodyTables, Executable},
    misc::Kind,
};
use tenchu_core::{
    actor::Context,
    effects::{Effect, Environment, View},
    game::Game,
    terrain::Cursor,
};

fn kind(kind: &Kind) -> Option<&'static str> {
    Some(match kind {
        Kind::Fire => "fire",
        Kind::Puff(_) => "puff",
        Kind::Snowfall => "snow",
        Kind::Sprite(_) => "sprite",
        Kind::Bonfire(_) => "bonfire",
        Kind::Sound { .. } => "sound",
        _ => return None,
    })
}

fn original_tick(game: &mut Game, world: &World, body: &BodyTables, cursor: &mut Cursor) {
    game.clock = world.tick as i32;
    game.random = world.random;
    game.camera.position = world.camera.eye;
    game.camera.reference = world.camera.look;
    game.camera.view = world.camera.view;
    // Native World owns scrolling; retain the reserved source slots while
    // comparing the full original effect runner and its other callbacks.
    let scrolls: Vec<_> = game
        .effects
        .slots
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Some(Effect::TextureScroll(_))))
        .map(|(i, e)| (i, e.clone()))
        .collect();
    game.effects
        .step(
            &mut game.random,
            View {
                position: game.camera.position,
                matrix: game.camera.view,
            },
            &mut game.scene,
            &mut Environment {
                terrain: &game.mission.terrain,
                cursor,
                clock: game.clock,
                owner_position: world.actors[world.player].actor.position,
                reference: game.camera.reference,
            },
        )
        .unwrap();
    for (index, scroll) in scrolls {
        game.effects.slots[index] = scroll;
    }
    game.effects.texture_moves.clear();
    game.misc
        .step(
            game.clock,
            &mut Context {
                mission: &mut game.mission,
                math: &game.math,
                body,
                cursor,
                random: &mut game.random,
                effects: &mut game.effects,
                scene: &mut game.scene,
                collision: &mut game.collision,
                camera: &mut game.camera,
                drawing: &mut game.drawing,
            },
        )
        .unwrap();
}

#[test]
#[ignore = "requires both discs; compares authored ambient lifetimes, activation, sound, RNG and snapshots with original callbacks across all stages"]
fn imported_ambient_matches_original_callbacks_and_native_ownership() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let body = Executable::new(&game.executable)
        .unwrap()
        .body_tables()
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut frames = 0;
    let mut emitted = 0;
    let mut sounds = 0;
    for stage in 0..11 {
        let (initial, initial_world, _) = director::fixture(&game, &disc, stage, 0, stage % 3, 1);
        let positions: BTreeMap<_, _> = initial
            .source
            .misc
            .objects
            .iter()
            .filter_map(|o| kind(&o.kind).map(|kind| (kind, o.position)))
            .collect();
        let snapshot = postcard::to_allocvec(&initial).unwrap();
        for (kind, position) in positions {
            seen.insert(kind);
            let mut director: tenchu_core::tenchu2::world::gaisen::Director =
                postcard::from_bytes(&snapshot).unwrap();
            let mut world = initial_world.clone();
            let mut original: Game =
                postcard::from_bytes(&postcard::to_allocvec(&director.source).unwrap()).unwrap();
            let mut cursor = Cursor::new(&original.mission.terrain);
            original
                .misc
                .objects
                .retain(|o| !matches!(o.kind, Kind::Door { .. } | Kind::Pitfall { .. }));
            let hinges: Vec<_> = director
                .source
                .misc
                .objects
                .iter()
                .filter(|o| matches!(o.kind, Kind::Door { .. } | Kind::Pitfall { .. }))
                .map(|o| postcard::to_allocvec(o).unwrap())
                .collect();
            for tick in 1..=180 {
                world.tick = tick;
                world.camera.look = position;
                if (120..140).contains(&tick) {
                    world.camera.look.x += 50000;
                }
                world.camera.eye = world
                    .camera
                    .look
                    .wrapping_add(tenchu_core::math::Position::new(3000, -1500, 5000));
                original
                    .camera
                    .set_view(world.camera.eye, world.camera.look, &original.math)
                    .unwrap();
                world.camera.view = original.camera.view;
                director.source.effects.begin_tick();
                original.effects.begin_tick();
                director.source.scene.finish_tick();
                original.scene.finish_tick();
                let before = tick.is_multiple_of(20).then(|| world.clone());
                original_tick(&mut original, &world, &body, &mut cursor);
                director
                    .advance_ambient(&mut world)
                    .unwrap_or_else(|e| panic!("stage {stage} {kind} tick {tick}: {e}"));
                assert_eq!(
                    world.random.seed, original.random.seed,
                    "stage {stage} {kind} tick {tick}"
                );
                assert_eq!(
                    postcard::to_allocvec(&director.source.effects).unwrap(),
                    postcard::to_allocvec(&original.effects).unwrap(),
                    "stage {stage} {kind} tick {tick}"
                );
                let objects: Vec<_> = director
                    .source
                    .misc
                    .objects
                    .iter()
                    .filter(|o| !matches!(o.kind, Kind::Door { .. } | Kind::Pitfall { .. }))
                    .collect();
                assert_eq!(
                    postcard::to_allocvec(&objects).unwrap(),
                    postcard::to_allocvec(&original.misc.objects).unwrap()
                );
                assert_eq!(
                    postcard::to_allocvec(&director.source.misc.drawings).unwrap(),
                    postcard::to_allocvec(&original.misc.drawings).unwrap()
                );
                assert!(director.source.effects.texture_moves.is_empty());
                if let Some(mut before) = before {
                    before.random = original.random;
                    assert_eq!(
                        postcard::to_allocvec(&before).unwrap(),
                        postcard::to_allocvec(&world).unwrap()
                    );
                }
                emitted += director.source.effects.billboards.len();
                sounds += director.source.effects.sounds.len();
                frames += 1;
                if tick == 90 {
                    director =
                        postcard::from_bytes(&postcard::to_allocvec(&director).unwrap()).unwrap();
                    world = postcard::from_bytes(&postcard::to_allocvec(&world).unwrap()).unwrap();
                }
            }
            assert_eq!(
                hinges,
                director
                    .source
                    .misc
                    .objects
                    .iter()
                    .filter(|o| matches!(o.kind, Kind::Door { .. } | Kind::Pitfall { .. }))
                    .map(|o| postcard::to_allocvec(o).unwrap())
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(emitted > 1000 && sounds > 0);
    eprintln!(
        "{frames} ambient ticks over all 11 templates: {seen:?}; {emitted} billboard frames and {sounds} source cues"
    );
}

#[test]
#[ignore = "requires both discs; native host advances authored ambience and scrolling once, with current-listener sound from the original bank"]
fn imported_host_ambient_ticks_and_audio_follow_native_gameplay() {
    use tenchu_assets::script::Language;
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut cues = 0;
    for stage in 0..11 {
        let source = gaisen::Source::open(
            path.as_ref(),
            stage,
            (stage % 2) as i16,
            1,
            Language::English,
        )
        .unwrap();
        let mut session = source
            .session(
                &disc,
                Options {
                    hero: stage % 3,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
                4000,
            )
            .unwrap();
        let game = &mut session.gaisen.as_mut().unwrap().director.source;
        game.events = Default::default();
        // Keep real authored sound/fire parameters, but put their emitters near
        // the listener so this fixture tests audio rather than map traversal.
        for object in &mut game.misc.objects {
            if matches!(
                object.kind,
                Kind::Sound { .. } | Kind::Bonfire(_) | Kind::Fire
            ) {
                object.position = session.world.listener().position;
                object.position.x += 2500;
                object.paused = false;
            }
        }
        let mut scrolling = session.world.environment.texture_scrolls.clone();
        for _ in 0..180 {
            let tick = session.world.tick;
            session.step(0).unwrap();
            assert!(!session.cinematic());
            assert_eq!(session.world.tick, tick + 1);
            let scene = session.gaisen.as_ref().unwrap();
            let game = &scene.director.source;
            assert_eq!(game.clock, session.world.tick as i32);
            assert_eq!(game.random.seed, session.world.random.seed);
            assert!(game.effects.texture_moves.is_empty());
            for scroll in &mut scrolling {
                scroll.step();
            }
            assert_eq!(
                postcard::to_allocvec(&scrolling).unwrap(),
                postcard::to_allocvec(&session.world.environment.texture_scrolls).unwrap()
            );
            let listener = session.world.listener();
            let expected = game
                .audio_with_listener(
                    tenchu_core::audio::Listener {
                        position: listener.position,
                        yaw: listener.yaw,
                        direction: None,
                    },
                    game.effects.sounds.iter().copied(),
                    None,
                )
                .unwrap();
            assert_eq!(
                scene
                    .audio
                    .sounds
                    .iter()
                    .map(|s| s.sound)
                    .collect::<Vec<_>>(),
                expected.iter().map(|s| s.sound).collect::<Vec<_>>()
            );
            for sound in &scene.audio.sounds {
                assert!(!sound.bank.as_ref().unwrap().samples.is_empty());
                cues += 1;
            }
        }
    }
    assert!(cues > 0);
    eprintln!(
        "1980 native host ticks, exact single scroll advancement and {cues} original-bank ambient cues"
    );
}
