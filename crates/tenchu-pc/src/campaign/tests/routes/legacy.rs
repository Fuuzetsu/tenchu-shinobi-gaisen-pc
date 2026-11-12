//! Unverified historical PC tapes. Keep the failed expectations for diagnosis;
//! these are not console recordings or passing campaign-completion evidence.

use super::*;

fn complete_estate(campaign: &mut Campaign, tape: &str) -> Save {
    let character = campaign.options.character as usize;
    let scenes = controller_route(campaign, tape);
    eprintln!("Mission 1 character {character}: {scenes:?}");
    assert_eq!(
        scenes,
        [
            [(2118, 0), (3189, 220), (4630, 150), (5257, 151)],
            [(1486, 0), (2555, 220), (3976, 150), (4444, 151)],
        ][character]
    );
    for kind in [0x80, 0x81] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    }
    assert_eq!(campaign.game.tick, [5667, 4834][character]);
    assert_eq!(campaign.game.player().health, [64, 94][character]);
    save_result(campaign, [3786, 2943][character])
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
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
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
        assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
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
#[ignore = "requires TENCHU_DISC; historical PC tape, currently Mission 6 stock differs; not a console parity reference"]
fn unverified_rikimaru_campaign() {
    let assets = assets();
    let mut campaign = Campaign::new(&assets, Options::default(), None).unwrap();
    campaign.action(Action::Confirm, &assets).unwrap();
    equip(&mut campaign, &assets, &[(1, 6), (2, 5), (3, 2)]);
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 6, 5, 2]);
    let save = complete_estate(
        &mut campaign,
        include_str!("../../../../tests/data/mission1-rikimaru.inputs"),
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
    equip(&mut resumed, &assets, &[(1, 2), (2, 6), (3, 3), (4, 2)]);
    assert_eq!(&resumed.game.player().inventory[..5], &[255, 2, 6, 3, 2]);
    let save = complete_rescue(
        &mut resumed,
        include_str!("../../../../tests/data/mission3-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 5952);
    assert_eq!(resumed.game.player().health, 7);
    assert_eq!(resumed.game.clock, 4221);
    assert_eq!(&resumed.game.player().inventory[..5], &[255, 0, 5, 3, 0]);
    assert_eq!(save.progress.flags & 7, 7);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    equip(&mut resumed, &assets, &[(2, 6), (3, 3), (5, 4), (6, 2)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 3, 0, 4, 2]
    );
    let save = complete_checkpoint(
        &mut resumed,
        include_str!("../../../../tests/data/mission4-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 9128);
    assert_eq!(resumed.game.player().health, 57);
    assert_eq!(resumed.game.clock, 5802);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 3, 1, 0, 3, 2]
    );
    assert_eq!(save.progress.flags & 15, 15);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 5);
    equip(&mut resumed, &assets, &[(2, 6), (3, 2), (5, 3), (6, 2)]);
    let save = complete_punishment(
        &mut resumed,
        include_str!("../../../../tests/data/mission5-rikimaru-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 8034);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(resumed.game.clock, 3928);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 2, 2]
    );
    assert_eq!(save.progress.flags & 31, 31);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 6);
    equip(&mut resumed, &assets, &[(3, 2), (4, 1), (5, 2), (6, 3)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 2, 1, 2, 3]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission6-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 6 saved campaign: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [20, 110, 150]
    );
    assert_eq!(resumed.game.tick, 6139);
    assert_eq!(resumed.game.player().health, 50);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 2, 0, 1, 3]
    );
    assert_eq!(resumed.progress.flags & 63, 63);
    let save = save_result(&mut resumed, 3868);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 7);
    equip(&mut resumed, &assets, &[(1, 1), (2, 6), (3, 2), (5, 1)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 1, 6, 2, 0, 1, 0]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission7-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 7 saved campaign: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [10, 20, 100]
    );
    assert_eq!(resumed.game.tick, 3744);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 1, 0, 1, 0]
    );
    let boss = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x85).unwrap());
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    assert_eq!(resumed.progress.flags & 127, 127);
    let save = save_result(&mut resumed, 3113);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 8);
    equip(&mut resumed, &assets, &[(3, 2), (4, 1), (5, 1)]);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 0, 2, 1, 1]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission8-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 8 saved campaign: {scenes:?}");
    assert_eq!(scenes, [(8218, 11)]);
    assert_eq!(resumed.game.tick, 8448);
    assert_eq!(resumed.game.player().health, 74);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 0, 0, 1, 0]);
    assert_eq!(resumed.game.player().position, [-18000, -53500, 143957]);
    assert_eq!(resumed.progress.flags & 255, 255);
    let save = save_result(&mut resumed, 8217);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 9);
    equip(&mut resumed, &assets, &[(2, 6), (4, 1), (5, 2), (6, 6)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 1, 2, 6]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission9-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 9 saved campaign: {scenes:?}");
    assert_eq!(scenes.iter().map(|s| s.1).collect::<Vec<_>>(), [10, 50]);
    assert_eq!(resumed.game.tick, 6292);
    assert_eq!(resumed.game.player().health, 3);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 1, 6]
    );
    let boss = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x87).unwrap());
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    assert_eq!(resumed.progress.flags & 511, 511);
    let save = save_result(&mut resumed, 5206);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 10);
    equip(&mut resumed, &assets, &[(2, 6), (3, 1), (5, 1), (6, 6)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 1, 0, 1, 6]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission10-rikimaru-campaign.inputs"),
    );
    eprintln!("Mission 10 saved campaign: {scenes:?}");
    assert_eq!(
        scenes,
        [(1499, 7), (1589, 10), (5370, 15), (10285, 35), (10966, 40)]
    );
    assert_eq!(resumed.game.tick, 15082);
    assert_eq!(resumed.game.player().health, 30);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 0, 5]
    );
    assert_eq!(resumed.progress.flags & 0x3ff, 0x3ff);
    save_and_complete(&mut resumed, &assets, 11661);
}

