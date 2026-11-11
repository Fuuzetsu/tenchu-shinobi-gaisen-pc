use super::{
    campaign::{Content, Playback},
    preferences::Menu,
    transport::Transport,
};
use crate::{controls::Controls, diagnostics::Recent, settings::Settings};
use serde::Serialize;
use serde_json::{Value, json};
use tenchu_assets::ReadAt;
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{actor::State, ai::status::Ki, world::World},
};

#[derive(Serialize)]
pub struct Input {
    tick_before: i64,
    random: Random,
    buttons: u16,
    navigation: Option<super::navigation::Input>,
    player: Option<Player>,
    observing_death: bool,
}

#[derive(Serialize)]
struct Player {
    position: Position,
    health: i16,
    state: State,
    radar: Ki,
    detections: u8,
}

pub fn record(recent: &mut Recent<Input>, playback: &Playback<impl ReadAt>, buttons: u16) {
    let (random, player) = match &playback.content {
        Content::Gameplay(session) => {
            let w = &session.world;
            let a = &w.actors[w.player].actor;
            (
                w.random,
                Some(Player {
                    position: a.position,
                    health: a.health,
                    state: a.state,
                    radar: w.ai.global.radar,
                    detections: w.ai.detections,
                }),
            )
        }
        Content::Story { story, .. } => (story.state.random, None),
        _ => return,
    };
    let tick_before = playback.content.tick();
    recent.push(
        playback.generation,
        tick_before.max(0) as u64,
        Input {
            tick_before,
            random,
            buttons,
            navigation: match &playback.content {
                Content::Gameplay(session) => session.navigation,
                _ => None,
            },
            player,
            observing_death: playback.death.waiting(),
        },
    );
}

fn world(w: &World) -> Value {
    let actors: Vec<_> = w.actors.iter().enumerate().map(|(id, agent)| {
        let a = &agent.actor;
        let rig = &w.instances[id].rig;
        json!({
            "id": id, "kind": a.kind, "definition": a.definition_kind(),
            "position": a.position, "previous": a.previous, "rotation": a.rotation,
            "velocity": a.velocity, "health": a.health, "max_health": a.max_health,
            "invulnerable": a.invulnerable, "state": a.state, "flags": a.flags,
            "breath": a.breath, "dimensions": [a.width, a.height], "turn": a.turn, "skill": a.skill,
            "terrain": a.terrain, "hit": a.hit, "target": a.target, "pad": a.pad,
            "motion": {"action": a.motion.action, "clip": a.motion.clip, "frame": a.motion.frame,
                "loops": a.motion.loops, "mask": a.motion.mask, "changes": a.motion_changes},
            "attack": a.attack, "inventory": a.inventory, "behavior": agent.behavior,
            "sight": agent.sight, "surroundings": agent.surroundings,
            "slot": agent.slot, "sight_range": agent.sight_range,
            "encounters": a.encounters, "damage_bonus_percent": a.encounters.bonus(w.ai.damage_escalation),
            "root_model": rig.root, "voice": w.instances[id].voice,
        })
    }).collect();
    let items: Vec<_> = w
        .items
        .entries
        .iter()
        .map(|e| {
            json!({
                "slot": e.slot, "state": e.state, "model": e.model,
                "transform": w.scene.models.get(e.model).map(|m| m.local),
            })
        })
        .collect();
    json!({
        "tick": w.tick, "random": w.random, "area": w.map, "sequence": w.sequence,
        "player_id": w.player, "difficulty": w.difficulty, "script": w.script,
        "hidden_scene": w.hidden_scene, "tracking": w.tracking, "view_distance": w.view_distance,
        "activation": {"keep_all_active": w.keep_all_active,
            "active": w.actors.iter().filter(|a| a.actor.flags & 8 == 0).count(),
            "total": w.actors.len()},
        "travel": w.travel, "effort": w.effort, "water": w.water,
        "terrain_options": w.terrain_options, "probe": w.probe, "controls": w.controls,
        "mission": w.mission, "statistics": w.statistics, "ai": w.ai,
        "camera": w.camera, "actors": actors, "items": items,
        "item_aim": w.items.aim, "item_held": w.items.held,
        "effect_count": w.effects.entries.len(), "scene_models": w.scene.models.len(),
        "colliders": w.collision.slots(), "triggers": w.environment.triggers,
    })
}

pub fn report(
    playback: &Playback<impl ReadAt>,
    transport: &Transport,
    menu: &Menu,
    settings: &Settings,
    controls: &Controls,
) -> Value {
    let mut report = crate::diagnostics::report(settings, &menu.tools.debug, controls);
    report["engine"] = json!("tenchu2-jp");
    report["disc"] = json!({"edition": "SLPS-03040", "source_validation": "JP executable and module hashes verified by the asset loader"});
    report["input_encoding"] =
        json!("Tenchu 2 native pad mask; byte-swapped relative to Gaisen PS1 input masks");
    report["session"] = json!({"sequence": playback.run.sequence, "hero": playback.run.hero,
        "mission": playback.run.mission, "difficulty": playback.run.difficulty,
        "generation": playback.generation, "played_seconds": playback.run.played_seconds,
        "random": playback.run.random, "summary": playback.run.summary,
        "carry": playback.run.carry, "results": playback.run.results});
    let screen = match &playback.content {
        Content::MissionSelection(_) => "mission-selection",
        Content::Prologue(_) => "prologue",
        Content::Equipment(_) => "equipment",
        Content::Gameplay(session) => {
            report["game"] = world(&session.world);
            report["pending_integration"] = json!(session.pending);
            report["model_override"] = session.models.diagnostic_snapshot();
            if let Some(scene) = &session.gaisen {
                let source = &scene.director.source;
                report["gaisen"] = json!({"stage": source.mission.stage.id,
                    "character": source.player().kind, "tick": source.tick,
                    "presenting": scene.director.presenting,
                    "sequence": source.cinematic.sequence, "elapsed": source.cinematic.elapsed,
                    "tail": source.cinematic.tail, "view": source.rendered_view,
                    "events": source.events, "random": source.random,
                    "script_hidden": session.world.script_hidden});
            }
            if playback.death.waiting() {
                "death-hold"
            } else if session.cinematic() {
                "gaisen-cinematic"
            } else {
                "playing"
            }
        }
        Content::Story { story, .. } => {
            report["story"] = json!({"event": story.event, "area": story.level.id,
                "frame": story.state.frame, "random": story.state.random, "camera": story.state.camera,
                "view": story.state.view, "actors": story.state.actors, "ending": story.state.ending});
            "story"
        }
        Content::Introduction(_) => "introduction",
        Content::Movie(_) => "movie",
        Content::Results(_) => "results",
        Content::Failed => "failed",
        Content::Complete => "complete",
    };
    report["runtime"] = json!({"screen": screen, "tick": playback.content.tick(),
        "menu_open": menu.panel.open, "integration_diagnostics_open": menu.diagnostics,
        "fast_forward": menu.fast_forward, "notice": menu.panel.notice,
        "death_hold": playback.death.waiting(), "live_death_scene": playback.death.live()});
    report["recent_inputs"] = json!(transport.recent);
    report
}

pub fn filename(playback: &Playback<impl ReadAt>) -> String {
    format!(
        "tenchu-report-{}-t2jp-s{}-tick{}.json",
        env!("CARGO_PKG_VERSION"),
        playback.run.sequence,
        playback.content.tick()
    )
}

#[cfg(test)]
mod tests;
