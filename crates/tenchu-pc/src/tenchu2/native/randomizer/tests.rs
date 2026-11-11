use super::*;
use crate::randomizer::{Document, Player, Range};
use tenchu_core::tenchu2::mission::{Outcome, Phase};

fn bytes(value: &impl Serialize) -> Vec<u8> {
    postcard::to_allocvec(value).unwrap()
}

fn gameplay(playback: &mut Playback<crate::source::DiscSource>) -> &mut Session {
    let Content::Gameplay(session) = &mut playback.content else {
        panic!("expected gameplay at sequence {}", playback.run.sequence)
    };
    session
}

fn disc() -> Disc<crate::source::DiscSource> {
    crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap()
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; seeded native play, retries, checkpoints, areas and results stay isolated from normal progress"]
fn native_randomizer_retry_carry_and_save_isolation() {
    let root = std::env::temp_dir().join(format!("tenchu-native-seed-{}", std::process::id()));
    let file = root.join("progress.json");
    let save = super::super::save::SaveFile::at(file.clone());
    let mut progress = tenchu_core::tenchu2::progress::Progress::default();
    progress.stock[0].items[7] = Some(13);
    save.write(&progress).unwrap();
    let original_save = std::fs::read(&file).unwrap();
    let mut playback = Playback::load(disc(), 9, 2, 17, Some([0; 21]), Some(save)).unwrap();
    let original = playback.capture_state().unwrap();
    playback.prepare_randomizer(None).unwrap();
    let resources = playback.randomizer.resources.as_ref().unwrap();
    assert!(resources.donor.is_none());
    assert!(resources.catalog.actors.len() > 30);
    assert!(resources.catalog.stages.len() > 20);
    let mut run = Run::new(Document::for_source(Source::Tenchu2));
    let rules = &mut run.document.rules;
    rules.player = Player::Chosen;
    rules.character = 2;
    rules.health = Some(Range::new(999, 999));
    rules.keep_grapple = false;
    rules.blocked_items.insert(0);
    rules.equipment = false;
    rules.keep_active = true;
    playback.start_randomizer(run.clone()).unwrap();
    assert_eq!(
        playback.run.hero, 0,
        "mission retains Rikimaru story identity"
    );
    playback.configure(&Settings::default()).unwrap();
    let session = gameplay(&mut playback);
    assert_eq!(session.world.actors[session.world.player].actor.health, 999);
    assert_eq!(
        session.world.actors[session.world.player]
            .actor
            .inventory
            .counts[0],
        0
    );
    assert_eq!(session.assets.models["override-player"].parts.len(), 15);
    assert!(session.world.keep_all_active);
    assert!(
        session
            .world
            .actors
            .iter()
            .any(|a| a.actor.mission_kind.is_some())
    );
    let initial = playback.capture_state().unwrap();
    let mut invalid = run.clone();
    invalid.document.rules.enemy_pool.groups.clear();
    assert!(playback.start_randomizer(invalid).is_err());
    assert!(
        playback.capture_state().unwrap() == initial,
        "failed seed changed the active attempt"
    );
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    let advanced = playback.capture_state().unwrap();
    playback.start_randomizer(run.clone()).unwrap();
    playback.configure(&Settings::default()).unwrap();
    assert!(
        playback.capture_state().unwrap() == initial,
        "retry changed the roll"
    );
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    assert!(
        playback.capture_state().unwrap() == advanced,
        "retry changed source continuation"
    );
    playback.restore_state(&initial).unwrap();
    for _ in 0..90 {
        playback.step(0).unwrap();
    }
    assert!(
        playback.capture_state().unwrap() == advanced,
        "checkpoint changed continuation"
    );
    let session = gameplay(&mut playback);
    let player = &mut session.world.actors[session.world.player].actor;
    player.health = 512;
    player.inventory.counts[4] = 1;
    session.world.mission.phase = Phase::Finished {
        outcome: Outcome::Arrived,
        finished_at: 300,
    };
    playback.step(0).unwrap();
    assert_eq!(playback.run.sequence, 10);
    for tick in 0..6000 {
        playback.step(if tick == 30 { 0x4000 } else { 0 }).unwrap();
        if matches!(playback.content, Content::Gameplay(_)) {
            break;
        }
    }
    assert_eq!(playback.run.sequence, 11);
    let session = gameplay(&mut playback);
    let player = &session.world.actors[session.world.player].actor;
    assert_eq!((player.health, player.max_health), (512, 999));
    assert_eq!(player.inventory.counts[0], 0);
    assert_eq!(
        player.inventory.counts[4], 1,
        "area transition refilled spent supplies"
    );
    for _ in 0..60 {
        playback.step(0).unwrap();
    }
    assert!(gameplay(&mut playback).world.keep_all_active);
    let results = playback
        .campaign
        .sequences
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            (s.kind == tenchu_assets::tenchu2::campaign::Kind::Results
                && playback.campaign.mission(0, i) == Some(1))
            .then_some(i)
        })
        .unwrap();
    playback.run.sequence = results;
    playback.load_current().unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), original_save);
    let Content::Results(results) = &mut playback.content else {
        unreachable!()
    };
    results.exit = Some((super::super::campaign::Action::Continue, 1));
    playback.step(0).unwrap();
    assert_eq!(playback.randomized().unwrap().0.round, 1);
    playback.end_randomizer();
    playback.configure(&Settings::default()).unwrap();
    assert!(
        playback.capture_state().unwrap() == original,
        "normal session was changed"
    );
    assert_eq!(std::fs::read(&file).unwrap(), original_save);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires both discs; native model conversion has identical contacts, animation and source continuation with and without a T1 donor"]
