use serde_json::{Value, json};
use tenchu_assets::Result;
use tenchu_core::{game::Game, math::Position};

pub const FORMAT: &str = "tenchu-diagnostic-report";
pub const VERSION: u32 = 1;

pub fn build() -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "revision": env!("TENCHU_BUILD_REVISION"),
        "target": env!("TENCHU_BUILD_TARGET"),
        "profile": env!("TENCHU_BUILD_PROFILE"),
        "compiler": env!("TENCHU_BUILD_COMPILER"),
        "debug_assertions": cfg!(debug_assertions),
    })
}

fn position(p: Position) -> Value {
    json!({"x": p.x, "y": p.y, "z": p.z})
}

pub fn game(game: &Game) -> Value {
    let actors: Vec<_> = game.roster.actors.iter().flatten().map(|a| json!({
        "id": a.id, "kind": a.kind, "script_kind": a.script_kind,
        "mesh_kind": a.mesh_kind, "definition": a.definition,
        "position": position(a.position), "previous": position(a.previous),
        "velocity": a.velocity, "rotation": a.rotation,
        "health": a.health, "max_health": a.max_health, "invulnerable": a.invulnerable,
        "status": a.status, "attribute": a.attribute, "scripted": a.scripted,
        "target_model": a.target, "hit_actor": a.hit, "home": a.home,
        "think": a.think,
        "brain": {"mode": a.brain.mode, "flag": a.brain.flag, "count": a.brain.count,
            "timer": a.brain.timer, "chase": a.brain.chase},
        "motion": {"action": a.motion.action, "clip": a.motion.clip, "count": a.motion.count,
            "loops": a.motion.loops, "mode": a.motion.mode, "mask": a.motion.mask},
        "terrain": {"area": a.map.area, "row": a.map.row, "level": a.map.level,
            "height": a.map.height, "attribute": a.map.attribute, "vector": a.map.vector},
        "pad": {"held": a.pad.held, "previous": a.pad.previous, "trigger": a.pad.trigger,
            "stream": a.pad.stream, "time": a.pad.time, "hold_counter": a.pad.hold_counter},
        "inventory": a.inventory.iter().copied().chain([u8::from(a.aiming)]).collect::<Vec<_>>(), "active_item": a.active_item, "attack": a.attack,
        "root_model": a.root_model, "render_mask": a.render_mask, "draw_fog": a.draw_fog,
    })).collect();
    let items: Vec<_> = game
        .items
        .slots
        .iter()
        .enumerate()
        .filter(|(_, i)| i.processor().is_some())
        .map(|(slot, i)| {
            let model = &game.scene.models[i.model];
            json!({"slot": slot, "kind": i.kind, "processor": format!("{:?}", i.processor().unwrap()),
                "owner": i.owner(), "model": i.model, "parent": model.parent,
                "local_position": position(model.local.translation),
                "mode": i.source_mode(), "count": i.count, "rotation": i.rotation,
                "velocity": i.rolling.velocity, "rolling_status": i.rolling.status.source_id(),
                "collision": {"class": i.collision.class, "paused": i.collision.paused,
                    "size": i.collision.size, "offset_y": i.collision.offset_y},
                "charm_remaining": i.charm_remaining,
                "lightning_start": position(i.lightning.start),
            })
        })
        .collect();
    let ai = &game.intelligence;
    json!({
        "tick": game.tick, "clock": game.clock, "rng": game.random.seed,
        "input": game.input, "player_id": game.player_id,
        "outcome": format!("{:?}", game.events.outcome), "event_time": game.events.time,
        "cinematic": {"active": game.cinematic.active, "sequence": game.cinematic.sequence,
            "elapsed": game.cinematic.elapsed, "tail": game.cinematic.tail},
        "ai": {"difficulty": ai.difficulty.name(), "sight_percent": ai.sight_percent,
            "ignore_player": ai.ignore_player, "alert": ai.alert, "strain": ai.strain,
            "player_strain": ai.player_strain, "found": ai.found, "death_index": ai.death_index,
            "action_halt": ai.action_halt.source_id(), "attack_clock": ai.attack_clock,
            "detection": format!("{:?}", ai.detection), "probe": ai.probe,
            "probe_attribute": ai.probe_attribute, "population_delta": ai.population_delta},
        "activation": {"keep_all_active": game.roster.keep_all_active,
            "raw_budget": game.activation.raw_budget, "budget": game.activation.budget,
            "count": game.activation.count},
        "statistics": {"enemies": game.statistics.enemies, "citizens": game.statistics.citizens,
            "bosses": game.statistics.bosses, "criticals": game.statistics.criticals,
            "murders": game.statistics.murders, "friend_hits": game.statistics.friend_hits},
        "hud": {"selected_item": game.hud.selected, "item_cursor": game.hud.cursor,
            "strain_phase": game.hud.strain_phase},
        "armor": game.combat.armor, "roster_order": game.roster.order,
        "visible_actors": game.visible, "actors": actors, "items": items,
        "camera": {"position": position(game.camera.position), "target": position(game.camera.target),
            "direction": game.camera.direction, "view_translation": position(game.rendered_view.translation),
            "view_rotation": game.rendered_view.rotation},
        "drawing": {"step": format!("{:?}", game.drawing.step), "bytes": game.drawing.bytes,
            "sampled_bytes": game.drawing.sampled_bytes},
    })
}

