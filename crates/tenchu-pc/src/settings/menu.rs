use super::Settings;
use crate::{campaign::Action, debug::Debug, render_effects, render_effects::Geometry};
use tenchu_assets::font::Font;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Fps,
    Focus,
    Fullscreen,
    Vsync,
    Hud,
    Volume,
    Reset,
    Fog,
    FogDistance,
    Textures,
    Shading,
    Exposure,
    Filtering,
    Antialiasing,
    Hitboxes,
    Sight,
    SightStance,
    Collision,
    CollisionOnly,
    Patrols,
    Events,
    Labels,
    Xray,
    Trail,
    FreeCamera,
    FlightSpeed,
    Fov,
    FocusPlayer,
    FocusActor,
    Overhead,
    SaveCamera,
    RestoreCamera,
    Actor,
    InspectActor,
    Target(usize),
    PlayerPosition,
    CameraPosition,
    SnapFloor,
    Teleport,
    PickTeleport,
    Pause,
    Step,
    Speed,
    SightDistance,
    CutsceneSpeed,
    Screenshot,
    ResetTweaks,
}

impl Item {
    pub fn button(self) -> bool {
        matches!(
            self,
            Self::Reset
                | Self::FocusPlayer
                | Self::FocusActor
                | Self::Overhead
                | Self::SaveCamera
                | Self::RestoreCamera
                | Self::PlayerPosition
                | Self::CameraPosition
                | Self::Teleport
                | Self::PickTeleport
                | Self::Pause
                | Self::Step
                | Self::Screenshot
                | Self::ResetTweaks
        )
    }
}

pub struct Row {
    pub item: Item,
    pub label: &'static str,
    pub value: String,
    pub help: &'static str,
}

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    pub row: usize,
    pub page: usize,
    pub notice: String,
    pub edit: Option<String>,
}

fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

pub const PAGES: [&str; 6] = [
    "General",
    "Graphics",
    "Overlays",
    "Camera",
    "Teleport / actor",
    "Simulation",
];