#[test]
#[ignore = "requires TENCHU_DISC; historical PC tape, currently dies in Mission 4; not a console parity reference"]
fn unverified_ayame_campaign() {
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
        include_str!("../../../../tests/data/mission1-ayame.inputs"),
    );
    assert_eq!(&campaign.game.player().inventory[..4], &[255, 0, 5, 2]);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 2);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission2-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1, 10), (2029, 20)]);
    assert_eq!(resumed.game.tick, 2464);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(resumed.progress.flags & 3, 3);
    let save = save_result(&mut resumed, 1878);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 3);
    equip(&mut resumed, &assets, &[(1, 2), (2, 5), (3, 3), (4, 3)]);
    let save = complete_rescue(
        &mut resumed,
        include_str!("../../../../tests/data/mission3-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 5883);
    assert_eq!(resumed.game.player().health, 46);
    assert_eq!(resumed.game.clock, 4096);
    assert_eq!(&resumed.game.player().inventory[..5], &[255, 0, 5, 3, 0]);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 4);
    equip(&mut resumed, &assets, &[(1, 1), (3, 3), (5, 2), (6, 2)]);
    let save = complete_checkpoint(
        &mut resumed,
        include_str!("../../../../tests/data/mission4-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 12062);
    assert_eq!(resumed.game.player().health, 24);
    assert_eq!(resumed.game.clock, 9491);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 0, 0, 2, 2]
    );
    assert_eq!(resumed.progress.flags & 15, 15);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 5);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[(2, 6), (5, 3), (6, 2)]);
    let save = complete_punishment(
        &mut resumed,
        include_str!("../../../../tests/data/mission5-ayame-campaign.inputs"),
    );
    assert_eq!(resumed.game.tick, 7919);
    assert_eq!(resumed.game.player().health, 46);
    assert_eq!(resumed.game.clock, 4048);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 1, 2]
    );
    assert_eq!(resumed.progress.flags & 31, 31);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 6);
    assert_eq!(resumed.options.character, 1);
    equip(&mut resumed, &assets, &[(2, 6), (5, 2), (6, 5)]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission6-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1182, 20), (3387, 110), (5983, 150)]);
    assert_eq!(resumed.game.tick, 6393);
    assert_eq!(resumed.game.player().health, 5);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 0, 0, 5]
    );
    let leader = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x84).unwrap());
    assert_eq!((leader.health, leader.status), (0, ActorState::Dead));
    let onikage = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x8a).unwrap());
    assert_eq!(onikage.health, 92);
    assert_eq!(resumed.progress.flags & 63, 63);
    let save = save_result(&mut resumed, 4132);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 7);
    equip(&mut resumed, &assets, &[(2, 6), (3, 1)]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission7-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1419, 10), (1509, 20), (2306, 100)]);
    assert_eq!(resumed.game.tick, 2546);
    assert_eq!(resumed.game.player().health, 100);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 0, 6, 0, 0, 0]);
    let balma = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x85).unwrap());
    assert_eq!((balma.health, balma.status), (0, ActorState::Dead));
    let save = save_result(&mut resumed, 1955);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 8);
    equip(&mut resumed, &assets, &[(1, 1), (3, 1), (4, 1), (5, 1)]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission8-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(7921, 11)]);
    assert_eq!(resumed.game.tick, 8126);
    assert_eq!(resumed.game.player().health, 56);
    assert_eq!(&resumed.game.player().inventory[..6], &[255, 1, 0, 0, 1, 0]);
    assert_eq!(resumed.progress.flags & 255, 255);
    let save = save_result(&mut resumed, 7920);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 9);
    equip(&mut resumed, &assets, &[(1, 2), (2, 6), (3, 1), (4, 1)]);
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission9-ayame-campaign.inputs"),
    );
    assert_eq!(scenes, [(1946, 10), (5461, 50)]);
    assert_eq!(resumed.game.tick, 5741);
    assert_eq!(resumed.game.player().health, 67);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 6, 0, 1, 0, 0]
    );
    let boss = resumed
        .game
        .roster
        .get(resumed.game.roster.find(0x87).unwrap());
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    assert_eq!(resumed.progress.flags & 511, 511);
    let save = save_result(&mut resumed, 4620);
    let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
    resumed.action(Action::Confirm, &assets).unwrap();
    assert_eq!(resumed.options.mission, 10);
    equip(&mut resumed, &assets, &[(1, 2), (4, 1), (5, 1), (6, 6)]);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 2, 0, 0, 1, 1, 6]
    );
    let scenes = controller_route(
        &mut resumed,
        include_str!("../../../../tests/data/mission10-ayame-campaign.inputs"),
    );
    assert_eq!(
        scenes,
        [(1449, 7), (1539, 10), (4700, 15), (9303, 35), (10004, 40)]
    );
    assert_eq!(resumed.game.tick, 16015);
    assert_eq!(resumed.game.player().health, 7);
    assert_eq!(
        &resumed.game.player().inventory[..7],
        &[255, 0, 0, 0, 1, 0, 4]
    );
    assert_eq!(resumed.progress.flags & 0x3ff, 0x3ff);
    save_and_complete(&mut resumed, &assets, 12629);
}

