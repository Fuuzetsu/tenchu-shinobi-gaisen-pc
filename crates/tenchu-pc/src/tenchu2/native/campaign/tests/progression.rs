use super::*;
use tenchu_core::tenchu2::{actor::State, mission::Outcome};

type Game = Playback<crate::source::DiscSource>;

fn prepare_success(playback: &mut Game) -> Outcome {
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("not gameplay")
    };
    let player = session.world.player;
    session.world.actors[player].actor.invulnerable = true;
    for _ in 0..60 {
        playback.step(0).unwrap();
    }
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("area finished before its objective was exercised")
    };
    assert!(session.world.mission.input_allowed());
    let world = &mut session.world;
    match world.mission.objective {
        Objective::Reach { region, .. } => {
            let actor = &mut world.actors[player].actor;
            actor.position = region.center.map(|v| i32::from(v) * 1000).into();
            actor.previous = actor.position;
            actor.velocity = Default::default();
            assert_ne!(actor.state, State::Swimming);
            world.publish(player, &session.assets);
            world.update_camera(&session.assets, true).unwrap();
            Outcome::Arrived
        }
        Objective::Weaken { targets } | Objective::Defeat { targets } => {
            let weaken = matches!(world.mission.objective, Objective::Weaken { .. });
            for (kind, index) in targets.into_iter().zip(world.mission.targets) {
                if kind == 0 {
                    assert!(index.is_none());
                    continue;
                }
                let index = index.expect("campaign boss is missing from its area");
                assert_ne!(index, player);
                let actor = &mut world.actors[index].actor;
                assert_eq!(actor.kind, kind);
                assert!(actor.health > 0);
                actor.health = if weaken { actor.max_health / 2 - 1 } else { 0 };
            }
            if weaken {
                Outcome::BossWeakened
            } else {
                Outcome::TargetsDefeated
            }
        }
        Objective::EliminateUnseen { count } => {
            world.statistics.killed = count.try_into().unwrap();
            world.statistics.stealth_kills = world.statistics.killed;
            assert_eq!(world.ai.detections, 0);
            Outcome::QuotaReached
        }
        Objective::EliminateAll => {
            // Direct kill-count setup exercises the campaign quota, not combat
            // or death credit (covered by the recorded damage/death fixtures).
            world.statistics.killed =
                world.mission.population.guards + world.mission.population.bosses;
            assert!(world.statistics.killed > 0);
            Outcome::AreaCleared
        }
        Objective::Scripted => panic!("Retail campaign contains an imported objective"),
    }
}

