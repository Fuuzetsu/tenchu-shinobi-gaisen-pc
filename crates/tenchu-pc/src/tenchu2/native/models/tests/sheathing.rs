use super::*;
use crate::tenchu2::native::Session;
use tenchu_core::tenchu2::{actor::Velocity, ai::Program};

#[test]
#[ignore = "requires both discs; T1 hero overrides retain the native sheathed idle and selected draw/attack motions"]
fn native_t1_heroes_keep_sheathed_idle_fallback() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&gaisen).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    for hero in 0..3 {
        for source in ["disc:0", "disc:1"] {
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
            let idle = session
                .assets
                .resources
                .registry
                .clip(hero as i16, 0)
                .unwrap();
            let settings = Settings {
                player_model: source.into(),
                model_moveset: true,
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
            assert_eq!(
                session.world.actors[player].actor.motion.clip,
                Some(idle),
                "host {hero}, selected {source}"
            );
            assert!(!session.world.actors[player].actor.weapon_drawn());
            let saved = session.capture_state().unwrap();
            if hero == 0
                && source == "disc:1"
                && let Some(directory) = std::env::var_os("TENCHU_SHEATHED_REPORT")
            {
                let directory = std::path::PathBuf::from(directory);
                std::fs::create_dir_all(&directory).unwrap();
                capture(&session, &donor, &directory.join("t2-t1-ayame-after.png"));
                // Reconstruct the old import: T1's neutral pose replaced T2's idle.
                let body = library.get(source).unwrap();
                let id = body
                    .character
                    .as_ref()
                    .unwrap()
                    .motions
                    .iter()
                    .find(|m| m.action == 0)
                    .unwrap()
                    .clip;
                let clip = body.clips.iter().find(|c| c.id == id).unwrap();
                session
                    .assets
                    .resources
                    .clips
                    .insert(-32000, gaisen_clip(clip, -32000));
                let definition = session.world.actors[player].actor.definition_kind();
                session
                    .assets
                    .resources
                    .registry
                    .actors
                    .iter_mut()
                    .find(|a| a.kind == definition)
                    .unwrap()
                    .motions
                    .iter_mut()
                    .find(|m| m.action == 0)
                    .unwrap()
                    .clip = -32000;
                let actor = &mut session.world.actors[player].actor;
                actor.motion.action = -1;
                actor
                    .change_motion(0, Velocity::Stop, &session.assets.resources)
                    .unwrap();
                for _ in 0..30 {
                    session.step(0).unwrap();
                }
                capture(&session, &donor, &directory.join("t2-t1-ayame-before.png"));
                session.restore_state(&saved).unwrap();
            }
            session.step(ATTACK).unwrap();
            for _ in 0..120 {
                session.step(0).unwrap();
            }
            assert!(session.world.actors[player].actor.weapon_drawn());
            assert_eq!(session.world.actors[player].actor.motion.action, 0x100);
            session.step(ATTACK).unwrap();
            assert_eq!(session.world.actors[player].actor.state, State::Attack);
            assert!(session.world.actors[player].actor.motion.clip.unwrap() < 0);
            session.restore_state(&saved).unwrap();
            assert_eq!(session.world.actors[player].actor.motion.clip, Some(idle));
        }
    }
}

fn capture(session: &Session, donor: &Game, path: &std::path::Path) {
    use glam::{Mat4, Vec3};
    let graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    let mut review = crate::capture::review::Review::new(&donor.mission).unwrap();
    review.configure(graphics.clone());
    review.renderer.reset_vram(&session.assets.level.vram);
    let models = crate::tenchu2::native::render::Models::new(
        &session.assets,
        &session.world,
        &graphics,
        &mut review.renderer,
    )
    .unwrap();
    let geometry = models
        .geometry(
            &session.world,
            &session.assets,
            &session.frame,
            &graphics,
            Default::default(),
        )
        .unwrap();
    let actor = &session.world.actors[session.world.player].actor;
    let center = Vec3::from(actor.position.to_array().map(|v| v as f32)) - Vec3::Y * 900.0;
    let offset =
        session
            .assets
            .resources
            .math
            .velocity(actor.rotation.y.wrapping_add(800), -3200, 0);
    let eye = center + Vec3::from(offset.map(f32::from)) - Vec3::Y * 300.0;
    let camera = Mat4::perspective_rh(50f32.to_radians(), 16.0 / 9.0, 10.0, 100000.0)
        * Mat4::look_at_rh(eye, center, Vec3::NEG_Y);
    let actors = review.renderer.upload(&geometry.actors);
    let terrain = review.renderer.upload(&geometry.terrain);
    let props = review.renderer.upload(&geometry.props);
    review
        .capture(camera, &[&terrain, &props, &actors], None, false, path)
        .unwrap();
}
