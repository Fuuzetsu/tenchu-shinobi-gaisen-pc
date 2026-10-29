use super::*;
use std::collections::BTreeSet;

fn catalog() -> Catalog {
    Catalog {
        actors: [
            ("disc:0", Group::Players),
            ("disc:1", Group::Players),
            ("disc:16", Group::Guards),
            ("disc:32", Group::Guards),
            ("disc:135", Group::Bosses),
            ("disc:160", Group::Animals),
        ]
        .into_iter()
        .map(|(id, group)| Entry {
            id: id.into(),
            name: id.into(),
            group,
        })
        .collect(),
        stages: (0..=10).map(|i| (i, format!("Stage {i}"))).collect(),
    }
}

#[test]
fn version_one_random_stream_is_stable() {
    let mut random = random::Random::new("morning-ninja", 0, "enemy", 7);
    assert_eq!(
        std::array::from_fn::<_, 4, _>(|_| random.next()),
        [
            0xe665739efd553689,
            0xd5e111f1d3437f17,
            0xf18f0248ed50a204,
            0x017a1e0cf90908a9,
        ]
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; verifies retry state, story identity, texture isolation and campaign save separation"]
fn retries_preserve_the_roll_without_touching_normal_progress() {
    use crate::{
        campaign::{Campaign, Screen},
        session::Options,
    };
    use tenchu_core::game::events::Outcome;
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let library = Library::load(&assets).unwrap();
    let mut campaign = Campaign::new(&assets, Options::default(), None).unwrap();
    campaign.progress.highest = [4, 7];
    campaign.progress.stock[1][13] = 11;
    campaign.seed = 12345;
    campaign.checkpoint(campaign.options);
    let normal_save = campaign.save_request.take().unwrap();
    campaign.saved = Some(normal_save.clone());
    let mut run = Run::new(Document::default());
    run.document.rules.player = Player::Models;
    run.document.rules.player_pool.groups = [Group::Bosses].into();
    run.document.rules.health = Some(Range::new(123, 123));
    campaign
        .start_randomizer(run.clone(), &assets, &library)
        .unwrap();
    assert_eq!(campaign.game.player().health, 123);
    assert_eq!(campaign.game.player().max_health, 123);
    let plan = &campaign.randomized.as_ref().unwrap().plan;
    let original = plan
        .options(run.document.rules.language.game())
        .load_equipped(&assets, plan.seed, plan.inventory)
        .unwrap();
    assert_eq!(
        campaign.game.mission.vram.words,
        original.mission.vram.words
    );
    assert!(!campaign.game.mission.imported_models.is_empty());
    for actor in &original.mission.story {
        let id = original.roster.find(actor.kind).unwrap();
        assert_eq!(campaign.game.roster.find(actor.kind), Some(id));
        assert_eq!(
            campaign.game.roster.get(id).kind,
            original.roster.get(id).kind
        );
        assert_eq!(
            campaign.game.roster.get(id).health,
            original.roster.get(id).health
        );
    }
    let initial = crate::session::tests::state(&campaign.game);
    for _ in 0..180 {
        campaign.tick(tenchu_core::input::UP).unwrap();
    }
    let advanced = crate::session::tests::state(&campaign.game);
    campaign
        .start_randomizer(run.clone(), &assets, &library)
        .unwrap();
    assert_eq!(crate::session::tests::state(&campaign.game), initial);
    for _ in 0..180 {
        campaign.tick(tenchu_core::input::UP).unwrap();
    }
    assert_eq!(crate::session::tests::state(&campaign.game), advanced);
    campaign.game.events.outcome = Outcome::Complete;
    campaign.tick(0).unwrap();
    assert!(matches!(campaign.screen, Screen::Results(_)));
    assert!(campaign.save_request.is_none());
    assert_eq!(campaign.progress, normal_save.progress);
    assert_eq!(campaign.saved.as_ref().unwrap(), &normal_save);
    let mut invalid = run.clone();
    invalid.document.rules.enemy_pool.groups.clear();
    assert!(
        campaign
            .start_randomizer(invalid, &assets, &library)
            .is_err()
    );
    assert_eq!(campaign.randomized.as_ref().unwrap().run, run);
    campaign
        .start_randomizer(run.next().unwrap(), &assets, &library)
        .unwrap();
    campaign.end_randomizer(&assets).unwrap();
    campaign.checkpoint(campaign.options);
    assert_eq!(campaign.save_request.unwrap(), normal_save);
}

#[test]
fn shared_runs_round_trip_and_independent_categories_keep_their_rolls() {
    let mut run = Run::new(Document::default());
    run.round = 47;
    run.document.seed = "朝の忍者 / a shared seed".into();
    run.document.rules.stages = Stages::Random;
    run.document.rules.player = Player::Models;
    let imported = Run::import(&run.export()).unwrap();
    assert_eq!(imported, run);
    let first = run.plan(&catalog()).unwrap();
    run.document.rules.health = Some(Range::new(1, 1));
    let second = run.plan(&catalog()).unwrap();
    assert_eq!(second.health, Some(1));
    assert_eq!(first.mission, second.mission);
    assert_eq!(first.player_model, second.player_model);
    assert_eq!(first.inventory, second.inventory);
    assert_eq!(first.seed, second.seed);
    assert_ne!(run.document.fingerprint(), imported.document.fingerprint());
    assert_eq!(imported.plan(&catalog()).unwrap(), first);
}

#[test]
fn exclusions_and_tours_hold_over_many_seeds_and_singleton_pools() {
    let catalog = catalog();
    let mut run = Run::new(Document::default());
    let r = &mut run.document.rules;
    r.stages = Stages::Tour;
    r.blocked_stages = [0, 2, 4, 6, 8, 10].into();
    r.player = Player::Models;
    r.player_pool.blocked = catalog
        .actors
        .iter()
        .filter(|a| a.id != "disc:160")
        .map(|a| a.id.clone())
        .collect();
    r.blocked_items = (2..20).collect();
    r.item_types = Range::new(1, 1);
    for seed in 0..50 {
        run.document.seed = seed.to_string();
        for cycle in 0..3 {
            let mut visited = BTreeSet::new();
            for round in 0..5 {
                run.round = cycle * 5 + round;
                let plan = run.plan(&catalog).unwrap();
                visited.insert(plan.mission);
                assert_eq!(plan.player_model.as_deref(), Some("disc:160"));
                assert_eq!(plan.inventory[0], 255);
                assert!((2..=8).contains(&plan.inventory[1]));
                assert!(plan.inventory[2..].iter().all(|&n| n == 0));
            }
            assert_eq!(visited, [1, 3, 5, 7, 9].into());
        }
    }
    run.document.rules.blocked_items = (1..20).collect();
    run.document.rules.keep_grapple = false;
    run.document.rules.pickups = false;
    let plan = run.plan(&catalog).unwrap();
    assert_eq!(plan.inventory[0], 255);
    assert!(plan.inventory[1..].iter().all(|&n| n == 0));
}

#[test]
fn invalid_or_incompatible_documents_fail_before_generating() {
    let catalog = catalog();
    let base = Document::default();
    let mut invalid = base.clone();
    invalid.rules.enemy_pool.groups.clear();
    assert!(invalid.validate(&catalog).is_err());
    invalid = base.clone();
    invalid.rules.health = Some(Range::new(0, 1));
    assert!(invalid.validate(&catalog).is_err());
    invalid = base.clone();
    invalid.rules.blocked_stages.insert(1);
    assert!(invalid.validate(&catalog).is_err());
    invalid = base.clone();
    invalid.rules.blocked_items.insert(0);
    assert!(invalid.validate(&catalog).is_err());
    invalid = base.clone();
    invalid
        .rules
        .player_pool
        .blocked
        .insert("foreign:model".into());
    assert!(invalid.validate(&catalog).is_err());
    assert!(Document::import(&base.export().replace("\"version\": 1", "\"version\": 2")).is_err());
    assert!(Run::import(&"x".repeat(65537)).is_err());
    assert!(Run::import("{}").is_err());
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises every disc model as a foreign enemy and player with deterministic starts"]
fn every_model_can_be_randomized_across_missions() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = tenchu_assets::game::GameAssets::open(
        crate::source::open_disc(std::path::Path::new(&disc)).unwrap(),
    )
    .unwrap();
    let library = crate::visuals::appearance::library::Library::load(&assets).unwrap();
    let catalog = Catalog::new(&library, &assets.stages().unwrap());
    for body in &library.bodies {
        println!(
            "{}: {} · definition {:?} · think {:04x} · voice {:?}",
            body.id,
            body.name,
            body.character.as_ref().map(|c| (c.kind, c.health)),
            body.think,
            body.voice.map(|v| (v.stage, v.program))
        );
    }
    for (i, body) in library.bodies.iter().enumerate() {
        let mut run = Run::new(Document::default());
        let r = &mut run.document.rules;
        r.mission = i % 10 + 1;
        r.player = Player::Models;
        r.player_pool.blocked = catalog
            .actors
            .iter()
            .filter(|a| a.id != body.id)
            .map(|a| a.id.clone())
            .collect();
        r.enemy_pool = r.player_pool.clone();
        r.enemies = Enemies::Uniform;
        r.keep_active = true;
        let mission = r.mission;
        let (mut game, _) = run
            .load(&assets, &library)
            .unwrap_or_else(|e| panic!("{}: {e}", body.name));
        game.events = Default::default();
        game.roster.get_mut(game.player_id).invulnerable = true;
        for tick in 0..600 {
            let input = if tick % 150 < 100 {
                tenchu_core::input::UP
            } else {
                tenchu_core::input::RIGHT | tenchu_core::input::SQUARE
            };
            game.step(input)
                .unwrap_or_else(|e| panic!("{} in mission {mission} tick {tick}: {e}", body.name));
            if game.events.outcome != tenchu_core::game::events::Outcome::Running {
                break;
            }
        }
    }
}
