use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; verifies retail music transitions and sound-test preview memory"]
fn options_and_shop_follow_retail_music_transitions() {
    fn music(frontend: &mut Frontend) -> Vec<Option<usize>> {
        std::mem::take(&mut frontend.events)
            .into_iter()
            .filter_map(|event| match event {
                Event::Music(track) => Some(track),
                _ => None,
            })
            .collect()
    }
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    frontend.events.clear();
    frontend.title.selection = Some(3);
    frontend.title.finished = true;
    frontend
        .advance(Duration::from_millis(17), false, &mut campaign, &assets)
        .unwrap();
    assert_eq!(frontend.page, Page::Options);
    assert_eq!(music(&mut frontend), [None]);
    let original_random = campaign.game.random.seed;
    for (option, expected) in [(3, Some(15)), (0, Some(12))] {
        frontend.option_selection = option;
        frontend.open_option(&campaign);
        assert_eq!(music(&mut frontend), [expected]);
        frontend.ticks = 16;
        frontend
            .action(Action::Back, &mut campaign, &assets)
            .unwrap();
        assert_eq!(music(&mut frontend), [None]);
    }
    for selection in [6, 0] {
        frontend.option_selection = 4;
        frontend.open_option(&campaign);
        assert!(music(&mut frontend).is_empty());
        frontend.ticks = 16;
        frontend.sound_choices[0] = selection;
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
        let track = selection
            .checked_sub(1)
            .map(|i| frontend.tables.sound_test_music[i]);
        assert_eq!(music(&mut frontend), [track]);
        frontend
            .action(Action::Back, &mut campaign, &assets)
            .unwrap();
        assert_eq!(music(&mut frontend), [None]);
        frontend.option_selection = 3;
        frontend.open_option(&campaign);
        assert_eq!(music(&mut frontend), [Some(track.unwrap_or(15))]);
        frontend.ticks = 16;
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
        assert_eq!(music(&mut frontend), [None]);
    }
    campaign.inventory();
    frontend.prepare(&campaign, &assets, false).unwrap();
    assert_eq!(frontend.page, Page::Shop);
    assert_eq!(music(&mut frontend), [None]);
    assert_eq!(campaign.game.tick, 0);
    assert_eq!(campaign.game.random.seed, original_random);
}

#[test]
#[ignore = "requires TENCHU_DISC; loads every original frontend asset and character animation"]
fn original_assets_and_selection_motions_load_for_all_languages() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    for language in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
    ] {
        frontend.language(&assets, language).unwrap();
        assert_eq!(frontend.mission_images.len(), 36);
        assert_eq!(frontend.character_images.len(), 3);
        for movie in [
            Movie::Training,
            Movie::Rikimaru,
            Movie::Ayame,
            Movie::Credits,
        ] {
            let bytes = assets.asset(&movie.subtitles(language).unwrap()).unwrap();
            tenchu_assets::frontend::movie_commands(&bytes).unwrap();
            Bank::parse(&assets.asset(&movie.voices(language).unwrap()).unwrap()).unwrap();
        }
    }
    for selected in 0..2 {
        frontend.characters.reset(&frontend.tables);
        frontend.characters.select(selected, &frontend.tables);
        for _ in 0..180 {
            frontend.characters.tick(&campaign.game.math).unwrap();
        }
        frontend.characters.confirm(&frontend.tables);
        for _ in 0..180 {
            frontend.characters.tick(&campaign.game.math).unwrap();
        }
        assert!(frontend.characters.done());
    }
    for page in [
        Page::Title,
        Page::Character,
        Page::Missions,
        Page::Options,
        Page::Controls,
        Page::Volume,
        Page::SoundTest,
        Page::Records,
        Page::Card,
        Page::Choice(1),
        Page::Choice(2),
        Page::Choice(6),
        Page::Choice(8),
        Page::Choice(9),
    ] {
        frontend.title.activate();
        frontend.set_page(page);
        frontend.ticks = 120;
        frontend.paint(&campaign);
        if let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE") {
            capture(&frontend.canvas, Path::new(&path), &format!("{page:?}"));
        }
    }
}

