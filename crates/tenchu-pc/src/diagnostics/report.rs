use crate::{controls::Controls, debug::Debug, settings::Settings};
use serde_json::{Value, json};
use tenchu_assets::Result;

pub fn report(settings: &Settings, debug: &Debug, controls: &Controls) -> Value {
    let mut settings = serde_json::to_value(settings).expect("serializable settings");
    if settings["model_directory"]
        .as_str()
        .is_some_and(|s| !s.is_empty())
    {
        settings["model_directory"] = "<redacted>".into();
    }
    json!({
        "format": super::FORMAT, "schema_version": super::VERSION,
        "created_unix_ms": web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64),
        "build": super::build(), "platform": super::environment::platform(),
        "notes": ["No disc assets, raw process memory, saved campaign or filesystem paths are required to read this report.",
            "Snapshot and recent inputs are diagnostic context, not a restorable savestate or complete replay.",
            "Game positions use original integer coordinates. See input_encoding for pad bitmasks. Timings are milliseconds.",
            "Personal path roots are redacted. Review custom names and errors before sharing. Nothing is uploaded automatically."],
        "settings": settings, "debug": debug_settings(debug),
        "controls": controls.diagnostic_settings(),
    })
}

fn debug_settings(debug: &Debug) -> Value {
    json!({"simulation_speed_percent": debug.speed, "god_mode": debug.god_mode,
                "stealth": debug.stealth, "click_teleport": debug.click_teleport, "armor": debug.armor,
                "sight_percent": debug.sight_percent, "photo_mode": debug.photo_mode,
                "hitboxes": debug.hitboxes, "sight": debug.sight, "sight_stance": debug.sight_stance,
                "collision": debug.collision, "collision_only": debug.collision_only,
                "patrols": debug.patrols, "events": debug.events, "labels": debug.labels,
                "xray": debug.xray, "show_trail": debug.show_trail, "inspect_actor": debug.inspect_actor,
                "actors_edited": debug.actors_edited, "imported_actor_models": debug.actor_imports.keys().collect::<Vec<_>>(),
                "camera_actor": debug.camera_actor.map(|subject| subject.id),
                "selected_actor": debug.selected_actor, "target": debug.target.to_array(),
                "snap_to_floor": debug.snap_to_floor, "teleport_armed": debug.teleport_armed,
                "flight_speed": debug.flight_speed, "pause_view": debug.pause_view,
                "light": debug.light.map(|p| p.to_array()), "light_follow": debug.light_follow,
                "free_camera": debug.camera.map(|c| json!({"position": c.position.to_array(),
                    "yaw_radians": c.yaw, "pitch_radians": c.pitch, "fov_degrees": c.fov}))})
}

pub fn finish(
    report: Value,
    settings: &Settings,
    store: Option<&std::path::Path>,
) -> Result<Vec<u8>> {
    let mut paths = vec![settings.model_directory.clone()];
    if let Some(store) = store {
        paths.push(store.to_string_lossy().into_owned());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        for key in [
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
        ] {
            if let Some(path) = std::env::var_os(key) {
                paths.push(path.to_string_lossy().into_owned());
            }
        }
        for path in [
            std::env::current_dir().ok(),
            crate::visuals::pack::configured_root(),
        ]
        .into_iter()
        .flatten()
        {
            paths.push(path.to_string_lossy().into_owned());
        }
    }
    paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
    crate::diagnostics::encode(report, &paths)
}
