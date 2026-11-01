use super::*;
use crate::visuals::appearance::library::{Library, import_actor};
use tenchu_core::{
    ai::programs::{Idle, Programs},
    collision,
    math::Position,
};

#[test]
#[ignore = "requires TENCHU_DISC; verifies all disc actor imports in a live mission, collision, protection and subsequent simulation"]
fn manually_placed_cross_stage_actors_play_without_taking_story_roles() {
    let mut app = application(Options::default());
    let library = Library::load(&app.assets).unwrap();
    let game = &mut app.campaign.game;
    game.events = Default::default();
    game.roster.get_mut(game.player_id).invulnerable = true;
    for _ in 0..40 {
        game.step(0).unwrap();
    }
    let mut imported = std::collections::BTreeMap::new();
    let baseline = game.roster.order.len();
    let player = game.player().position;
    assert!(game.remove_actor(game.player_id).is_err());
    let protected = game
        .mission
        .story
        .iter()
        .find_map(|a| game.roster.find(a.kind))
        .unwrap();
    assert!(game.move_actor(protected, player, 0, true).is_err());
    assert!(game.remove_actor(protected).is_err());
    let mut shadowed = 0;
    for body in &library.bodies {
        let source = import_actor(&mut game.mission, &library, &body.id, &mut imported).unwrap();
        let definitions = game.mission.characters.len();
        let again = import_actor(&mut game.mission, &library, &body.id, &mut imported).unwrap();
        assert_eq!(source.actor.model, again.actor.model);
        assert_eq!(definitions, game.mission.characters.len());
        let think = Programs::decode(source.think).unwrap();
        assert!(
            game.place_actor(source.actor, think, Position::new(320001, 0, 0), 0, true)
                .is_err()
        );
        assert_eq!(game.roster.order.len(), baseline);
        let kind = game.mission.characters[source.actor.definition].kind;
        let story = game.roster.find(kind);
        let id = game
            .place_actor(source.actor, think, player, 1024, true)
            .unwrap_or_else(|e| panic!("{}: {e}", body.name));
        game.roster.get_mut(id).voice_program = source.voice;
        let actor = game.roster.get(id);
        assert_eq!(
            game.roster.find(kind),
            story,
            "{} must not become a story lookup target",
            body.name
        );
        assert_eq!(actor.position.y, actor.map.level);
        assert_eq!(actor.previous, actor.position);
        assert_eq!(actor.target, Some(game.player().root_model));
        assert!(
            game.collision.slots[..game.collision.len]
                .iter()
                .any(|slot| slot.actor() == Some(id) && slot.class & collision::HIT == 0)
        );
        let target = Position::new(player.x + 2000, player.y, player.z);
        game.move_actor(id, target, 2048, true).unwrap();
        assert_eq!(game.roster.get(id).position.x, target.x);
        let body_slot = game.collision.slots[..game.collision.len]
            .iter()
            .find(|s| s.actor() == Some(id) && s.class & collision::HIT == 0)
            .unwrap();
        assert!(
            (body_slot.position.x - target.x).abs() < 1000,
            "{} collider must move immediately: {:?} / {:?}",
            body.name,
            body_slot.position,
            target
        );
        assert_eq!(game.roster.get(id).home, [target.x, target.z]);
        assert_ne!(game.roster.get(id).think.idle, Idle::Trace);
        assert!(game.roster.get(id).patrol.is_none());
        for _ in 0..40 {
            game.step(0)
                .unwrap_or_else(|e| panic!("{}: {e}", body.name));
        }
        let root = game.roster.get(id).root_model;
        shadowed += usize::from(
            game.effects
                .shadows
                .iter()
                .any(|shadow| shadow.owner == root),
        );
        game.remove_actor(id).unwrap();
        assert!(
            !game
                .effects
                .shadows
                .iter()
                .any(|shadow| shadow.owner == root)
        );
        assert_eq!(game.roster.order.len(), baseline);
        assert!(!game.visible.contains(&id));
        assert!(
            !game.collision.slots[..game.collision.len]
                .iter()
                .any(|slot| slot.actor() == Some(id))
        );
        game.step(0).unwrap();
    }
    assert!(
        shadowed > 0,
        "Exercise removal of a visible round shadow while paused"
    );
    crate::audio::game_bank(&app.assets, game).unwrap();
    game.cinematic.active = true;
    let source = imported.values().next().unwrap();
    assert!(
        game.place_actor(
            source.actor,
            Programs::decode(source.think).unwrap(),
            player,
            0,
            true
        )
        .is_err()
    );
    game.cinematic.active = false;
    assert!(game.remove_actor(usize::MAX).is_err());
}

#[test]
#[ignore = "requires TENCHU_DISC; manual edits must end a replayable recording and be discarded on retry"]
fn actor_edit_tools_end_recording_and_retry_restores_layout() {
    let mut app = application(Options::default());
    let directory = std::env::temp_dir().join(format!(
        "tenchu-actor-edit-recording-{}",
        std::process::id()
    ));
    app.recording_directory = Some(directory.clone());
    app.campaign.screen = Screen::Launcher;
    app.action(Action::Confirm);
    app.action(Action::Confirm);
    assert!(app.recording.is_some());
    app.record_input(0);
    let id = app
        .campaign
        .game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| {
            let actor = app.campaign.game.roster.get(id);
            id != app.campaign.game.player_id && actor.patrol.is_some() && actor.kind < 0x80
        })
        .unwrap();
    app.debug.selected_actor = id;
    app.edit_actor(Item::ActorPosition).unwrap();
    assert!(app.recording.is_some());
    let original = app.debug.target;
    app.debug.target.x += 500;
    app.trial.eligible = true;
    app.edit_actor(Item::MoveActor).unwrap();
    assert!(app.recording.is_none());
    assert!(app.debug.actors_edited);
    assert!(!app.trial.eligible);
    assert_eq!(
        app.campaign.game.roster.get(id).position.x,
        original.x + 500
    );
    app.edit_actor(Item::RemoveActor).unwrap();
    assert!(app.campaign.game.roster.actors[id].is_none());
    app.action(Action::Replay);
    app.action(Action::Confirm);
    assert!(!app.debug.actors_edited);
    assert_eq!(app.campaign.game.roster.get(id).position.x, original.x);
    app.finish_recording();
    std::fs::remove_dir_all(directory).unwrap();
}