pub(super) fn capture(canvas: &Canvas, directory: &Path, name: &str) {
    std::fs::create_dir_all(directory).unwrap();
    let file = std::fs::File::create(directory.join(format!("{name}.png"))).unwrap();
    let mut png = png::Encoder::new(file, canvas.size[0] as u32, canvas.size[1] as u32);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&canvas.pixels)
        .unwrap();
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises menu cancellation, remapping and progress restrictions"]
fn menu_navigation_does_not_change_preferences_or_gameplay() {
    use tenchu_core::{game::score::Record, progress::Completion};
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    frontend.play_movie(Movie::Opening, &assets).unwrap();
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    let random = campaign.game.random.seed;
    for option in 0..10 {
        frontend.option_selection = option;
        frontend.open_option(&campaign);
        frontend.events.clear();
        frontend
            .advance(Duration::from_millis(250), false, &mut campaign, &assets)
            .unwrap();
        frontend
            .advance(Duration::from_millis(50), false, &mut campaign, &assets)
            .unwrap();
        frontend.paint(&campaign);
        frontend
            .action(Action::Back, &mut campaign, &assets)
            .unwrap();
        assert_eq!(frontend.preferences, Preferences::default());
        assert_eq!(frontend.page, Page::Options);
    }
    assert_eq!(campaign.game.tick, 0);
    assert_eq!(campaign.game.random.seed, random);
    frontend.option_selection = 3;
    frontend.open_option(&campaign);
    frontend.ticks = 16;
    frontend
        .action(Action::Right, &mut campaign, &assets)
        .unwrap();
    assert_eq!(frontend.preferences.music, 98);
    frontend
        .action(Action::Back, &mut campaign, &assets)
        .unwrap();
    assert_eq!(frontend.preferences.music, 96);
    frontend.option_selection = 6;
    frontend.open_option(&campaign);
    frontend.ticks = 16;
    frontend.set_rumble(false);
    assert_eq!(frontend.choice, 1);
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    assert!(!frontend.preferences.rumble);
    frontend.set_rumble(true);
    frontend.open_option(&campaign);
    frontend.ticks = 16;
    assert_eq!(frontend.choice, 0);
    frontend
        .action(Action::Down, &mut campaign, &assets)
        .unwrap();
    frontend.set_rumble(true);
    assert_eq!(
        frontend.choice, 1,
        "unrelated F1 edits must preserve an unconfirmed choice"
    );
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    assert!(!frontend.preferences.rumble);
    for scheme in 0..4 {
        frontend.preferences.control_scheme = scheme;
        for (source, target) in frontend.tables.button_assign[0]
            .iter()
            .zip(frontend.tables.button_assign[scheme])
        {
            assert_eq!(
                frontend.buttons(u16::from(*source) | tenchu_core::input::UP),
                u16::from(target) | tenchu_core::input::UP
            );
        }
    }
    assert_eq!(unlocked_mission(&campaign), 1);
    assert!(!layout_available(&campaign, 1, 1));
    // A zero-score clear advances the campaign even when it doesn't replace a record.
    let stage = campaign.stages.iter().find(|s| s.order == 1).unwrap();
    campaign.progress.complete(
        Completion {
            stage,
            character: 0,
            layout: 0,
            language: Language::English,
            record: Record {
                enemies: 10,
                found: 20,
                clock: 3000,
                ..Record::default()
            },
            remaining: [0; 20],
        },
        &mut campaign.game.random,
    );
    assert!(!campaign.progress.records[0][stage.id][0].played());
    assert_eq!(unlocked_mission(&campaign), 2);
    campaign.progress.records[0][stage.id][0].enemies = 10;
    assert!(layout_available(&campaign, 1, 1));
    assert!(!layout_available(&campaign, 1, 2));
    campaign.options.character = 1;
    assert_eq!(unlocked_mission(&campaign), 1);
    assert!(!layout_available(&campaign, 1, 1));
    frontend.quick_launcher();
    assert!(frontend.movie.is_none());
    assert!(matches!(frontend.events.as_slice(), [Event::QuickLauncher]));
}

