use super::*;

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

fn complete_estate(campaign: &mut Campaign, tape: &str) -> Save {
    let character = campaign.options.character as usize;
    let scenes = controller_route(campaign, tape);
    eprintln!("Mission 1 character {character}: {scenes:?}");
    assert_eq!(
        scenes,
        [
            [(2118, 0), (4267, 220), (6295, 150), (7274, 151)],
            [(1486, 0), (3146, 220), (4573, 150), (5031, 151)],
        ][character]
    );
    for kind in [0x80, 0x81] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, 17));
    }
    assert_eq!(campaign.game.tick, [7684, 5421][character]);
    assert_eq!(campaign.game.player().health, [75, 70][character]);
    save_result(campaign, [5803, 3530][character])
}

fn complete_delivery(campaign: &mut Campaign) -> Save {
    let character = campaign.options.character as usize;
    let tape = [
        include_str!("../../../tests/data/mission2-rikimaru.inputs"),
        include_str!("../../../tests/data/mission2-ayame.inputs"),
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
    eprintln!(
        "Mission 3 character {}: {scenes:?}",
        campaign.options.character
    );
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

fn complete_checkpoint(campaign: &mut Campaign, tape: &str) -> Save {
    let scenes = controller_route(campaign, tape);
    eprintln!(
        "Mission 4 character {}: {scenes:?}",
        campaign.options.character
    );
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
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected checkpoint results");
    };
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 8, 8);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    save
}

fn complete_punishment(campaign: &mut Campaign, tape: &str) -> Save {
    let character = campaign.options.character as usize;
    let scenes = controller_route(campaign, tape);
    eprintln!("Mission 5 character {character}: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [&[5, 6, 10, 20, 100][..], &[5, 6, 10, 20, 40, 45][..]][character]
    );
    if character == 1 {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(0x8e).unwrap());
        assert_eq!((boss.health, boss.status), (0, 17));
    }
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected punishment results");
    };
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    assert_eq!(campaign.progress.flags & 16, 16);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    save
}

