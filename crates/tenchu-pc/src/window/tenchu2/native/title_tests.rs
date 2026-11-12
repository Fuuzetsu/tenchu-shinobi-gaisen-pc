use super::*;
use crate::tenchu2::menu::Event;

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU2_DISC and empty isolated TENCHU_T2_CONFIG/TENCHU_T2_SAVE; title switching, import, saved progress and held input suppression"]
fn titles_switch_through_import_and_remembered_disc() {
    let config = std::env::var_os("TENCHU_T2_CONFIG").expect("Isolate disc preferences");
    let save = std::env::var_os("TENCHU_T2_SAVE").expect("Isolate native progress");
    assert!(!std::path::Path::new(&config).exists());
    let progress = std::fs::read(&save).ok();
    let mut app = crate::window::tests::application(Default::default());
    app.campaign.saved = Some(Default::default());
    let gaisen_save = app.campaign.saved.as_ref().unwrap().encode();
    app.frontend = Some(
        crate::frontend::Frontend::new(
            &app.assets,
            &app.disc,
            &app.campaign,
            app.settings.frontend.clone(),
        )
        .unwrap(),
    );
    app.open_title();
    app.controls.key(KeyCode::ArrowRight, true);
    app.frontend_action(Action::Right);
    assert!(app.tenchu2.open && app.tenchu2.title);
    assert!(app.native_tenchu2.is_none());
    assert!(!app.controls.held(KeyCode::ArrowRight));
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        assert!(app.tenchu2.show(ctx).is_none());
    });
    app.apply_tenchu2(Event::Close).unwrap();
    assert!(app.frontend_visible());
    app.frontend_action(Action::Left);
    app.tenchu2.path = std::env::var("TENCHU2_DISC").unwrap();
    app.apply_tenchu2(Event::Connect).unwrap();
    assert!(matches!(
        app.native_tenchu2.as_ref().unwrap().playback.content,
        Content::Title(_)
    ));
    assert!(!app.tenchu2.open && !app.tenchu2.title);
    for _ in 0..15 {
        app.step_native_tenchu2().unwrap();
    }
    app.native_tenchu2.as_mut().unwrap().playback.confirm();
    app.step_native_tenchu2().unwrap();
    assert!(
        matches!(&app.native_tenchu2.as_ref().unwrap().playback.content, Content::Title(title) if title.state.active)
    );
    app.native_tenchu2.as_mut().unwrap().playback.action = Some(NativeAction::Quit);
    assert!(app.step_native_tenchu2().unwrap());
    app.back_from_native_tenchu2().unwrap();
    assert!(app.frontend_visible());
    assert_eq!(
        app.frontend.as_ref().unwrap().page,
        crate::frontend::Page::Title
    );
    app.controls.key(KeyCode::ArrowLeft, true);
    app.frontend_action(Action::Left);
    assert!(matches!(
        app.native_tenchu2.as_ref().unwrap().playback.content,
        Content::Title(_)
    ));
    assert!(!app.controls.held(KeyCode::ArrowLeft));
    assert_eq!(app.campaign.saved.as_ref().unwrap().encode(), gaisen_save);
    assert_eq!(std::fs::read(save).ok(), progress);
    let disc = app.tenchu2.japanese_disc().unwrap().to_owned();
    app.back_from_native_tenchu2().unwrap();
    app.tenchu2 = Default::default();
    app.frontend_action(Action::Right);
    assert_eq!(app.tenchu2.japanese_disc(), Some(disc.as_str()));
    assert!(app.native_tenchu2.is_some());
}
