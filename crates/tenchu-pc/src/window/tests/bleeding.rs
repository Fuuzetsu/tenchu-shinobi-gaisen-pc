use super::*;
use tenchu_core::{game::bleeding::Settings as Bleeding, input};

#[test]
#[ignore = "requires TENCHU_DISC; disabled settings preserve exact gameplay and treatment survives save/rewind"]
fn bleeding_options_input_and_saved_continuation() {
    let mut app = application(Options::default());
    let mut original = application(Options::default());
    for a in [&mut app, &mut original] {
        a.campaign.game.events = Default::default();
    }
    app.set_bleeding(Bleeding {
        chance: 100,
        damage: 20,
        ..Default::default()
    });
    for tick in 0..120 {
        let buttons = if tick < 60 { input::UP } else { input::RIGHT };
        assert!(app.game_tick(buttons) && original.game_tick(buttons));
        assert_eq!(
            postcard::to_allocvec(&app.campaign.game).unwrap(),
            postcard::to_allocvec(&original.campaign.game).unwrap()
        );
    }
    let previous = app.settings.clone();
    app.apply_item(Item::Bleeding, 1).unwrap();
    app.reset_item(Item::BleedDamage).unwrap();
    app.apply_settings(&previous);
    assert!(app.campaign.game.player().bleeding.is_some());
    assert_eq!(app.settings.bleeding.damage, 1);
    assert!(!app.trial.eligible);
    app.settings.rewind_seconds = 5;
    let player = app
        .campaign
        .game
        .roster
        .get_mut(app.campaign.game.player_id);
    player.health = 80;
    let state = player.bleeding.as_mut().unwrap();
    state.active = true;
    state.remaining = 300;
    for _ in 0..17 {
        assert!(app.game_tick(0));
    }
    assert!(app.game_tick(input::R2));
    assert!(app.campaign.game.hud.bandage_selected());
    let unchanged = postcard::to_allocvec(&app.campaign.game).unwrap();
    let previous = app.settings.clone();
    app.apply_item(Item::BleedingDetails, 1).unwrap();
    app.apply_settings(&previous);
    assert!(app.settings.bleeding_details);
    assert_eq!(
        postcard::to_allocvec(&app.campaign.game).unwrap(),
        unchanged
    );
    let restored_settings: Settings =
        serde_json::from_slice(&serde_json::to_vec(&app.settings).unwrap()).unwrap();
    assert!(restored_settings.bleeding_details);
    assert!(
        !serde_json::from_str::<Settings>("{}")
            .unwrap()
            .bleeding_details
    );
    let previous = app.settings.clone();
    app.reset_item(Item::BleedingDetails).unwrap();
    app.apply_settings(&previous);
    assert!(!app.settings.bleeding_details);
    assert_eq!(
        postcard::to_allocvec(&app.campaign.game).unwrap(),
        unchanged
    );
    if let Some(path) = std::env::var_os("TENCHU_BANDAGE_REVIEW") {
        let game = &app.campaign.game;
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        let background = review.backdrop(0.3);
        let mut geometry = crate::render_effects::build(game);
        for vertex in &mut geometry.vertices {
            if vertex.flags & crate::render_mesh::UI == 0 {
                vertex.flags |= crate::render_mesh::FOG;
            }
        }
        geometry.remove_flag(crate::render_mesh::FOG);
        let effects = review.renderer.upload_effects(geometry);
        review
            .capture(
                glam::Mat4::IDENTITY,
                &[&background],
                Some(&effects),
                true,
                path.as_ref(),
            )
            .unwrap();
    }
    let saved = app.capture_state().unwrap();
    if let Some(path) = std::env::var_os("TENCHU_BLEED_REVIEW") {
        std::fs::write(path, &saved).unwrap();
    }
    let expected = postcard::to_allocvec(&app.campaign.game).unwrap();
    let rewind_tick = app.campaign.game.tick;
    let mut restored = application(Options::default());
    restored.restore_state(&saved).unwrap();
    assert_eq!(restored.settings.bleeding, app.settings.bleeding);
    for _ in 0..35 {
        assert!(app.game_tick(0) && restored.game_tick(0));
    }
    assert_eq!(
        postcard::to_allocvec(&app.campaign.game).unwrap(),
        postcard::to_allocvec(&restored.campaign.game).unwrap()
    );
    assert_eq!(app.campaign.game.player().health, 79);
    app.seek_rewind(rewind_tick).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), expected);
    app.set_paused(false);
    app.controls.key(KeyCode::KeyB, true);
    app.controls.key(KeyCode::KeyB, false);
    let buttons = app.player_input();
    assert_eq!(
        buttons, 0,
        "bandage binding leaked into original pad buttons"
    );
    assert!(app.game_tick(buttons));
    let state = app.campaign.game.player().bleeding.as_ref().unwrap();
    assert!(!state.active);
    assert_eq!(state.bandages, 2);
    assert_eq!(app.campaign.game.player().health, 80);
    assert!(
        app.rewind
            .entries
            .back()
            .unwrap()
            .input
            .as_ref()
            .unwrap()
            .bandage
    );
    let previous = app.settings.clone();
    app.apply_item(Item::BandageLoadout, 9).unwrap();
    app.apply_settings(&previous);
    assert_eq!(
        app.campaign
            .game
            .player()
            .bleeding
            .as_ref()
            .unwrap()
            .bandages,
        2
    );
    let previous = app.settings.clone();
    app.reset_item(Item::Bleeding).unwrap();
    app.apply_settings(&previous);
    assert!(app.campaign.game.player().bleeding.is_none());
    app.set_bleeding(Bleeding {
        enabled: true,
        ..Default::default()
    });
    app.set_gameplay_preset(0).unwrap();
    assert!(!app.settings.bleeding.enabled && app.campaign.game.player().bleeding.is_none());
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_BLEED_HUD_REVIEW and X11 GPU; compact/detailed HUD on desktop/small viewports without changing gameplay"]
fn bleeding_hud_is_compact_and_respects_visibility() {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_BLEED_HUD_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut app = application(Options::default());
    app.campaign.game.events = Default::default();
    app.set_bleeding(Bleeding {
        enabled: true,
        ..Default::default()
    });
    for _ in 0..30 {
        assert!(app.game_tick(0));
    }
    let player = app
        .campaign
        .game
        .roster
        .get_mut(app.campaign.game.player_id);
    player.health = 80;
    let state = player.bleeding.as_mut().unwrap();
    state.active = true;
    state.remaining = 300;
    assert!(app.game_tick(input::R2));
    assert!(app.campaign.game.hud.bandage_selected());
    let before = postcard::to_allocvec(&app.campaign.game).unwrap();
    app.settings.graphics.asset_pack = false;
    let mut builder = EventLoop::builder();
    builder.with_x11().with_any_thread(true);
    let events = builder.build().unwrap();
    #[allow(deprecated)]
    let window = events
        .create_window(winit::window::Window::default_attributes().with_visible(false))
        .unwrap();
    app.view = Some(
        pollster::block_on(View::from_window(
            std::sync::Arc::new(window),
            &app.assets,
            &app.campaign.game,
            &app.settings,
        ))
        .unwrap(),
    );
    let capture = |app: &mut Application, name: &str| {
        app.update_view();
        app.update_overlay();
        let view = app.view.as_mut().unwrap();
        assert_eq!(
            view.labels.iter().any(|l| l.1.contains("bandages")),
            app.settings.bleeding_details && app.settings.show_hud && !app.debug.photo_mode
        );
        let path = output.join(format!("t1-{name}.png"));
        view.capture(&app.campaign.game, &app.debug, &path).unwrap();
        std::fs::read(path).unwrap()
    };
    for (preset, name) in [
        (0, "original"),
        (settings::presets::MODERN_HIGH, "high"),
        (settings::presets::MODERN_ULTRA, "ultra"),
    ] {
        app.settings.set_visual_preset(preset);
        app.view
            .as_mut()
            .unwrap()
            .settings(&app.settings, &app.campaign.game)
            .unwrap();
        app.view.as_mut().unwrap().resize(1280, 720).unwrap();
        for details in [true, false] {
            app.settings.bleeding_details = details;
            capture(&mut app, &format!("{name}-details-{details}"));
        }
    }
    app.settings
        .set_visual_preset(settings::presets::MODERN_HIGH);
    app.view
        .as_mut()
        .unwrap()
        .settings(&app.settings, &app.campaign.game)
        .unwrap();
    app.view.as_mut().unwrap().resize(640, 360).unwrap();
    for details in [true, false] {
        app.settings.bleeding_details = details;
        capture(&mut app, &format!("small-details-{details}"));
    }
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), before);
    let active = capture(&mut app, "active");
    app.debug.photo_mode = true;
    let photo = capture(&mut app, "photo");
    app.debug.photo_mode = false;
    app.settings.show_hud = false;
    assert_eq!(capture(&mut app, "hidden"), photo);
    app.settings.show_hud = true;
    app.campaign
        .game
        .roster
        .get_mut(app.campaign.game.player_id)
        .bleeding
        .as_mut()
        .unwrap()
        .clear();
    let treated = capture(&mut app, "treated");
    assert_ne!(active, treated);
    app.settings.show_hud = false;
    assert_eq!(capture(&mut app, "treated-hidden"), photo);
    app.settings.show_hud = true;
    app.set_bleeding(Bleeding::default());
    let disabled = capture(&mut app, "disabled");
    app.settings.bleeding_details = true;
    app.update_view();
    assert!(
        !app.view
            .as_ref()
            .unwrap()
            .labels
            .iter()
            .any(|l| l.1.contains("bandages"))
    );
    app.update_overlay();
    app.view
        .as_mut()
        .unwrap()
        .capture(
            &app.campaign.game,
            &app.debug,
            &output.join("t1-disabled-details.png"),
        )
        .unwrap();
    assert_eq!(
        disabled,
        std::fs::read(output.join("t1-disabled-details.png")).unwrap()
    );
}
