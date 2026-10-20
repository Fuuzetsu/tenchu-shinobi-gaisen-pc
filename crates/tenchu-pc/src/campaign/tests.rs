use super::*;
mod routes;

fn assets() -> GameAssets<FileSource> {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap()
}

#[test]
#[ignore = "requires TENCHU_DISC; changes localized mission assets through the launcher and saved continuation"]
fn launcher_language_applies_to_new_and_saved_games() {
    let assets = assets();
    let mut campaign = Campaign::new(&assets, Options::default(), None).unwrap();
    campaign.action(Action::Up, &assets).unwrap();
    campaign.action(Action::Up, &assets).unwrap();
    campaign.action(Action::Right, &assets).unwrap();
    for _ in 0..2 {
        campaign.action(Action::Down, &assets).unwrap();
    }
    campaign.action(Action::Confirm, &assets).unwrap();
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.selection.language, Language::French);
    campaign.action(Action::Confirm, &assets).unwrap();
    assert_eq!(campaign.game.mission.scripts.language, Language::French);

    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    assert_eq!(resumed.options.language, Some(Language::French));
    resumed.action(Action::Up, &assets).unwrap();
    resumed.action(Action::Right, &assets).unwrap();
    resumed.action(Action::Down, &assets).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    let save = Save::decode(&resumed.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.selection.language, Language::Italian);
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.game.mission.scripts.language, Language::Italian);

    let options = Options::parse(&["--language".into(), "english".into()]).unwrap();
    let mut resumed = Campaign::new(&assets, options, Some(save)).unwrap();
    assert_eq!(resumed.options.language, Some(Language::English));
    resumed.action(Action::Up, &assets).unwrap();
    resumed.action(Action::Left, &assets).unwrap();
    assert_eq!(resumed.options.language, Some(Language::Japanese));
    resumed.action(Action::Down, &assets).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.game.mission.scripts.language, Language::Japanese);
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises campaign transitions using controlled completion/death outcomes, not a playthrough"]
fn campaign_results_retry_and_saved_continuation() {
    let assets = assets();
    for character in 0..2 {
        let options = Options {
            character,
            ..Options::default()
        };
        let mut campaign = Campaign::new(&assets, options, None).unwrap();
        campaign.action(Action::Confirm, &assets).unwrap();
        campaign.action(Action::Right, &assets).unwrap();
        campaign.action(Action::Right, &assets).unwrap();
        assert!(campaign.action(Action::Confirm, &assets).unwrap());
        assert_eq!(campaign.game.player().inventory[1], 2);
        assert_eq!(campaign.progress.stock[character as usize][1], 4);
        campaign.game.events.outcome = Outcome::GameOver;
        campaign.tick(0).unwrap();
        assert!(matches!(campaign.screen, Screen::GameOver));
        let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
        assert_eq!(save.progress.stock[character as usize][1], 4);
        campaign.saved = Some(save);
        campaign.action(Action::Confirm, &assets).unwrap();
        let Screen::Inventory(loadout) = &campaign.screen else {
            panic!("Expected retry equipment");
        };
        assert_eq!(loadout.stock[1], 4);
        assert_eq!(loadout.selected[1], 0);
        campaign.action(Action::Confirm, &assets).unwrap();
        for mission in 1..=10 {
            assert_eq!(campaign.options.mission, mission);
            assert!(campaign.playing());
            let stage = campaign.game.mission.stage.id;
            let layout = campaign.game.mission.layout_index;
            campaign.game.clock = 3000;
            campaign.game.statistics.criticals = 10;
            campaign.game.events.outcome = Outcome::Complete;
            campaign.tick(0).unwrap();
            let Screen::Results(result) = &campaign.screen else {
                panic!("Expected results");
            };
            assert_eq!(result.score.grade, 4);
            assert_eq!(result.record.clock, 3000);
            assert_eq!(
                campaign.progress.records[character as usize][stage][layout],
                result.record
            );
            let progress = campaign.progress.clone();
            campaign.tick(0xffff).unwrap();
            assert_eq!(campaign.progress, progress, "Results must only award once");
            let save = campaign.save_request.take().unwrap();
            assert_eq!(
                save.selection.stage as usize,
                campaign
                    .stages
                    .iter()
                    .find(|s| s.order == (mission + 1).min(10) as i32)
                    .unwrap()
                    .id
            );
            campaign.saved = Some(Save::decode(&save.encode()).unwrap());
            campaign.action(Action::Confirm, &assets).unwrap();
            if mission != 10 {
                campaign.action(Action::Confirm, &assets).unwrap();
            }
        }
        assert!(matches!(campaign.screen, Screen::Complete));
        assert_eq!(campaign.progress.flags & 0x7ff, 0x7ff);
        let mut resumed =
            Campaign::new(&assets, Options::default(), campaign.saved.clone()).unwrap();
        resumed.action(Action::Confirm, &assets).unwrap();
        assert_eq!(resumed.options.character, character);
        assert_eq!(resumed.options.mission, 10);
        assert_eq!(resumed.progress, campaign.progress);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; validates random layouts and equipped armour assets for both characters"]
fn random_layouts_and_armour_load_from_disc() {
    let assets = assets();
    let mut selections = std::collections::BTreeSet::new();
    for character in 0..2 {
        for seed in 1..=9 {
            let options = Options {
                character,
                layout: 0,
                ..Options::default()
            };
            let mut selected = [0; 20];
            selected[0] = 255;
            selected[19] = if seed % 2 == 0 { 255 } else { 1 };
            let mut game = options.load_equipped(&assets, seed, selected).unwrap();
            assert!(game.combat.armor);
            assert_eq!(game.player().inventory[19], 0);
            assert_eq!(
                game.mission.characters[game.player().definition].model,
                if character == 0 {
                    "RIKIMAUA"
                } else if selected[19] == 255 {
                    "AYAMES"
                } else {
                    "AYAMEA"
                }
            );
            assert!(game.mission.random_layouts.is_none());
            let layout = game.mission.layout_index;
            selections.insert(layout);
            let original =
                tenchu_assets::stage::MissionAssets::load(&assets, 0, character, layout + 1)
                    .unwrap();
            assert_eq!(game.mission.layout, original.layout);
            for _ in 0..40 {
                game.step(0).unwrap();
            }
            assert!(!crate::render_mesh::characters(&game, None).is_empty());
        }
    }
    assert_eq!(selections.len(), 3);
}