fn native_randomizer_models_do_not_require_gaisen() {
    let disc = disc();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut native = Library::default();
    native.load_tenchu2(&disc).unwrap();
    let mut combined = Library::load(&gaisen).unwrap();
    combined.load_tenchu2(&disc).unwrap();
    let mut donor = models::donor(&gaisen).unwrap();
    let mut count = 0;
    for body in native
        .bodies
        .iter()
        .filter(|b| b.parts.len() == 15 && models::compatible_moveset(b))
    {
        let id = &body.id;
        count += 1;
        combined
            .get("disc:135")
            .unwrap()
            .apply_moveset(&mut donor, Default::default())
            .unwrap();
        let settings = Settings {
            player_model: id.clone(),
            model_moveset: true,
            ..Default::default()
        };
        let mut a = Session::load(&disc, 9, 1).unwrap();
        let mut b = Session::load(&disc, 9, 1).unwrap();
        a.models
            .sync_optional(
                &mut a.world,
                &mut a.assets,
                None,
                &native,
                &Arc::default(),
                &settings,
            )
            .unwrap();
        b.models
            .sync(
                &mut b.world,
                &mut b.assets,
                &mut donor,
                &combined,
                &Arc::default(),
                &settings,
            )
            .unwrap();
        assert!(bytes(&a.world) == bytes(&b.world), "{id}: initial world");
        assert!(
            bytes(&a.assets) == bytes(&b.assets),
            "{id}: converted assets"
        );
        for tick in 0..120 {
            let buttons = if tick < 45 {
                tenchu_core::tenchu2::input::UP
            } else if tick % 20 == 0 {
                tenchu_core::tenchu2::input::ATTACK
            } else {
                0
            };
            a.step(buttons).unwrap();
            b.step(buttons).unwrap();
            assert!(bytes(&a.world) == bytes(&b.world), "{id}: update {tick}");
        }
    }
    assert!(count > 20);
    eprintln!("Matched native-only/combined conversion and 120 updates for {count} movesets");
}

