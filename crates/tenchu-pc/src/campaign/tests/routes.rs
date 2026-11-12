use super::*;
mod legacy;
use tenchu_core::actor::State as ActorState;

fn equip(campaign: &mut Campaign, assets: &GameAssets<DiscSource>, items: &[(usize, usize)]) {
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
            assert!(
                campaign.game.player().health > 0,
                "Player died in mission {} at tick {} (clock {}, scenes {scenes:?})",
                campaign.options.mission,
                campaign.game.tick,
                campaign.game.clock,
            );
        }
    }
    scenes
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
        assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
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

#[test]
#[ignore = "requires TENCHU_DISC; completes training with both characters, saves their rankings and retries with fixed training equipment"]
fn training_controller_routes_save_rankings_and_retry() {
    let assets = assets();
    for (character, tape, ticks, clock, health, closing) in [
        (
            0,
            include_str!("../../../tests/data/training-rikimaru.inputs"),
            4062,
            3842,
            70,
            3903,
        ),
        (
            1,
            include_str!("../../../tests/data/training-ayame.inputs"),
            3724,
            3513,
            90,
            3574,
        ),
    ] {
        let options = Options {
            mission: 0,
            character,
            ..Options::default()
        };
        let mut campaign = Campaign::new(&assets, options, None).unwrap();
        let stock = campaign.progress.stock;
        campaign.action(Action::Confirm, &assets).unwrap();
        equip(&mut campaign, &assets, &[]);
        assert_eq!(&campaign.game.player().inventory[..4], &[255, 5, 0, 0]);
        assert_eq!(
            controller_route(&mut campaign, tape),
            [(1, 20), (closing, 5)]
        );
        assert_eq!(campaign.game.tick, ticks);
        assert_eq!(campaign.game.player().health, health);
        assert_eq!(&campaign.game.player().inventory[..4], &[255, 3, 0, 0]);
        let mut replayed = crate::recording::Start::parse(tape)
            .unwrap()
            .expect("Shared training tape needs its launch metadata")
            .load(&assets)
            .unwrap();
        for (count, buttons) in crate::session::parse_inputs(tape).unwrap() {
            for _ in 0..count {
                replayed.step(buttons).unwrap();
            }
        }
        assert_eq!(
            crate::session::tests::state(&campaign.game),
            crate::session::tests::state(&replayed)
        );
        for id in 1..=7 {
            let enemy = campaign.game.roster.get(id);
            assert_eq!((enemy.health, enemy.status), (0, ActorState::Dead));
        }
        let Screen::Results(results) = &campaign.screen else {
            panic!("Expected training results")
        };
        assert_eq!(results.record, campaign.game.record());
        assert_eq!(results.record.clock, clock);
        assert_eq!(results.score.grade, 0);
        assert_eq!(results.training_place, Some(0));
        assert_eq!(campaign.progress.stock, stock);
        assert_eq!(campaign.progress.highest, [0, 0]);
        assert_eq!(campaign.progress.flags, 1 << 31);
        let ranked = campaign.progress.training[0];
        assert_eq!(
            (ranked.character, ranked.grade, ranked.clock),
            (character as u8, 0, clock)
        );
        let save = Save::decode(&campaign.save_request.take().unwrap().encode()).unwrap();
        assert_eq!(save.progress, campaign.progress);
        let mut resumed = Campaign::new(&assets, Options::default(), Some(save)).unwrap();
        resumed.action(Action::Confirm, &assets).unwrap();
        assert_eq!(
            (resumed.options.character, resumed.options.mission),
            (character, 0)
        );
        resumed.action(Action::Confirm, &assets).unwrap();
        assert!(resumed.playing());
        assert_eq!(resumed.game.player().health, 100);
        assert_eq!(&resumed.game.player().inventory[..4], &[255, 5, 0, 0]);
        assert_eq!(resumed.progress.stock, stock);
        campaign.action(Action::Confirm, &assets).unwrap();
        assert!(matches!(campaign.screen, Screen::Inventory(_)));
        campaign.action(Action::Confirm, &assets).unwrap();
        assert!(campaign.playing());
        assert_eq!(campaign.options.mission, 0);
        assert_eq!(&campaign.game.player().inventory[..4], &[255, 5, 0, 0]);
        assert_eq!(campaign.progress.training[0], ranked);
        assert_eq!(campaign.progress.stock, stock);
    }
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

fn save_and_start_next(campaign: &mut Campaign, assets: &GameAssets<DiscSource>, clock: i32) {
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
    assert_eq!((boss.health, boss.status), (0, ActorState::Dead));
    save_and_start_next(&mut campaign, &assets, 3449);
}
