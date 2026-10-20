use super::*;

fn assets() -> GameAssets<FileSource> {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap()
}

fn equip(campaign: &mut Campaign, assets: &GameAssets<FileSource>, items: &[(usize, usize)]) {
    for &(item, count) in items {
        while campaign.shop.order[campaign.cursor] != item {
            campaign.action(Action::Down, assets).unwrap();
        }
        for _ in 0..count {
            campaign.action(Action::Right, assets).unwrap();
        }
    }
    campaign.action(Action::Confirm, assets).unwrap();
}

fn controller_route(campaign: &mut Campaign, tape: &str) -> Vec<(u64, i16)> {
    let mut scenes = Vec::new();
    for (ticks, buttons) in crate::session::parse_inputs(tape).unwrap() {
        for _ in 0..ticks {
            assert!(
                campaign.playing(),
                "Premature outcome at {}",
                campaign.game.tick
            );
            let was_active = campaign.game.cinematic.active;
            campaign.tick(buttons).unwrap();
            if !was_active && campaign.game.cinematic.active {
                scenes.push((campaign.game.tick, campaign.game.cinematic.sequence));
            }
            assert!(campaign.game.player().health > 0);
        }
    }
    scenes
}

fn complete_delivery(campaign: &mut Campaign) -> Save {
    let character = campaign.options.character as usize;
    let tape = [
        include_str!("../../tests/data/mission2-rikimaru.inputs"),
        include_str!("../../tests/data/mission2-ayame.inputs"),
    ][character];
    let scenes = controller_route(campaign, tape);
    eprintln!("Mission 2 character {character}: {scenes:?}");
    assert_eq!(scenes.iter().map(|s| s.1).collect::<Vec<_>>(), [10, 20]);
    assert_eq!(campaign.game.tick, [2457, 2465][character]);
    assert_eq!(campaign.game.player().health, 100);
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected delivery results");
    };
    assert_eq!(results.record.clock, [1860, 1879][character]);
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 2, 2);
    Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap()
}

fn complete_rescue(campaign: &mut Campaign, tape: &str) -> Save {
    let scenes = controller_route(campaign, tape);
    eprintln!("Mission 3 Rikimaru: {scenes:?}");
    assert_eq!(scenes.iter().map(|s| s.1).collect::<Vec<_>>(), [30, 40]);
    let boss = campaign
        .game
        .roster
        .get(campaign.game.roster.find(0x83).unwrap());
    assert_eq!((boss.health, boss.status), (0, 17));
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected rescue results");
    };
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 4, 4);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    save
}

#[test]
#[ignore = "requires TENCHU_DISC; completes Missions 1–3 from new-game stock using controller input, saves between missions and starts Mission 4"]
fn controller_routes_complete_first_three_missions() {
    let assets = assets();
    let mut campaign = Campaign::new(&assets, Options::default(), None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 5), (3, 2)]);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 5, 2]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../tests/data/mission1-rikimaru.inputs"),
    );
    eprintln!("Mission 1 controller route: {scenes:?}");
    assert_eq!(scenes.len(), 4);
    assert_eq!(scenes[1..], [(4267, 220), (6295, 150), (7274, 151)]);
    for kind in [0x80, 0x81] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, 17));
    }
    assert_eq!(campaign.game.tick, 7684);
    assert_eq!(campaign.game.player().health, 75);
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected the mission results");
    };
    assert_eq!(results.record.clock, 5803);
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 1, 1);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 2);
    assert!(matches!(resumed.screen, Screen::Inventory(_)));
    resumed.action(Action::Confirm, &assets).unwrap();
    let save = complete_delivery(&mut resumed);
    assert_eq!(save.progress.flags & 3, 3);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 3);
    equip(&mut resumed, &assets, &[(1, 5), (2, 6), (3, 1)]);
    assert_eq!(&resumed.game.player().inventory[..4], &[255, 5, 6, 1]);
    let save = complete_rescue(
        &mut resumed,
        include_str!("../../tests/data/mission3-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 4853);
    assert_eq!(resumed.game.player().health, 68);
    assert_eq!(resumed.game.clock, 3122);
    assert_eq!(&resumed.game.player().inventory[..4], &[255, 0, 6, 1]);
    assert_eq!(save.progress.flags & 7, 7);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(
        resumed.game.player().health,
        resumed.game.player().max_health
    );
    for _ in 0..30 {
        resumed.tick(0).unwrap();
    }
    assert!(resumed.playing());
    assert_eq!(resumed.game.events.outcome, Outcome::Running);
    assert!(resumed.game.player().health > 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; completes Mission 2 as Ayame with only the hook carried"]
fn ayame_delivery_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 2,
        character: 1,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    assert_eq!(
        campaign.game.player().inventory[..20],
        [255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    let save = complete_delivery(&mut campaign);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 3);
    assert_eq!(resumed.options.character, 1);
}

#[test]
#[ignore = "requires TENCHU_DISC; completes Mission 3 as Rikimaru using controller input, saves and starts Mission 4"]
fn rikimaru_rescue_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 3,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 6, 2]);
    let save = complete_rescue(
        &mut campaign,
        include_str!("../../tests/data/mission3-rikimaru.inputs"),
    );
    assert_eq!(campaign.game.tick, 5010);
    assert_eq!(campaign.game.player().health, 28);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 2, 6, 2]);
    assert_eq!(campaign.game.clock, 3279);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    resumed.action(Action::Confirm, &assets).unwrap();
    for _ in 0..30 {
        resumed.tick(0).unwrap();
    }
    assert!(resumed.playing());
    assert_eq!(resumed.game.events.outcome, Outcome::Running);
    assert_eq!(
        resumed.game.player().health,
        resumed.game.player().max_health
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; completes both Mission 4 bosses as Rikimaru using controller input, saves and starts Mission 5"]
fn rikimaru_checkpoint_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 4,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 6, 2]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../tests/data/mission4-rikimaru.inputs"),
    );
    eprintln!("Mission 4 Rikimaru: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [10, 20, 40, 50]
    );
    for kind in [0x8c, 0x8b] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, 17));
    }
    assert_eq!(campaign.game.tick, 7961);
    assert_eq!(campaign.game.player().health, 84);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 6, 0]);
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected checkpoint results");
    };
    assert_eq!(results.record.clock, 4635);
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 8, 8);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 5);
    resumed.action(Action::Confirm, &assets).unwrap();
    for _ in 0..30 {
        resumed.tick(0).unwrap();
    }
    assert!(resumed.playing());
    assert_eq!(resumed.game.events.outcome, Outcome::Running);
    assert_eq!(
        resumed.game.player().health,
        resumed.game.player().max_health
    );
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
            assert!(!crate::render_mesh::characters(&game).is_empty());
        }
    }
    assert_eq!(selections.len(), 3);
}