#[test]
#[ignore = "requires both discs; seeded starts across the native/imported catalog and cross-stage checkpoints retain mission roles"]
fn native_randomizer_catalog_starts_and_cross_stage_states() {
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let mut playback = Playback::load(disc(), 9, 2, 17, Some([0; 21]), None).unwrap();
    playback.prepare_randomizer(Some(path.as_ref())).unwrap();
    let missions: Vec<_> = playback
        .randomizer_catalog()
        .unwrap()
        .stages
        .iter()
        .map(|s| s.0)
        .collect();
    let mut saved = Vec::new();
    for &mission in &missions {
        let mut run = Run::new(Document::for_source(Source::Tenchu2));
        run.document.rules.mission = mission;
        run.document.rules.blocked_stages.clear();
        run.document.rules.protect_story = false;
        run.document.rules.replace_civilians = true;
        run.document.rules.player = Player::Models;
        run.document.rules.health = Some(Range::new(300, 500));
        run.document.rules.language = crate::randomizer::Language::English;
        playback
            .start_randomizer(run)
            .unwrap_or_else(|e| panic!("mission {mission}: {e}"));
        let native = mission < 100;
        assert_eq!(playback.gaisen.is_none(), native);
        let session = gameplay(&mut playback);
        assert!(session.world.actors[session.world.player].actor.health >= 300);
        let mission_state = bytes(&session.world.mission);
        let random = session.world.random;
        let roles: Vec<_> = session
            .world
            .actors
            .iter()
            .map(|a| (a.slot, a.actor.mission_kind()))
            .collect();
        let checkpoint = playback.capture_state().unwrap();
        for _ in 0..30 {
            playback
                .step(0)
                .unwrap_or_else(|e| panic!("mission {mission}: {e}"));
        }
        let continued = playback.capture_state().unwrap();
        playback
            .restore_state(&checkpoint)
            .unwrap_or_else(|e| panic!("mission {mission} checkpoint: {e}"));
        let session = gameplay(&mut playback);
        assert_eq!(session.world.random, random);
        assert!(bytes(&session.world.mission) == mission_state);
        assert_eq!(
            session
                .world
                .actors
                .iter()
                .map(|a| (a.slot, a.actor.mission_kind()))
                .collect::<Vec<_>>(),
            roles
        );
        for _ in 0..30 {
            playback.step(0).unwrap();
        }
        assert!(
            playback.capture_state().unwrap() == continued,
            "mission {mission} continuation"
        );
        if matches!(mission, 1 | 12 | 101 | 104) {
            saved.push((mission, checkpoint));
        }
    }
    assert!(saved.len() >= 3);
    for (mission, checkpoint) in saved {
        playback
            .restore_state(&checkpoint)
            .unwrap_or_else(|e| panic!("cross-stage {mission}: {e}"));
        assert_eq!(playback.randomized().unwrap().1.mission, mission);
        assert!(
            playback.capture_state().unwrap() == checkpoint,
            "cross-stage {mission}"
        );
        for _ in 0..30 {
            playback.step(0).unwrap();
        }
    }
    eprintln!(
        "Verified {} native/imported seeded mission starts and continuations",
        missions.len()
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; real native item throwing consumes finite supplies but retains seeded unlimited supplies"]
fn native_randomizer_unlimited_items_follow_native_throws() {
    let mut playback = Playback::load(disc(), 9, 2, 17, Some([0; 21]), None).unwrap();
    playback.prepare_randomizer(None).unwrap();
    for unlimited in [false, true] {
        let mut run = Run::new(Document::for_source(Source::Tenchu2));
        let rules = &mut run.document.rules;
        rules.player = Player::Chosen;
        rules.character = 0;
        rules.enemies = Enemies::Original;
        rules.infinite_items = unlimited;
        rules.item_types = Range::new(1, 1);
        rules.item_count = Range::new(3, 3);
        rules.blocked_items = Source::Tenchu2
            .items()
            .map(|(i, _)| i)
            .filter(|&i| i != 0 && i != 4)
            .collect();
        playback.start_randomizer(run).unwrap();
        let session = gameplay(&mut playback);
        let player = &mut session.world.actors[session.world.player].actor;
        player.inventory.selected =
            Some(tenchu_core::tenchu2::inventory::Item::try_from(4).unwrap());
        player.invulnerable = true;
        let mut threw = false;
        for tick in 0..180 {
            playback
                .step(if tick == 40 {
                    tenchu_core::tenchu2::input::ITEM
                } else {
                    0
                })
                .unwrap();
            let session = gameplay(&mut playback);
            threw |= session
                .world
                .items
                .entries
                .iter()
                .any(|e| matches!(e.state, tenchu_core::tenchu2::items::State::Projectile(_)));
        }
        assert!(threw, "fixture did not throw a shuriken");
        let session = gameplay(&mut playback);
        assert_eq!(
            session.world.actors[session.world.player]
                .actor
                .inventory
                .counts[4],
            if unlimited { 255 } else { 2 }
        );
    }
}