#[test]
#[ignore = "requires TENCHU_DISC; historical PC tape, currently dies before the ending; not a console parity reference"]
fn unverified_rikimaru_rescue() {
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
        include_str!("../../../../tests/data/mission3-rikimaru.inputs"),
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
#[ignore = "requires TENCHU_DISC; historical PC tape, currently dies before the ending; not a console parity reference"]
fn unverified_punishment() {
    let assets = assets();
    let tapes = [
        include_str!("../../../../tests/data/mission5-rikimaru.inputs"),
        include_str!("../../../../tests/data/mission5-ayame.inputs"),
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

fn save_and_complete(campaign: &mut Campaign, assets: &GameAssets<DiscSource>, clock: i32) {
    for kind in [0x87, 0x88] {
        let boss = campaign
            .game
            .roster
            .get(campaign.game.roster.find(kind).unwrap());
        assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
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
#[ignore = "requires TENCHU_DISC; historical PC tape, currently misses the final scene; not a console parity reference"]
fn unverified_rikimaru_pirates() {
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
        include_str!("../../../../tests/data/mission7-rikimaru.inputs"),
    );
    eprintln!("Mission 7 Rikimaru: {scenes:?}");
    assert_eq!(
        scenes.iter().map(|s| s.1).collect::<Vec<_>>(),
        [10, 20, 100]
    );
    assert_eq!(campaign.game.tick, 3461);
    assert_eq!(campaign.game.player().health, 93);
    assert_eq!(
        &campaign.game.player().inventory[..6],
        &[255, 1, 6, 1, 0, 1]
    );
    let boss = campaign
        .game
        .roster
        .get(campaign.game.roster.find(0x85).unwrap());
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    save_and_start_next(&mut campaign, &assets, 2830);
}

#[test]
#[ignore = "requires TENCHU_DISC; historical PC tape, currently dies before completion; not a console parity reference"]
fn unverified_rikimaru_final() {
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
        include_str!("../../../../tests/data/mission10-rikimaru.inputs"),
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
