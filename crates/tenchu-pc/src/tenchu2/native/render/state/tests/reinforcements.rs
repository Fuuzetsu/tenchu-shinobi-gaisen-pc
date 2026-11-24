use super::*;
use crate::tenchu2::native::behavior_tests as fixture;
use tenchu_core::{
    ai::profiles::{Combat, Settings},
    roster::ReinforcementCall,
};

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; retained renderer handles optional aid, repeated calls and restores without changing gameplay"]
fn native_reinforcements_keep_live_weapon_bindings() {
    let out = output();
    for preset in [0, 8, 9] {
        for count in [1, 32] {
            let (mut session, guard) = fixture::fixture();
            fixture::cover(&mut session);
            fixture::pending(&mut session, guard);
            session.world.configure_profiles(
                Settings {
                    enabled: true,
                    combat: Combat::Mixed,
                    seed: 1,
                    ..Default::default()
                },
                &session.assets,
            );
            assert_eq!(
                session.world.profile_selection(guard).unwrap().combat,
                Combat::Coward
            );
            session.world.configure_reinforcements(
                ReinforcementCall {
                    count,
                    ..Default::default()
                },
                60,
            );
            let population = session.world.actors.len();
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.set_preset(preset);
            session.interpolate(graphics.interpolation).unwrap();
            let vram = session.assets.sprites.vram.clone();
            let path = out.join(format!("reinforcements-{preset}-{count}.png"));
            capture_frame(&vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                drawing.camera = Some(
                    crate::demo::framing::Path {
                        eye: [0.0, -4000.0, 14000.0],
                        target: [0.0, -700.0, 22000.0],
                        end_eye: None,
                        end_target: None,
                        follow_player: false,
                        fov: 55.0,
                    }
                    .sample(0.0),
                );
                let before_call = session.capture_state()?;
                let before_drawing = postcard::to_allocvec(&drawing.capture_state()).unwrap();
                session.step(0)?;
                assert_eq!(session.world.actors.len(), population + usize::from(count));
                let called = session.capture_state()?;
                drawing.tick(&session, &graphics)?;
                let called_drawing = postcard::to_allocvec(&drawing.capture_state()).unwrap();
                for tick in 0..=12 {
                    if tick != 0 {
                        session.step(0)?;
                    }
                    let before = postcard::to_allocvec(&session.world).unwrap();
                    for alpha in [0.25, 1.0] {
                        drawing.alpha = alpha;
                        drawing.update(&session, renderer, None)?;
                    }
                    assert!(postcard::to_allocvec(&session.world).unwrap() == before);
                }

                session.restore_state(&before_call)?;
                drawing.restore_state(
                    postcard::from_bytes(&before_drawing).unwrap(),
                    &session,
                    renderer,
                )?;
                drawing.update(&session, renderer, None)?;
                session.step(0)?;
                assert!(session.capture_state()? == called);
                drawing.update(&session, renderer, None)?;

                session.restore_state(&called)?;
                drawing.restore_state(
                    postcard::from_bytes(&called_drawing).unwrap(),
                    &session,
                    renderer,
                )?;
                drawing.update(&session, renderer, None)?;
                fixture::pending(&mut session, guard);
                session.step(0)?;
                assert_eq!(
                    session.world.actors.len(),
                    (population + 2 * usize::from(count)).min(60)
                );
                let before = postcard::to_allocvec(&session.world).unwrap();
                drawing.update(&session, renderer, None)?;
                assert!(postcard::to_allocvec(&session.world).unwrap() == before);
                Ok((
                    drawing.scene,
                    drawing.camera.unwrap().source_view(),
                    drawing.texts,
                ))
            })
            .unwrap();
        }
    }
}
