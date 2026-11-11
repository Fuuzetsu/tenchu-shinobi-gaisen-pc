use super::*;
use crate::tenchu2::native::{Output, Session};
use tenchu_core::tenchu2::world::controls::{Event, EventSource, Events};

#[test]
#[ignore = "requires both discs; independent native equipment retains attack contacts and separates weapon/voice audio"]
fn independent_equipment_and_voice_in_native_t2() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = donor(&gaisen).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let owner = session.world.player;
    let bodies = Arc::default();
    for moves in ["", "tenchu2-jp:ayame", "disc:0"] {
        let mut settings = Settings {
            model_moveset: !moves.is_empty(),
            moveset_model: moves.into(),
            ..Default::default()
        };
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &bodies,
                &settings,
            )
            .unwrap();
        let actor = &session.world.actors[owner].actor;
        let definition = session
            .assets
            .resources
            .registry
            .actor(actor.definition_kind())
            .unwrap();
        let registrations = postcard::to_allocvec(&definition.motions).unwrap();
        let contacts = session
            .assets
            .resources
            .weapons
            .parts(definition.weapon)
            .filter(|p| p.radius > 0)
            .map(|p| (p.bone, p.radius, p.center, p.trail))
            .collect::<Vec<_>>();
        assert!(!contacts.is_empty());
        let original = actor.kind;
        let rng = session.world.random;
        for selected in [
            "tenchu2-jp:kagami",
            "tenchu2-jp:yorojyu",
            "tenchu2-jp:tatu2",
            "disc:1",
        ] {
            let position = session.world.actors[owner].actor.position;
            let motion = postcard::to_allocvec(&session.world.actors[owner].actor.motion).unwrap();
            settings.weapon_model = selected.into();
            settings.voice_model = "tenchu2-jp:ayame".into();
            assert!(
                session
                    .models
                    .sync(
                        &mut session.world,
                        &mut session.assets,
                        &mut donor,
                        &library,
                        &bodies,
                        &settings
                    )
                    .unwrap()
            );
            assert!(
                !session
                    .models
                    .sync(
                        &mut session.world,
                        &mut session.assets,
                        &mut donor,
                        &library,
                        &bodies,
                        &settings
                    )
                    .unwrap()
            );
            assert_eq!(session.world.random, rng);
            let actor = &session.world.actors[owner].actor;
            assert_eq!(actor.kind, original);
            assert_eq!(actor.position, position);
            assert!(motion == postcard::to_allocvec(&actor.motion).unwrap());

            let definition = session
                .assets
                .resources
                .registry
                .actor(actor.definition_kind())
                .unwrap();
            assert!(registrations == postcard::to_allocvec(&definition.motions).unwrap());
            let current = session
                .assets
                .resources
                .weapons
                .parts(definition.weapon)
                .filter(|p| p.radius > 0)
                .map(|p| (p.bone, p.radius, p.center, p.trail))
                .collect::<Vec<_>>();
            assert_eq!(current, contacts);
            let source = library.weapon(selected).unwrap().unwrap();
            let equipment = source.equipment.as_ref().unwrap();
            assert!(!equipment.attachments.is_empty());
            assert_eq!(
                session.assets.weapon_imports.len(),
                equipment.attachments.len()
            );
            for attachment in &equipment.attachments {
                assert!(
                    session
                        .assets
                        .weapon_imports
                        .values()
                        .any(|m| m.id == attachment.model.id)
                );
            }
            for drawn in [true, false] {
                let actor = &mut session.world.actors[owner].actor;
                actor
                    .set_weapon(
                        &mut session.world.instances[owner].rig,
                        owner,
                        &mut session.world.collision,
                        &session.assets.resources,
                        if drawn {
                            WeaponMode::Draw
                        } else {
                            WeaponMode::Sheathe
                        },
                    )
                    .unwrap();
                assert_eq!(actor.weapon_drawn(), drawn);
            }
            let listener = session.world.listener();
            let mut pending = BTreeSet::new();
            let mut output = Output {
                player: owner,
                feedback: &mut session.feedback,
                voice: session.models.voice.as_ref(),
                weapon_voice: session.models.weapon_voice.as_ref(),
                actor_voices: &session.actor_voices,
                listener,
                requests: Vec::new(),
                wounds: Vec::new(),
                melee_hits: BTreeSet::new(),
                pending: &mut pending,
            };
            for cue in [0, 1, 2, 10, 13, 15] {
                output
                    .emit(
                        EventSource {
                            listener,
                            voice: session.world.instances[owner].voice,
                            index: owner,
                            actor: &session.world.actors[owner].actor,
                            rig: &session.world.instances[owner].rig,
                            scene: &mut session.world.scene,
                        },
                        Event::Voice(cue),
                        &mut session.world.random,
                    )
                    .unwrap();
                let (bank, sound) = output.requests.pop().unwrap().source.unwrap();
                let expected = if cue <= 5 {
                    source.moveset_voice().unwrap()
                } else {
                    library.voice(&settings.voice_model).unwrap().unwrap()
                };
                assert!(Arc::ptr_eq(&bank, &expected.bank));
                assert_eq!(
                    i16::from(sound),
                    expected.cue(tenchu_assets::audio::voice::Dialect::Tenchu2, cue, false)
                );
            }
            drop(output);
            if let Some(folder) = std::env::var_os("TENCHU_MODEL_REVIEW") {
                let path = std::path::PathBuf::from(folder);
                std::fs::create_dir_all(&path).unwrap();
                session.world.actors[owner]
                    .actor
                    .set_weapon(
                        &mut session.world.instances[owner].rig,
                        owner,
                        &mut session.world.collision,
                        &session.assets.resources,
                        WeaponMode::Draw,
                    )
                    .unwrap();
                session.world.publish(owner, &session.assets);
                let mut camera =
                    crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                camera.focus(
                    glam::Vec3::from(
                        session.world.actors[owner]
                            .actor
                            .position
                            .to_array()
                            .map(|v| v as f32),
                    ) - glam::Vec3::Y * 1000.0,
                    false,
                );
                session.world.camera.view = camera.source_view();
                let capture = Settings {
                    show_hud: false,
                    graphics: crate::settings::Graphics {
                        fog: false,
                        asset_pack: false,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                crate::tenchu2::native::render::capture(
                    &session,
                    &capture,
                    &path.join(format!(
                        "weapons-{}-{}.png",
                        moves.replace(':', "-"),
                        selected.replace(':', "-")
                    )),
                )
                .unwrap();
            }
        }
        settings.weapon_model.clear();
        settings.voice_model.clear();
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &bodies,
                &settings,
            )
            .unwrap();
        let definition = session
            .assets
            .resources
            .registry
            .actor(session.world.actors[owner].actor.definition_kind())
            .unwrap();
        assert!(registrations == postcard::to_allocvec(&definition.motions).unwrap());
    }
}