#[test]
#[ignore = "requires TENCHU_DISC; completes all ten missions from new-game stock using controller input and saves between missions"]
fn rikimaru_campaign_controller_routes_reach_completion() {
    let assets = assets();
    let mut campaign = Campaign::new(&assets, Options::default(), None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 5), (3, 2)]);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 5, 2]);
    let save = complete_estate(
        &mut campaign,
        include_str!("../../../tests/data/mission1-rikimaru.inputs"),
    );
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
        include_str!("../../../tests/data/mission3-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 4853);
    assert_eq!(resumed.game.player().health, 68);
    assert_eq!(resumed.game.clock, 3122);
    assert_eq!(&resumed.game.player().inventory[..4], &[255, 0, 6, 1]);
    assert_eq!(save.progress.flags & 7, 7);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    equip(&mut resumed, &assets, &[(1, 2), (2, 6), (3, 3), (4, 4)]);
    assert_eq!(&resumed.game.player().inventory[..5], &[255, 2, 6, 3, 4]);
    let save = complete_checkpoint(
        &mut resumed,
        include_str!("../../../tests/data/mission4-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 7742);
    assert_eq!(resumed.game.player().health, 16);
    assert_eq!(resumed.game.clock, 4416);
    assert_eq!(&resumed.game.player().inventory[..5], &[255, 0, 6, 1, 0]);
    assert_eq!(save.progress.flags & 15, 15);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 5);
    resumed.action(Action::Confirm, &assets).unwrap();
    let save = complete_punishment(
        &mut resumed,
        include_str!("../../../tests/data/mission5-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 7300);
    assert_eq!(resumed.game.player().health, 72);
    assert_eq!(resumed.game.clock, 3194);
    assert_eq!(&resumed.game.player().inventory[..4], &[255, 0, 0, 0]);
    assert_eq!(save.progress.flags & 31, 31);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 6);
    equip(&mut resumed, &assets, &[(1, 1), (3, 2), (4, 2), (5, 4)]);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 1, 0, 2, 2, 4]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission6-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 6 saved campaign: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [20, 110, 150]
    );
    assert_eq!(resumed.game.tick, 5819);
    assert_eq!(resumed.game.player().health, 12);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 0, 2, 0, 2]);
    assert_eq!(resumed.progress.flags & 63, 63);
    let save = save_result(&mut resumed, 3548);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 7);
    equip(&mut resumed, &assets, &[(3, 2), (4, 1), (5, 2), (6, 4)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 2, 1, 2, 4]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission7-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 7 saved campaign: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [10, 20, 100]
    );
    assert_eq!(resumed.game.tick, 2817);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 1, 0, 2, 4]
    );
    let boss = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x85).unwrap());
    assert_eq!((boss.health, boss.status), (0, 17));
    assert_eq!(resumed.progress.flags & 127, 127);
    let save = save_result(&mut resumed, 2186);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 8);
    equip(&mut resumed, &assets, &[(1, 1), (3, 2), (4, 1), (5, 3)]);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 1, 0, 2, 1, 3]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission8-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 8 saved campaign: {scenes:?}");
    assert_eq!(scenes, [(5336, 11)]);
    assert_eq!(resumed.game.tick, 5566);
    assert_eq!(resumed.game.player().health, 50);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 0, 1, 0, 0]);
    assert_eq!(resumed.game.player().position, [-18000, -53500, 143957]);
    assert_eq!(resumed.progress.flags & 255, 255);
    let save = save_result(&mut resumed, 5335);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 9);
    equip(&mut resumed, &assets, &[(2, 6), (3, 2), (6, 6)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 2, 0, 0, 6]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission9-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 9 saved campaign: {scenes:?}");
    assert_eq!(scenes.iter().map(|s| s.1).collect::<Vec<_>>(), [10, 50]);
    assert_eq!(resumed.game.tick, 5112);
    assert_eq!(resumed.game.player().health, 22);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 0, 2]
    );
    let boss = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x87).unwrap());
    assert_eq!((boss.health, boss.status), (0, 17));
    assert_eq!(resumed.progress.flags & 511, 511);
    let save = save_result(&mut resumed, 4026);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 10);
    equip(&mut resumed, &assets, &[(2, 6), (3, 1), (5, 1), (6, 2)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 1, 0, 1, 2]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission10-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 10 saved campaign: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [7, 10, 15, 35, 40]
    );
    assert_eq!(resumed.game.tick, 14348);
    assert_eq!(resumed.game.player().health, 30);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 0, 1]
    );
    assert_eq!(resumed.progress.flags & 0x3ff, 0x3ff);
    save_and_complete(&mut resumed, &assets, 10927);
}

