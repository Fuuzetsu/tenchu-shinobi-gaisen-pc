use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; Studio previews/playtests/return retain the complete campaign game and settings"]
fn studio_playtests_restore_campaign_and_keep_editor_drafts_separate() {
    let mut app = crate::window::tests::application(Default::default());
    let path = std::env::temp_dir().join(format!("studio-transition-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    app.studio = crate::level_editor::Editor::new(path.clone());
    let original = postcard::to_allocvec(&app.campaign.game).unwrap();
    let progress = app.campaign.progress.clone();
    app.settings
        .graphics
        .set_preset(crate::settings::presets::MODERN_ULTRA);
    app.settings.graphics.sky.time = 243;
    app.settings.graphics.weather.mode = crate::settings::WeatherMode::Snow;
    let settings = app.settings.clone();
    let seed = app.campaign.seed;
    app.apply_studio(Command::Open).unwrap();
    assert!(app.studio.open);
    assert_eq!(app.studio.project.graphics, settings.graphics);
    assert!(app.campaign.game.trial.is_some());
    let preview = postcard::to_allocvec(&app.campaign.game).unwrap();
    for _ in 0..3 {
        app.advance_studio_preview(0.2);
        app.refresh_studio().unwrap();
    }
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), preview);
    app.apply_studio(Command::Play).unwrap();
    for _ in 0..4 {
        app.game_tick(0);
    }
    assert!(app.studio.testing);
    assert_eq!(app.campaign.game.tick, 4);
    app.apply_studio(Command::Return).unwrap();
    assert_eq!(app.campaign.game.tick, 0);
    app.apply_studio(Command::Close).unwrap();
    assert_eq!(postcard::to_allocvec(&app.campaign.game).unwrap(), original);
    assert_eq!(app.campaign.progress, progress);
    assert_eq!(app.campaign.seed, seed);
    assert_eq!(app.settings, settings);
    assert!(matches!(app.campaign.screen, Screen::Playing));
    assert!(!app.trial.open);
    assert!(app.trial.mission.is_none());
    assert!(path.exists());
    app.studio.project.graphics.sky.time = 1023;
    app.studio.persist().unwrap();
    app.studio = crate::level_editor::Editor::new(path.clone());
    app.apply_studio(Command::Open).unwrap();
    assert_eq!(app.studio.project.graphics.sky.time, 1023);
    app.apply_studio(Command::Close).unwrap();
    assert_eq!(app.settings, settings);
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC; controlled Studio records and immediate cue completion"]
fn studio_records_require_original_gameplay_and_source_success() {
    use crate::level_editor::{Action, Cue, Event};
    use tenchu_core::game::trial::Phase;
    let mut app = crate::window::tests::application(Default::default());
    let path = std::env::temp_dir().join(format!("studio-records-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    app.studio = crate::level_editor::Editor::new(path.clone());
    app.apply_studio(Command::Open).unwrap();
    app.apply_studio(Command::Play).unwrap();
    assert!(app.studio_previous.as_ref().unwrap().eligible);
    app.campaign.game.trial.as_mut().unwrap().phase = Phase::Success(0);
    app.campaign.game.trial.as_mut().unwrap().elapsed = 900;
    app.campaign.game.events.outcome = Outcome::Complete;
    app.studio_tick().unwrap();
    assert!(app.studio.open);
    assert_eq!(app.studio.project.mission.records[0].ticks, 900);
    app.studio.project.events.push(Event {
        cues: vec![Cue {
            delay: 0,
            action: Action::Complete,
        }],
        ..Event::default()
    });
    app.apply_studio(Command::Play).unwrap();
    assert!(app.studio.open && !app.studio.testing);
    assert_eq!(app.campaign.game.tick, 0);
    assert_eq!(app.studio.project.mission.records.len(), 1);
    app.studio.project.events[0].cues[0].delay = 1;
    app.apply_studio(Command::Play).unwrap();
    assert!(!app.game_tick(0));
    assert!(app.studio.open);
    assert_eq!(app.campaign.game.tick, 0);
    app.apply_studio(Command::Close).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; Studio must retain the native title playback"]
fn studio_restores_native_tenchu2_title() {
    let mut app = crate::window::tests::application(Default::default());
    app.tenchu2.path = std::env::var("TENCHU2_DISC").unwrap();
    app.start_native_tenchu2(super::super::tenchu2::Entry::Title)
        .unwrap();
    let native = app.native_tenchu2.as_ref().unwrap();
    let tick = native.playback.content.tick();
    let generation = native.playback.generation;
    let sequence = native.playback.run.sequence;
    let path = std::env::temp_dir().join(format!("studio-native-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    app.studio = crate::level_editor::Editor::new(path.clone());
    app.apply_studio(Command::Open).unwrap();
    assert!(app.native_tenchu2.is_none());
    app.apply_studio(Command::Play).unwrap();
    app.game_tick(0);
    app.apply_studio(Command::Return).unwrap();
    app.apply_studio(Command::Close).unwrap();
    let native = app.native_tenchu2.as_ref().unwrap();
    assert!(matches!(
        native.playback.content,
        crate::tenchu2::native::campaign::Content::Title(_)
    ));
    assert_eq!(native.playback.content.tick(), tick);
    assert_eq!(native.playback.generation, generation);
    assert_eq!(native.playback.run.sequence, sequence);
    std::fs::remove_file(path).unwrap();
}