#[test]
#[ignore = "requires TENCHU_DISC; controlled final completion checks ending/results/retry, not a campaign playthrough"]
fn final_completion_hands_off_through_both_epilogues_to_results() {
    use crate::campaign::Screen;
    use tenchu_core::game::events::Outcome;
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for character in 0..2 {
        let options = crate::session::Options {
            mission: 10,
            character,
            ..Default::default()
        };
        let mut campaign = Campaign::new(&assets, options, None).unwrap();
        campaign.frontend = true;
        let mut frontend =
            Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
        frontend.movie = None;
        campaign.inventory();
        campaign.action(Action::Confirm, &assets).unwrap();
        campaign.game.clock = 3000;
        campaign.game.statistics.criticals = 10;
        campaign.game.statistics.friend_hits = 2;
        campaign.game.events.outcome = Outcome::Complete;
        campaign.tick(0).unwrap();
        assert!(matches!(campaign.screen, Screen::Ending(_)));
        let progress = campaign.progress.clone();
        let movie = if character == 0 {
            Movie::Rikimaru
        } else {
            Movie::Ayame
        };
        frontend.play_movie(movie, &assets).unwrap();
        assert!(frontend.movie.is_some(), "{}", frontend.notice);
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
        assert_eq!(frontend.page, Page::Movie(movie));
        frontend.movie_finished(&assets).unwrap();
        assert_eq!(frontend.page, Page::Movie(Movie::Credits));
        frontend.events.clear();
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
        assert_eq!(frontend.page, Page::Results);
        assert!(frontend.events.iter().any(|e| matches!(e, Event::Results)));
        let Screen::Ending(results) = std::mem::replace(&mut campaign.screen, Screen::Frontend)
        else {
            unreachable!()
        };
        campaign.screen = Screen::Results(results);
        frontend.ticks = 120;
        frontend.paint(&campaign);
        if let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE") {
            capture(
                &frontend.canvas,
                Path::new(&path),
                &format!("results-{character}"),
            );
        }
        assert_eq!(campaign.progress, progress);
        campaign.action(Action::Replay, &assets).unwrap();
        assert!(matches!(campaign.screen, Screen::Inventory(_)));
        assert_eq!(campaign.options.mission, 10);
        frontend.enter_title(&assets).unwrap();
        assert_eq!(frontend.page, Page::Title);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises deterministic showcase menus without campaign updates"]
fn showcase_pages_restart_identically_without_advancing_gameplay() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    campaign.options.mission = 7;
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    let state = crate::session::tests::state(&campaign.game);
    let progress = campaign.progress.clone();
    for page in [
        showcase::Menu::Title,
        showcase::Menu::Character,
        showcase::Menu::Missions,
        showcase::Menu::Shop,
    ] {
        for language in ["en", "ja", "fr", "it"] {
            let scene = showcase::Scene {
                page,
                language: language.into(),
            };
            let mut previous = None;
            for _ in 0..2 {
                frontend
                    .stage_showcase(&scene, &assets, &mut campaign)
                    .unwrap();
                for _ in 0..120 {
                    frontend
                        .advance(
                            Duration::from_nanos(16_666_667),
                            false,
                            &mut campaign,
                            &assets,
                        )
                        .unwrap();
                }
                frontend.paint(&campaign);
                if let Some(previous) = previous {
                    assert_eq!(frontend.canvas.pixels, previous);
                }
                previous = Some(frontend.canvas.pixels.clone());
                assert!(frontend.events.iter().all(|event| !matches!(
                    event,
                    Event::Save | Event::Load | Event::Launch | Event::Quit
                )));
            }
        }
    }
    assert_eq!(state, crate::session::tests::state(&campaign.game));
    assert_eq!(progress, campaign.progress);
    assert!(campaign.save_request.is_none());
}