#[test]
#[ignore = "requires TENCHU_DISC; completes Ayame's first six missions from new-game stock, saves between missions and starts Mission 7"]
fn ayame_campaign_controller_routes_complete_first_six_missions() {
    let assets = assets();
    let options = Options {
        character: 1,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 5), (3, 2)]);
    let save = complete_estate(
        &mut campaign,
        include_str!("../../../tests/data/mission1-ayame.inputs"),
    );
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 5, 2]);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 2);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission2-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1, 10), (2029, 20)]);
    assert_eq!(resumed.game.tick, 2464);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(resumed.progress.flags & 3, 3);
    let save = save_result(&mut resumed, 1878);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 3);
    equip(&mut resumed, &assets, &[(1, 6), (2, 5), (3, 2)]);
    let save = complete_rescue(
        &mut resumed,
        include_str!("../../../tests/data/mission3-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 5471);
    assert_eq!(resumed.game.player().health, 64);
    assert_eq!(resumed.game.clock, 3684);
    assert_eq!(&resumed.game.player().inventory[..4], &[255, 2, 5, 2]);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    equip(&mut resumed, &assets, &[(1, 3), (3, 3), (4, 4), (5, 3)]);
    let save = complete_checkpoint(
        &mut resumed,
        include_str!("../../../tests/data/mission4-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 7476);
    assert_eq!(resumed.game.player().health, 67);
    assert_eq!(resumed.game.clock, 4905);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 0, 0, 0, 2]);
    assert_eq!(resumed.progress.flags & 15, 15);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 5);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[(2, 6), (3, 2), (5, 4), (6, 2)]);
    let save = complete_punishment(
        &mut resumed,
        include_str!("../../../tests/data/mission5-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 7469);
    assert_eq!(resumed.game.player().health, 27);
    assert_eq!(resumed.game.clock, 3598);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 1, 0, 4, 2]
    );
    assert_eq!(resumed.progress.flags & 31, 31);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 6);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[(1, 2), (3, 1), (5, 5), (6, 2)]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../tests/data/mission6-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1181, 20), (2949, 110), (4627, 150)]);
    assert_eq!(resumed.game.tick, 5037);
    assert_eq!(resumed.game.player().health, 25);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 0, 0, 4, 0]
    );
    let leader = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x84).unwrap());
    assert_eq!((leader.health, leader.status), (0, 17));
    let onikage = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x8a).unwrap());
    assert_eq!(onikage.health, 94);
    assert_eq!(resumed.progress.flags & 63, 63);
    save_and_start_next(&mut resumed, &assets, 2776);
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
        include_str!("../../../tests/data/mission3-rikimaru.inputs"),
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
    let save = complete_checkpoint(
        &mut campaign,
        include_str!("../../../tests/data/mission4-rikimaru.inputs"),
    );
    assert_eq!(campaign.game.tick, 7961);
    assert_eq!(campaign.game.player().health, 84);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 6, 0]);
    assert_eq!(campaign.game.clock, 4635);
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
#[ignore = "requires TENCHU_DISC; completes both Mission 5 branches using controller input, saves and starts Mission 6"]
fn punishment_controller_routes_reach_results() {
    let assets = assets();
    let tapes = [
        include_str!("../../../tests/data/mission5-rikimaru.inputs"),
        include_str!("../../../tests/data/mission5-ayame.inputs"),
    ];
    for (character, tape) in tapes.iter().enumerate() {
        let options = Options {
            mission: 5,
            character: character as i16,
            ..Options::default()
        };
        let mut campaign = Campaign::new(&assets, options, None).unwrap();
        campaign.action(Action::Confirm, &assets).unwrap();
        let gear = if character == 0 {
            &[][..]
        } else {
            &[(1, 6), (2, 6), (3, 2)][..]
        };
        equip(&mut campaign, &assets, gear);
        let save = complete_punishment(&mut campaign, tape);
        assert_eq!(campaign.game.tick, [7586, 7618][character]);
        assert_eq!(campaign.game.player().health, [56, 80][character]);
        assert_eq!(
            campaign.game.player().inventory[..4],
            [[255, 0, 0, 0], [255, 1, 6, 1]][character]
        );
        assert_eq!(campaign.game.clock, [3480, 3747][character]);
        let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
        resumed.action(Action::Confirm, &assets).unwrap();
        assert_eq!(resumed.options.mission, 6);
        assert_eq!(resumed.options.character as usize, character);
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
}

fn save_result(campaign: &mut Campaign, clock: i32) -> Save {
    let Screen::Results(results) = &campaign.screen else {
        panic!("Expected mission results");
    };
    assert_eq!(results.record.clock, clock);
    assert!(results.record.played());
    assert_eq!(results.record, campaign.game.record());
    let flag = 1 << (campaign.options.mission - 1);
    assert_eq!(campaign.progress.flags & flag, flag);
    let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
    assert_eq!(save.progress, campaign.progress);
    save
}

fn save_and_start_next(campaign: &mut Campaign, assets: &GameAssets<FileSource>, clock: i32) {
    let save = save_result(campaign, clock);
    let mut resumed = Campaign::new(assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, assets).unwrap();
    assert_eq!(resumed.options.mission, campaign.options.mission + 1);
    assert_eq!(resumed.options.character, campaign.options.character);
    resumed.action(Action::Confirm, assets).unwrap();
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

fn save_and_complete(campaign: &mut Campaign, assets: &GameAssets<FileSource>, clock: i32) {
    for kind in [0x87, 0x88] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, 17));
    }
    let save = save_result(campaign, clock);
    assert_eq!(campaign.progress.flags & 0x600, 0x600);
    assert_eq!(
        campaign.progress.highest[campaign.options.character as usize],
        10
    );
    campaign.action(Action::Confirm, assets).unwrap();
    assert!(matches!(campaign.screen, Screen::Complete));
    campaign.action(Action::Confirm, assets).unwrap();
    assert!(matches!(campaign.screen, Screen::Launcher));
    let mut resumed = Campaign::new(assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, assets).unwrap();
    assert_eq!(resumed.options.mission, 10);
    assert_eq!(resumed.options.character, campaign.options.character);
    assert_eq!(resumed.progress, campaign.progress);
    assert!(matches!(resumed.screen, Screen::Inventory(_)));
}

