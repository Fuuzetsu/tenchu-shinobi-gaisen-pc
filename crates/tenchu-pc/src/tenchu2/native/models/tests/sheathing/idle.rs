use super::*;

#[test]
#[ignore = "requires both discs; T2 NPC bind poses use each native hero idle without changing original NPCs or authored optional moves"]
fn native_t2_placeholder_idles_use_host_fallback() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for hero in 0..3 {
        for (name, moves) in [
            ("kagami", true),
            ("toda", true),
            ("yorojyu", true),
            ("todanin", true),
            ("kagami", false),
        ] {
            let mut session = Session::with_options(
                &disc,
                9,
                tenchu_core::tenchu2::world::Options {
                    hero,
                    difficulty: 2,
                    seed: 17,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap();
            let player = session.world.player;
            for (id, agent) in session.world.actors.iter_mut().enumerate() {
                if id != player {
                    agent.behavior.programs.idle = Program::Disabled;
                }
            }
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let registry = &session.assets.resources.registry;
            let selected = format!("{}{name}", registry.edition.model_prefix());
            let body = library.get(&selected).unwrap();
            let original_idle = registry.clip(hero as i16, 0).unwrap();
            let source_idle = registry
                .clip(body.native.as_ref().unwrap().kind, 0)
                .unwrap();
            let npc_definitions: Vec<_> = registry
                .actors
                .iter()
                .filter(|a| a.kind >= 3)
                .map(|a| (a.kind, postcard::to_allocvec(a).unwrap()))
                .collect();
            let settings = Settings {
                player_model: selected.clone(),
                model_moveset: moves,
                model_moveset_options: Options {
                    no_idle_taunts: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &settings,
                )
                .unwrap();
            for (kind, before) in npc_definitions {
                let after = session
                    .assets
                    .resources
                    .registry
                    .actors
                    .iter()
                    .find(|a| a.kind == kind)
                    .unwrap();
                assert_eq!(
                    postcard::to_allocvec(after).unwrap(),
                    before,
                    "original NPC {kind}"
                );
            }
            let actor = &mut session.world.actors[player].actor;
            actor
                .set_weapon(
                    &mut session.world.instances[player].rig,
                    player,
                    &mut session.world.collision,
                    &session.assets.resources,
                    WeaponMode::Sheathe,
                )
                .unwrap();
            actor
                .change_motion(0, Velocity::Stop, &session.assets.resources)
                .unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            if hero == 0
                && name == "kagami"
                && moves
                && let Some(directory) = std::env::var_os("TENCHU_SHEATHED_REPORT")
            {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                capture(&session, &donor, &directory.join("native-kagami-idle.png"));
            }
            let actor = &session.world.actors[player].actor;
            assert!(!actor.weapon_drawn());
            assert_eq!(actor.motion.action, 0);
            let actual = actor.motion.clip.unwrap();
            if !moves || source_idle == 1 {
                assert_eq!(
                    actual, original_idle,
                    "hero {hero}, {selected}, moves {moves}"
                );
            } else {
                let imported = body.imported_moveset.as_ref().unwrap();
                let id = imported
                    .character
                    .motions
                    .iter()
                    .find(|m| m.action == 0)
                    .unwrap()
                    .clip;
                let clip = imported.clips.iter().find(|c| c.id == id).unwrap();
                assert!(actual < 0);
                assert_eq!(
                    postcard::to_allocvec(&session.assets.resources.clips[&actual]).unwrap(),
                    postcard::to_allocvec(&gaisen_clip(clip, actual)).unwrap()
                );
            }
            let saved = session.capture_state().unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let continued = session.capture_state().unwrap();
            session.restore_state(&saved).unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            assert_eq!(session.capture_state().unwrap(), continued);
            session.step(ATTACK).unwrap();
            for _ in 0..120 {
                session.step(0).unwrap();
            }
            assert!(session.world.actors[player].actor.weapon_drawn());
            assert_eq!(session.world.actors[player].actor.motion.action, 0x100);
            session.step(ATTACK).unwrap();
            assert_eq!(session.world.actors[player].actor.state, State::Attack);
            if moves {
                assert!(session.world.actors[player].actor.motion.clip.unwrap() < 0);
            }
            session.restore_state(&saved).unwrap();
            assert_eq!(session.world.actors[player].actor.motion.clip, Some(actual));
        }
    }
}