#[test]
#[ignore = "requires TENCHU_DISC; verifies all retail briefings, localized shop/results artwork and narrated transitions"]
fn mission_preparation_assets_and_transitions() {
    use tenchu_core::{game::score::Record, progress::Completion};
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    let stock = campaign.progress.stock;
    let seed = campaign.seed;
    for language in [
        Language::English,
        Language::French,
        Language::Italian,
        Language::Japanese,
    ] {
        campaign.options.language = Some(language);
        for mission in 1..=10 {
            campaign.options.mission = mission;
            campaign.inventory();
            frontend.prepare(&campaign, &assets, true).unwrap();
            assert_eq!(frontend.page, Page::Briefing);
            if mission == 1
                && let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE")
            {
                let brief = frontend.briefing.as_ref().unwrap();
                eprintln!(
                    "{language:?}: text {} {:?}, hold {}, step {}, narration {:?}",
                    brief.assets.text,
                    brief.text.size,
                    brief.assets.hold,
                    brief.assets.scroll_step,
                    brief.assets.narration
                );
                let mut canvas = Canvas::new(brief.text.size[0]);
                canvas.sprite(raster::Sprite::new(&brief.text, 0.0, 0.0));
                capture(
                    &canvas,
                    Path::new(&path),
                    &format!("briefing-text-{language:?}"),
                );
            }
            frontend.events.clear();
            for _ in 0..60 {
                frontend.advance_briefing();
            }
            let brief = frontend.briefing.as_ref().unwrap();
            assert_eq!(brief.fade, 0);
            assert_eq!(brief.scroll, -0xa000, "must wait for audio to load");
            assert!(
                frontend
                    .events
                    .iter()
                    .any(|e| matches!(e, Event::Narration(_)))
            );
            frontend.narration_ready(None);
            let remaining = {
                let brief = frontend.briefing.as_ref().unwrap();
                (brief.assets.narration.end - brief.assets.narration.start) * 60 / 150
            };
            for _ in 0..remaining / 2 {
                frontend.advance_briefing();
            }
            frontend.paint(&campaign);
            if let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE") {
                capture(
                    &frontend.canvas,
                    Path::new(&path),
                    &format!("briefing-{language:?}-{mission}"),
                );
            }
            for _ in 0..remaining + 100 {
                frontend.advance_briefing();
            }
            assert_eq!(frontend.page, Page::Shop);
            frontend.ticks = 120;
            frontend.paint(&campaign);
            if mission == 1
                && let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE")
            {
                capture(
                    &frontend.canvas,
                    Path::new(&path),
                    &format!("shop-{language:?}"),
                );
            }
        }
        for mission in [0, 1, 10] {
            campaign.options.mission = mission;
            campaign.game = campaign.options.load(&assets).unwrap();
            let results = campaign.progress.complete(
                Completion {
                    stage: &campaign.game.mission.stage,
                    character: 0,
                    layout: 0,
                    language,
                    record: Record {
                        enemies: 10,
                        bosses: 1,
                        criticals: 9,
                        murders: 1,
                        clock: 4890,
                        ..Record::default()
                    },
                    remaining: [0; 20],
                },
                &mut campaign.game.random,
            );
            campaign.screen = crate::campaign::Screen::Results(results);
            frontend.enter_results(&assets, &campaign).unwrap();
            frontend.ticks = 120;
            frontend.paint(&campaign);
            if let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE") {
                capture(
                    &frontend.canvas,
                    Path::new(&path),
                    &format!("results-{language:?}-{mission}"),
                );
            }
        }
        campaign.progress = tenchu_core::progress::Progress::default();
    }
    assert_eq!(campaign.progress.stock, stock);
    assert_eq!(campaign.seed, seed);
    for stage in [6, 7] {
        let a = preparation::Briefing::load(&assets, Language::Japanese, stage, 0).unwrap();
        let b = preparation::Briefing::load(&assets, Language::Japanese, stage, 1).unwrap();
        assert_ne!(a.assets.narration, b.assets.narration);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; shop transfers, limits, cancellation, launch and retry preserve campaign stock"]
fn shop_actions_commit_only_the_selected_loadout() {
    use crate::campaign::Screen;
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, crate::session::Options::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    campaign.progress.stock[0][1..9].fill(20);
    let stock = campaign.progress.stock;
    campaign.inventory();
    frontend.prepare(&campaign, &assets, false).unwrap();
    frontend.ticks = 16;
    for item in 1..=4 {
        campaign.cursor = campaign.shop.order.iter().position(|&i| i == item).unwrap();
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
    }
    campaign.cursor = campaign.shop.order.iter().position(|&i| i == 5).unwrap();
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    assert_eq!(frontend.shop_help, Some(20));
    let Screen::Inventory(loadout) = &campaign.screen else {
        panic!()
    };
    assert_eq!(loadout.taken, 4);
    assert_eq!(loadout.selected[5], 0);
    campaign.cursor = 0;
    for _ in 0..11 {
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
    }
    frontend
        .action(Action::Confirm, &mut campaign, &assets)
        .unwrap();
    assert_eq!(frontend.shop_help, Some(19));
    let Screen::Inventory(loadout) = &campaign.screen else {
        panic!()
    };
    assert_eq!(loadout.taken, 15);
    frontend
        .action(Action::Replay, &mut campaign, &assets)
        .unwrap();
    let Screen::Inventory(loadout) = &campaign.screen else {
        panic!()
    };
    assert_eq!(loadout.taken, 14);
    assert_eq!(campaign.progress.stock, stock);
    frontend
        .action(Action::Back, &mut campaign, &assets)
        .unwrap();
    assert!(
        frontend
            .events
            .iter()
            .any(|e| matches!(e, Event::CancelPreparation))
    );
    campaign.action(Action::Back, &assets).unwrap();
    assert_eq!(
        campaign.progress.stock, stock,
        "cancel must refund the whole selection"
    );
    campaign.inventory();
    frontend.prepare(&campaign, &assets, false).unwrap();
    frontend.ticks = 16;
    for _ in 0..3 {
        frontend
            .action(Action::Confirm, &mut campaign, &assets)
            .unwrap();
    }
    frontend.events.clear();
    frontend
        .action(Action::Start, &mut campaign, &assets)
        .unwrap();
    assert!(matches!(frontend.events.as_slice(), [Event::StartMission]));
    assert!(campaign.action(Action::Confirm, &assets).unwrap());
    assert!(campaign.playing());
    assert_eq!(campaign.game.player().inventory[1], 3);
    assert_eq!(campaign.progress.stock[0][1], 17);
    assert_eq!(campaign.progress.stock[1], stock[1]);
    campaign.action(Action::Replay, &assets).unwrap();
    frontend.prepare(&campaign, &assets, false).unwrap();
    assert_eq!(frontend.page, Page::Shop);
    let Screen::Inventory(loadout) = &campaign.screen else {
        panic!()
    };
    assert_eq!(loadout.stock[1], 17);
    assert_eq!(loadout.selected[1], 0);
    campaign.options.mission = 0;
    campaign.inventory();
    frontend.events.clear();
    frontend.prepare(&campaign, &assets, true).unwrap();
    assert!(
        frontend
            .events
            .iter()
            .any(|e| matches!(e, Event::StartMission))
    );
    assert!(campaign.action(Action::Confirm, &assets).unwrap());
    assert_eq!(campaign.game.player().inventory[1], 5);
    assert_eq!(campaign.progress.stock[0][1], 17);
    campaign.action(Action::Back, &assets).unwrap();
    assert!(matches!(campaign.screen, Screen::Launcher));
    assert_eq!(campaign.progress.stock[0][1], 17);
}

#[test]
#[ignore = "requires TENCHU_DISC; decodes narration in all four languages and both Japanese character variants"]
fn briefing_narration_has_audio_for_each_language() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    for (language, stage, character) in [
        (Language::English, 0, 0),
        (Language::French, 0, 0),
        (Language::Italian, 0, 0),
        (Language::Japanese, 0, 0),
        (Language::Japanese, 6, 0),
        (Language::Japanese, 6, 1),
        (Language::Japanese, 7, 0),
        (Language::Japanese, 7, 1),
    ] {
        let data = tenchu_assets::frontend::mission::Briefing::load(
            &assets.executable,
            language,
            stage,
            character,
        )
        .unwrap();
        let clip = Clip::load(&assets.disc, &data.narration).unwrap();
        assert!(
            clip.frames
                .iter()
                .any(|p| p[0].unsigned_abs() > 256 || p[1].unsigned_abs() > 256)
        );
        let seconds = clip.frames.len() as f64 / f64::from(clip.rate);
        let expected = f64::from(data.narration.end - data.narration.start) / 150.0;
        assert!(
            (seconds - expected).abs() < 1.0,
            "{language:?} stage {stage}: {seconds} vs {expected}"
        );
    }
}