fn finish_sequence(playback: &mut Game, outcome: Option<Outcome>) {
    let sequence = playback.run.sequence;
    let generation = playback.generation;
    let mut observed = None;
    for tick in 0..6000 {
        if let Content::Gameplay(session) = &playback.content {
            match session.world.mission.phase {
                Phase::Ending { outcome, .. } | Phase::Finished { outcome, .. } => {
                    observed = Some(outcome)
                }
                Phase::Playing { .. } => {}
            }
        }
        let buttons = if tick % 15 == 0
            && matches!(
                playback.content,
                Content::Story { .. } | Content::Movie(_) | Content::Introduction(_)
            ) {
            0x2000
        } else {
            0
        };
        let update = playback
            .step(buttons)
            .unwrap_or_else(|e| panic!("sequence {sequence}: {e}"));
        assert!(!update.quit);
        if playback.generation != generation {
            assert_eq!(observed, outcome, "sequence {sequence}");
            return;
        }
    }
    panic!("sequence {sequence} did not finish");
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; controlled exits, boss health and quotas traverse all three full campaigns, results, saved records and Tatsumaru unlock"]
fn native_campaigns_complete_through_actual_area_transitions() {
    use super::super::super::save::SaveFile;
    let path = std::env::temp_dir().join(format!(
        "tenchu-jp-campaign-audit-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut completed = 0;
    let mut areas = 0;
    let mut sequences = 0;
    for hero in 0..3 {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let campaign = Campaign::load(&disc).unwrap();
        let first = campaign.starts[hero][0].unwrap();
        let mut items = [0; 21];
        items[0] = 255;
        items[4] = 5;
        let mut playback = Playback::load(
            disc,
            first,
            2,
            17,
            Some(items),
            Some(SaveFile::at(path.clone())),
        )
        .unwrap();
        assert_eq!(playback.run.hero, hero);
        assert_eq!(playback.run.progress.tatsumaru_unlocked, hero == 2);
        let mut visited = std::collections::BTreeSet::new();
        let mut results = std::collections::BTreeSet::new();
        loop {
            let sequence = playback.run.sequence;
            assert!(
                visited.insert(sequence),
                "hero {hero}: cycle at sequence {sequence}"
            );
            let descriptor = campaign.sequences[sequence];
            let outcome = if matches!(playback.content, Content::Gameplay(_)) {
                areas += 1;
                Some(prepare_success(&mut playback))
            } else {
                None
            };
            if matches!(playback.content, Content::Results(_)) {
                let mission = playback.run.mission;
                assert_eq!(campaign.result_mission(hero, sequence), Some(mission));
                assert!(results.insert(mission));
                let record = playback.run.progress.records[hero][mission];
                assert!(!matches!(record.time_tenths, 0 | 59999));
                let saved = std::fs::read(&path).unwrap();
                assert_eq!(
                    serde_json::to_value(SaveFile::at(path.clone()).load().unwrap()).unwrap(),
                    serde_json::to_value(&playback.run.progress).unwrap()
                );
                for _ in 0..60 {
                    playback.step(0).unwrap();
                }
                assert_eq!(
                    std::fs::read(&path).unwrap(),
                    saved,
                    "results awarded twice"
                );
                eprintln!(
                    "hero {hero}, mission {mission}: result {sequence}, rank {}, time {}, Tatsumaru {}",
                    record.rank, record.time_tenths, playback.run.progress.tatsumaru_unlocked
                );
                playback.action = Some(Action::Continue);
                completed += 1;
            }
            finish_sequence(&mut playback, outcome);
            sequences += 1;
            if let Some(next) = descriptor.next {
                assert_eq!(
                    playback.run.sequence, next,
                    "hero {hero}, sequence {sequence}"
                );
            } else {
                assert!(matches!(playback.content, Content::Complete));
                break;
            }
            eprintln!("hero {hero}: {sequence} -> {}", playback.run.sequence);
        }
        let expected: std::collections::BTreeSet<_> = campaign.starts[hero]
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|_| i))
            .collect();
        assert_eq!(results, expected);
        assert_eq!(playback.run.progress.tatsumaru_unlocked, hero >= 1);
        eprintln!(
            "hero {hero}: {} sequences, {} completed missions",
            visited.len(),
            results.len()
        );
    }
    assert_eq!((sequences, areas, completed), (231, 56, 29));
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; every gameplay area follows its actual death/detection failure route, preserves progress and retries its mission's first area"]
fn native_campaign_failures_return_to_the_correct_retry_area() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut deaths = 0;
    let mut detections = 0;
    for hero in 0..3 {
        for (sequence, descriptor) in campaign.sequences.iter().enumerate() {
            let Kind::Gameplay(objective) = descriptor.kind else {
                continue;
            };
            let Some(mission) = campaign.mission(hero, sequence) else {
                continue;
            };
            let unseen = matches!(
                objective,
                Objective::Reach { unseen: true, .. } | Objective::EliminateUnseen { .. }
            );
            for detected in [false, true] {
                if detected && !unseen {
                    continue;
                }
                let mut playback = start(sequence);
                let progress = serde_json::to_value(&playback.run.progress).unwrap();
                let outcome = if detected {
                    let Content::Gameplay(session) = &mut playback.content else {
                        unreachable!()
                    };
                    session.world.ai.detections = 1;
                    detections += 1;
                    Outcome::Detected
                } else {
                    super::death::start_death(&mut playback);
                    deaths += 1;
                    Outcome::Dead
                };
                finish_sequence(&mut playback, Some(outcome));
                let mut expected = descriptor.failure;
                let mut failure_scenes = 0;
                while let Some(index) = expected {
                    assert_eq!(playback.run.sequence, index);
                    assert!(matches!(
                        playback.content,
                        Content::Story { .. } | Content::Movie(_)
                    ));
                    finish_sequence(&mut playback, None);
                    expected = campaign.sequences[index].next;
                    failure_scenes += 1;
                    assert!(failure_scenes < 10);
                }
                assert!(
                    matches!(playback.content, Content::Failed),
                    "sequence {sequence}, {outcome:?}"
                );
                assert_eq!(
                    serde_json::to_value(&playback.run.progress).unwrap(),
                    progress
                );
                playback.action = Some(Action::Retry);
                playback.step(0).unwrap();
                assert_eq!(
                    playback.run.sequence,
                    campaign.first_gameplay(hero, mission).unwrap()
                );
                assert_eq!((playback.run.hero, playback.run.mission), (hero, mission));
                assert_eq!(
                    serde_json::to_value(&playback.run.progress).unwrap(),
                    progress
                );
                let Content::Gameplay(session) = &playback.content else {
                    panic!("retry did not load gameplay")
                };
                let actor = &session.world.actors[session.world.player].actor;
                assert_eq!(actor.health, actor.max_health);
                assert_eq!(actor.inventory.counts[4], 5);
                assert_eq!(session.world.statistics.killed, 0);
                assert_eq!(session.world.ai.detections, 0);
                assert!(session.world.mission.input_allowed());
                eprintln!(
                    "hero {hero}, mission {mission}: {sequence} {outcome:?}, {failure_scenes} failure scenes, retry {}",
                    playback.run.sequence
                );
            }
        }
    }
    assert_eq!((deaths, detections), (56, 2));
}