pub fn rows(page: usize, s: &Settings, d: &Debug, paused: bool) -> Vec<Row> {
    let entries = match page {
        0 => vec![
            (
                Item::Fps,
                "FPS and tick counter",
                on(s.show_fps),
                "Display frames/second and game tick.",
            ),
            (
                Item::Focus,
                "Pause on defocus",
                on(s.pause_on_defocus),
                "Off: game and audio keep running.",
            ),
            (
                Item::Fullscreen,
                "Fullscreen",
                on(s.fullscreen),
                "Borderless desktop fullscreen.",
            ),
            (
                Item::Vsync,
                "VSync",
                on(s.vsync),
                "Display pacing only; logic stays 30 Hz.",
            ),
            (
                Item::Hud,
                "Gameplay HUD",
                on(s.show_hud),
                "Show health, inventory and proximity.",
            ),
            (
                Item::Volume,
                "Master volume",
                format!("{}%", s.volume),
                "Changes output volume; 0 is mute.",
            ),
            (
                Item::Reset,
                "Restore defaults",
                "Enter".into(),
                "Reset settings to the original experience.",
            ),
        ],
        1 => vec![
            (
                Item::Fog,
                "Black distance fog",
                on(s.graphics.fog),
                "Visual only; AI and activation stay intact.",
            ),
            (
                Item::FogDistance,
                "Fog distance",
                format!("{}%", s.graphics.fog_distance),
                "Scale the original camera-depth fog curve.",
            ),
            (
                Item::Textures,
                "Textures",
                on(s.graphics.textures),
                "Original textures or untextured geometry.",
            ),
            (
                Item::Shading,
                "Vertex shading",
                on(s.graphics.shading),
                "Toggle original baked vertex colors.",
            ),
            (
                Item::Exposure,
                "Exposure",
                format!("{}%", s.graphics.exposure),
                "Scene brightness; leaves UI unchanged.",
            ),
            (
                Item::Filtering,
                "Texture filtering",
                if s.graphics.bilinear {
                    "Linear"
                } else {
                    "Nearest"
                }
                .into(),
                "Bilinear decoded texels; UI stays sharp.",
            ),
            (
                Item::Antialiasing,
                "Edge anti-aliasing",
                if s.graphics.samples == 1 {
                    "Off".into()
                } else {
                    format!("{}x", s.graphics.samples)
                },
                "MSAA: only modes supported by this GPU.",
            ),
        ],
        2 => vec![
            (
                Item::Hitboxes,
                "Body / attack hitboxes",
                on(d.hitboxes),
                "Cyan: body; amber: active attack slots.",
            ),
            (
                Item::Sight,
                "Enemy sight bounds",
                on(d.sight),
                "Calm-state bounds; occlusion not drawn.",
            ),
            (
                Item::SightStance,
                "Sight target stance",
                ["Player", "Stand", "Sneak"][d.sight_stance as usize].into(),
                "Player follows current stance; no AI edits.",
            ),
            (
                Item::Collision,
                "Collision surfaces",
                on(d.collision),
                "ACM floor masks/slopes; not model bounds.",
            ),
            (
                Item::CollisionOnly,
                "Collision only",
                on(d.collision_only),
                "Hide scenery/models to inspect terrain.",
            ),
            (
                Item::Patrols,
                "Patrol paths",
                on(d.patrols),
                "Original routes, projected onto terrain.",
            ),
            (
                Item::Events,
                "Event regions",
                on(d.events),
                "Spatial ESD records, including inactive.",
            ),
            (
                Item::Labels,
                "Actor labels",
                on(d.labels),
                "Live IDs, types and HP; gray: suspended.",
            ),
            (
                Item::Xray,
                "X-ray overlays",
                on(d.xray),
                "Show inspection geometry through walls.",
            ),
            (
                Item::Trail,
                "Player trail",
                on(d.show_trail),
                "Last 1800 sampled updates; about 60 sec.",
            ),
        ],
        3 => vec![
            (
                Item::FreeCamera,
                "Free camera / F2",
                on(d.camera.is_some()),
                "WASD/QE fly, RMB look; no player input.",
            ),
            (
                Item::FlightSpeed,
                "Flight speed",
                format!("{}k/s", d.flight_speed),
                "World units/sec; hold Shift for 4x speed.",
            ),
            (
                Item::Fov,
                "Camera field of view",
                format!("{:.0}", d.camera.map_or(44.0, |c| c.fov)),
                "Free camera only; original view unchanged.",
            ),
            (
                Item::FocusPlayer,
                "Focus player",
                "Enter".into(),
                "Move the inspection camera to the player.",
            ),
            (
                Item::FocusActor,
                "Focus selected actor",
                "Enter".into(),
                "Choose an actor on the Teleport page.",
            ),
            (
                Item::Overhead,
                "Overhead view",
                "Enter".into(),
                "Look down over the selected actor.",
            ),
            (
                Item::SaveCamera,
                "Save camera bookmark",
                "Enter".into(),
                "One camera bookmark for this mission.",
            ),
            (
                Item::RestoreCamera,
                "Recall camera bookmark",
                "Enter".into(),
                "Restore position, heading and field of view.",
            ),
        ],
        4 => vec![
            (
                Item::Actor,
                "Selected actor",
                format!("#{}", d.selected_actor),
                "Left/right selects; click actors in freecam.",
            ),
            (
                Item::InspectActor,
                "Actor state display",
                on(d.inspect_actor),
                "Position, HP, motion, flags and AI programs.",
            ),
            (
                Item::Target(0),
                "Teleport X",
                d.target[0].to_string(),
                "Enter: type exact value. Arrows: +/-100.",
            ),
            (
                Item::Target(1),
                "Teleport Y",
                d.target[1].to_string(),
                "Game coordinates: negative Y is upward.",
            ),
            (
                Item::Target(2),
                "Teleport Z",
                d.target[2].to_string(),
                "Enter: type exact value. Arrows: +/-100.",
            ),
            (
                Item::PlayerPosition,
                "Copy player position",
                "Enter".into(),
                "Set the destination to the player position.",
            ),
            (
                Item::CameraPosition,
                "Copy camera position",
                "Enter".into(),
                "Set destination to the free camera position.",
            ),
            (
                Item::SnapFloor,
                "Snap to collision floor",
                on(d.snap_to_floor),
                "Nearest floor below target, from 1000 above.",
            ),
            (
                Item::Teleport,
                "Teleport player",
                "Enter".into(),
                "GAMEPLAY EDIT: ends the current recording.",
            ),
            (
                Item::PickTeleport,
                "Pick teleport destination",
                "Enter".into(),
                "Close menu and click a collision surface.",
            ),
        ],
        _ => vec![
            (
                Item::Pause,
                "Pause simulation / F3",
                on(paused),
                "Menu pauses too; F3 works without a menu.",
            ),
            (
                Item::Step,
                "Advance one tick / F4",
                "Enter".into(),
                "Run one 1/30s update, then remain paused.",
            ),
            (
                Item::Speed,
                "Simulation speed",
                format!("{}%", d.speed),
                "Host pacing only; every logic tick still runs.",
            ),
            (
                Item::CutsceneSpeed,
                "Auto cutscene speed",
                if s.cutscene_speed == 0 {
                    "Max".into()
                } else {
                    format!("{}x", s.cutscene_speed)
                },
                "Hold Backspace for Max during cutscenes.",
            ),
            (
                Item::SightDistance,
                "Enemy sight distance",
                format!("{}%", d.sight_percent),
                "GAMEPLAY EDIT: sight ranges, not activation.",
            ),
            (
                Item::Screenshot,
                "Screenshot / F9",
                "Enter".into(),
                "Save the current view under screenshots/.",
            ),
            (
                Item::ResetTweaks,
                "Reset simulation tweaks",
                "Enter".into(),
                "Restore 100% speed and original enemy sight.",
            ),
        ],
    };
    entries
        .into_iter()
        .map(|(item, label, value, help)| Row {
            item,
            label,
            value,
            help,
        })
        .collect()
}