pub fn encode(mut report: Value, paths: &[String]) -> Result<Vec<u8>> {
    fn redact(value: &mut Value, paths: &[String]) {
        match value {
            Value::String(text) => {
                for path in paths.iter().filter(|p| p.len() > 1) {
                    let normalized = text.to_ascii_lowercase().replace('\\', "/");
                    let path = path.to_ascii_lowercase().replace('\\', "/");
                    for (start, _) in normalized
                        .match_indices(&path)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                    {
                        text.replace_range(start..start + path.len(), "<redacted>");
                    }
                }
            }
            Value::Array(values) => values.iter_mut().for_each(|v| redact(v, paths)),
            Value::Object(values) => values.values_mut().for_each(|v| redact(v, paths)),
            _ => (),
        }
    }
    redact(&mut report, paths);
    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(any(test, not(target_arch = "wasm32")))]
pub fn decode(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Diagnostic report exceeds 16 MiB".into());
    }
    let report: Value =
        serde_json::from_slice(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
            .map_err(|e| format!("Invalid diagnostic JSON: {e}"))?;
    if report["format"] != FORMAT || report["schema_version"] != VERSION {
        return Err("Unsupported diagnostic report format/version".into());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_reports_are_utf8_and_platform_independent() {
        let report = json!({"format": FORMAT, "schema_version": VERSION,
            "platform": {"os": "windows", "version": "10.0.26100"},
            "model": "鬼陰", "tick": 9876543210u64, "strain": i32::MAX,
            "error": "Loading C:\\Users\\Example\\ゲーム\\models: missing texture",
            "nested": ["c:/users/EXAMPLE/ゲーム", "D:\\packs\\artwork"]});
        let bytes = encode(report, &[r"C:\Users\Example".into(), r"D:\packs".into()]).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(!text.to_ascii_lowercase().contains("example") && !text.contains("D:"));
        let windows = format!("\u{feff}{}", text.replace('\n', "\r\n"));
        let parsed = decode(windows.as_bytes()).unwrap();
        assert_eq!(parsed["model"], "鬼陰");
        assert_eq!(parsed["tick"], 9876543210u64);
        assert_eq!(parsed["strain"], i32::MAX);
        assert_eq!(parsed["platform"]["os"], "windows");
        assert_eq!(
            parsed["error"],
            "Loading <redacted>\\ゲーム\\models: missing texture"
        );
        assert!(decode(br#"{"format":"tenchu-diagnostic-report","schema_version":2}"#).is_err());
        assert!(decode(b"not json").is_err());
    }

    #[test]
    #[ignore = "requires TENCHU_DISC"]
    fn snapshot_has_no_assets_and_does_not_change_simulation() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        for _ in 0..120 {
            game.step(0).unwrap();
        }
        let before = crate::session::tests::state(&game);
        let report = super::game(&game);
        assert_eq!(
            report["actors"][game.player_id]["health"],
            game.player().health
        );
        assert_eq!(report["ai"]["alert"], game.intelligence.alert);
        assert_eq!(report["rng"], game.random.seed);
        assert_eq!(crate::session::tests::state(&game), before);
        assert!(serde_json::to_vec(&report).unwrap().len() < 128 * 1024);
        assert!(report.get("mission").is_none());
    }
}
