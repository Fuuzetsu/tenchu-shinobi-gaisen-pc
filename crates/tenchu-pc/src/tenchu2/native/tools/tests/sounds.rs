use super::*;
use crate::audio::output::{Command, Output};
use crate::tenchu2::native::inspection::sounds as regions;
use glam::Vec3;

#[test]
#[ignore = "requires TENCHU2_DISC; clicking sign previews while paused leaves lessons, RNG and scene state untouched"]
fn sound_marker_previews_do_not_activate_game_triggers() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut effects = 0;
    for sequence in [1, 9, 119] {
        let session = crate::tenchu2::native::Session::load(&disc, sequence, 1).unwrap();
        let mut content = Content::Gameplay(Box::new(session));
        let mut tools = Tools::default();
        tools.sync(&content, 0);
        tools.debug.pause_view = true;
        tools.change(Item::SoundTriggers, 1).unwrap();
        let (output, commands) = Output::recording();
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        let before = postcard::to_allocvec(&session.world).unwrap();
        let view = session.world.camera.view;
        let markers = regions::markers(session);
        assert_eq!(
            markers
                .iter()
                .filter(|m| m.label.starts_with("Sign "))
                .count(),
            if sequence == 1 { 4 } else { 0 }
        );
        if sequence == 1 {
            assert!(markers.iter().any(|m| m.label.starts_with("Lesson ")));
        }
        effects += markers
            .iter()
            .filter(|m| matches!(m.cue, Cue::Effect(_)))
            .count();
        let targets: Vec<_> = markers
            .into_iter()
            .filter(|m| m.label.starts_with("Sign ") || matches!(m.cue, Cue::Effect(_)))
            .collect();
        for (index, marker) in targets.into_iter().enumerate() {
            let mut camera = Camera::from_view(view, 300.0);
            camera.position = marker.position - Vec3::Z * 3000.0;
            camera.yaw = 0.0;
            camera.pitch = 0.0;
            if index % 2 == 0 {
                tools.debug.camera = Some(camera);
            } else {
                tools.debug.camera = None;
                tools.scene.camera = Some(camera);
            }
            tools.cursor = Some((400.0, 300.0));
            let event = WindowEvent::MouseInput {
                device_id: winit::event::DeviceId::dummy(),
                state: ElementState::Pressed,
                button: MouseButton::Left,
            };
            tools.event(&event, true, &Default::default());
            tools.apply(&mut content, [800, 600]).unwrap();
            assert_eq!(tools.preview, Some(marker.cue));
            tools.preview_audio(&disc, &content, Some(&output)).unwrap();
            match (marker.cue, commands.try_recv().unwrap()) {
                (Cue::Narration(_), Command::Preview(Some(clip))) => {
                    assert!(clip.frames.iter().flatten().any(|s| s.unsigned_abs() > 100))
                }
                (Cue::Effect(expected), Command::PreviewEffect(actual)) => {
                    assert_eq!(actual, expected)
                }
                _ => panic!("preview replaced the campaign audio"),
            }
            tools.event(&event, false, &Default::default());
            tools.apply(&mut content, [800, 600]).unwrap();
            assert_eq!(tools.preview, None, "menu must consume clicks");
            tools.debug.photo_mode = true;
            tools.event(&event, true, &Default::default());
            tools.apply(&mut content, [800, 600]).unwrap();
            assert_eq!(tools.preview, None, "photo mode hides clickable markers");
            tools.debug.photo_mode = false;
            // Reborrow after applying the UI request: no gameplay update took place.
            let Content::Gameplay(current) = &content else {
                unreachable!()
            };
            assert_eq!(postcard::to_allocvec(&current.world).unwrap(), before);
            assert!(!tools.debug.actors_edited);
        }
        tools.change(Item::SoundTriggers, 1).unwrap();
        tools.preview_audio(&disc, &content, Some(&output)).unwrap();
        assert!(matches!(commands.try_recv(), Ok(Command::Preview(None))));
        tools.preview = Some(Cue::Narration(7));
        tools.sync(&Content::Failed, 1);
        assert_eq!(tools.preview, None, "scene changes discard pending clicks");
    }
    assert!(effects > 0, "scenery sound previews were not exercised");
}