impl Menu {
    pub fn page(&mut self, direction: i32) {
        self.page = (self.page as i32 + direction).rem_euclid(PAGES.len() as i32) as usize;
        self.row = 0;
        self.notice.clear();
        self.edit = None;
    }

    pub fn action(&mut self, action: Action, count: usize) -> Option<i32> {
        match action {
            Action::Back => self.open = false,
            Action::Up => self.row = (self.row + count - 1) % count,
            Action::Down => self.row = (self.row + 1) % count,
            Action::Left => return Some(-1),
            Action::Right | Action::Confirm => return Some(1),
            Action::Replay => {}
        }
        None
    }

    pub fn offset(&self) -> usize {
        self.row.saturating_sub(6)
    }

    pub fn build(&self, font: &Font, settings: &Settings, debug: &Debug, paused: bool) -> Geometry {
        let rows = rows(self.page, settings, debug, paused);
        let mut lines = vec![
            (
                -146,
                -108,
                format!("SETTINGS / {}", PAGES[self.page]),
                [1.0, 0.8, 0.4],
            ),
            (
                -146,
                -90,
                format!("< Page {}/{} >  Tab / L1 / R1", self.page + 1, PAGES.len()),
                [0.6, 0.65, 0.7],
            ),
        ];
        for (index, row) in rows.iter().enumerate().skip(self.offset()).take(7) {
            let color = if index == self.row {
                [1.0, 0.8, 0.4]
            } else {
                [1.0; 3]
            };
            let y = -64 + (index - self.offset()) as i16 * 19;
            lines.push((
                -146,
                y,
                format!(
                    "{} {}",
                    if index == self.row { ">" } else { " " },
                    row.label
                ),
                color,
            ));
            lines.push((
                90,
                y,
                if index == self.row {
                    self.edit
                        .as_ref()
                        .map_or_else(|| row.value.clone(), |value| format!("{value}_"))
                } else {
                    row.value.clone()
                },
                color,
            ));
        }
        let help = &rows[self.row.min(rows.len() - 1)].help;
        lines.push((-146, 70, (*help).into(), [0.6, 0.8, 0.85]));
        if !self.notice.is_empty() {
            lines.push((
                -146,
                86,
                self.notice.chars().take(46).collect(),
                [1.0, 0.65, 0.3],
            ));
        }
        lines.push((
            -146,
            102,
            "Arrows: change  F1/Esc/East: close".into(),
            [0.6, 0.65, 0.7],
        ));
        let mut geometry = render_effects::panel(font, &[]);
        for line in lines {
            let start = f32::from(line.0);
            let width = line.2.bytes().map(|c| font.width(c)).sum::<i32>() as f32 + 4.0;
            let scale = ((150.0 - start) / width.max(1.0)).min(1.0);
            let mut text = render_effects::text(font, &[line]);
            for vertex in &mut text.vertices {
                vertex.position[0] = start + (vertex.position[0] - start) * scale;
            }
            geometry.append(text);
        }
        geometry
    }
}