#[test]
#[ignore = "requires TENCHU_DISC; grapples the statue, completes both Mission 6 encounters, exercises guard ledge release and starts Mission 7 from a save"]
fn rikimaru_cult_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 6,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../../tests/data/mission6-rikimaru.inputs"),
    );
    eprintln!("Mission 6 Rikimaru: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [20, 110, 150]
    );
    assert_eq!(campaign.game.tick, 5458);
    assert_eq!(campaign.game.player().health, 5);
    assert_eq!(
        &campaign.game.player().inventory[..6],
        &[255, 0, 6, 2, 0, 0]
    );
    save_and_start_next(&mut campaign, &assets, 3187);
}

#[test]
#[ignore = "requires TENCHU_DISC; boards the pirate ship, defeats Balma with controller input, saves and starts Mission 8"]
fn rikimaru_pirates_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 7,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../../tests/data/mission7-rikimaru.inputs"),
    );
    eprintln!("Mission 7 Rikimaru: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [10, 20, 100]
    );
    assert_eq!(campaign.game.tick, 3309);
    assert_eq!(campaign.game.player().health, 100);
    assert_eq!(
        &campaign.game.player().inventory[..6],
        &[255, 0, 6, 1, 0, 1]
    );
    let boss = campaign
        .game
        .roster
        .get(campaign.game.roster.find(0x85).unwrap());
    assert_eq!((boss.health, boss.status), (0, 17));
    save_and_start_next(&mut campaign, &assets, 2678);
}

#[test]
#[ignore = "requires TENCHU_DISC; climbs the mountain and obtains the cure with controller input, saves and starts Mission 9"]
fn rikimaru_mountain_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 8,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../../tests/data/mission8-rikimaru.inputs"),
    );
    assert_eq!(scenes, [(10502, 11)]);
    assert_eq!(campaign.game.tick, 10732);
    assert_eq!(campaign.game.player().health, 100);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 6, 0]);
    assert_eq!(campaign.game.player().position, [-18000, -53500, 143957]);
    save_and_start_next(&mut campaign, &assets, 10501);
}

#[test]
#[ignore = "requires TENCHU_DISC; climbs the castle, defeats Onikage with controller input, saves and starts Mission 10"]
fn rikimaru_castle_controller_route_reaches_results() {
    let assets = assets();
    let options = Options {
        mission: 9,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../../tests/data/mission9-rikimaru.inputs"),
    );
    eprintln!("Mission 9 Rikimaru: {scenes:?}");
    assert_eq!(scenes.iter().map(|s| s.1).collect::<Vec<_>>(), [10, 50]);
    assert_eq!(campaign.game.tick, 4535);
    assert_eq!(campaign.game.player().health, 53);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 6, 1]);
    let boss = campaign
        .game
        .roster
        .get(campaign.game.roster.find(0x87).unwrap());
    assert_eq!((boss.health, boss.status), (0, 17));
    save_and_start_next(&mut campaign, &assets, 3449);
}

#[test]
#[ignore = "requires TENCHU_DISC; completes both final mission bosses with controller input and verifies saved completion"]
fn rikimaru_final_controller_route_reaches_completion() {
    let assets = assets();
    let options = Options {
        mission: 10,
        ..Options::default()
    };
    let mut campaign = Campaign::new(&assets, options, None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 6), (3, 2)]);
    let scenes = controller_route(
        &mut campaign,
        include_str!("../../../tests/data/mission10-rikimaru.inputs"),
    );
    eprintln!("Mission 10 Rikimaru: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [7, 10, 15, 35, 40]
    );
    assert_eq!(campaign.game.tick, 15556);
    assert_eq!(campaign.game.player().health, 10);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 6, 0]);
    save_and_complete(&mut campaign, &assets, 12135);
}
