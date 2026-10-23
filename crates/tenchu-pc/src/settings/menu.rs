use super::Settings;
use crate::{campaign::Action, debug::Debug};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    PlayerModel,
    ReloadModels,
    Preset,
    Interpolation,
    PhotoMode,
    Fps,
    Performance,
    Focus,
    MenuPause,
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
    Magnification,
    SurfaceDetail,
    SurfaceStrength,
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
    KeepEnemiesActive,
    CutsceneSpeed,
    Screenshot,
    ResetTweaks,
    Lighting,
    Shadows,
    FireLights,
    LightRadius,
    LightIntensity,
    LightAmbient,
    LightResolution,
    LightCount,
    ShadowCount,
    ShadowSoftness,
    LampLights,
    LightMarkers,
    FocusLight,
    DebugLight,
    PlaceLight,
    PlayerLight,
    FollowLight,
    LightAxis(usize),
    BlobShadows,
    ShadowPreview,
    AssetPack,
    SharpText,
    ReloadAssets,
    Upscale,
    SmoothModels,
    ModelDetail,
    SkyTint,
    DirectionalIntensity,
    DirectionalColor,
    DirectionalAzimuth,
    DirectionalElevation,
    DirectionalRadius,
    DirectionalResolution,
    Post,
    Bloom,
    BloomThreshold,
    Occlusion,
    OcclusionRadius,
    Filmic,
    Contrast,
    Saturation,
    Temperature,
    Gamma,
    Vignette,
    Sharpen,
    FogTint,
    Haze,
}

impl Item {
    pub fn button(self) -> bool {
        matches!(
            self,
            Self::Reset
                | Self::ReloadModels
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
                | Self::PlaceLight
                | Self::PlayerLight
                | Self::ShadowPreview
                | Self::ReloadAssets
                | Self::FocusLight
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
    pub hovered: Option<Item>,
    pub page: usize,
    pub notice: String,
    pub edit: Option<(Item, String)>,
    pub reveal_row: bool,
    pub player_models: Vec<(String, String)>,
    pub model_directory: String,
}

fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

pub const PAGES: [&str; 11] = [
    "General",
    "Graphics",
    "Overlays",
    "Camera",
    "Teleport / actor",
    "Simulation",
    "Lighting",
    "Visual assets",
    "Sky lighting",
    "Image effects",
    "Player model",
];

pub fn rows(page: usize, s: &Settings, d: &Debug, paused: bool) -> Vec<Row> {
    let entries = match page {
        10 => vec![
            (
                Item::PlayerModel,
                "Player body",
                s.player_model.clone(),
                "Appearance only; weapons, animation and gameplay remain unchanged.",
            ),
            (
                Item::ReloadModels,
                "Load / reload folder",
                "Apply".into(),
                "Load pack.json from the folder above. Failed reloads keep the previous models.",
            ),
        ],
        9 => {
            let post = &s.graphics.post;
            vec![
                (
                    Item::Post,
                    "Image effects",
                    on(post.enabled),
                    "Master switch; HUD remains unchanged.",
                ),
                (
                    Item::Bloom,
                    "Highlight bloom",
                    format!("{}%", post.bloom),
                    "Soft glow around bright surfaces and flames.",
                ),
                (
                    Item::BloomThreshold,
                    "Bloom threshold",
                    format!("{}%", post.bloom_threshold),
                    "Higher values restrict glow to brighter areas.",
                ),
                (
                    Item::Occlusion,
                    "Contact shading",
                    format!("{}%", post.occlusion),
                    "Nearby visible surfaces darken corners.",
                ),
                (
                    Item::OcclusionRadius,
                    "Contact shading radius",
                    format!("{}", post.occlusion_radius * 100),
                    "World units; larger values shade wider corners.",
                ),
                (
                    Item::Filmic,
                    "Filmic tone curve",
                    on(post.filmic),
                    "Roll off bright highlights; deepen dark tones.",
                ),
                (
                    Item::Contrast,
                    "Contrast",
                    format!("{}%", post.contrast),
                    "Contrast around middle gray.",
                ),
                (
                    Item::Saturation,
                    "Saturation",
                    format!("{}%", post.saturation),
                    "0 is monochrome; 100 is neutral.",
                ),
                (
                    Item::Temperature,
                    "Warm / cool balance",
                    post.temperature.to_string(),
                    "Negative cools; positive warms the image.",
                ),
                (
                    Item::Gamma,
                    "Shadow brightness",
                    format!("{}%", post.gamma),
                    "Raise to reveal dark details.",
                ),
                (
                    Item::Vignette,
                    "Vignette",
                    format!("{}%", post.vignette),
                    "Darken image edges; 0 disables.",
                ),
                (
                    Item::Sharpen,
                    "Sharpen",
                    format!("{}%", post.sharpen),
                    "Gentle local contrast; avoid high values.",
                ),
            ]
        }
        8 => {
            let sun = &s.graphics.lighting.directional;
            vec![
                (
                    Item::Lighting,
                    "Dynamic lighting",
                    on(s.graphics.lighting.enabled),
                    "Master switch for ambient, lamps and sky light.",
                ),
                (
                    Item::DirectionalIntensity,
                    "Sun / moon strength",
                    format!("{}%", sun.intensity),
                    "Requires Dynamic lighting; 0 disables sky light.",
                ),
                (
                    Item::DirectionalColor,
                    "Sun / moon color",
                    ["Neutral", "Cool moon", "Warm sun"][sun.color as usize].into(),
                    "Tint of the directional light.",
                ),
                (
                    Item::DirectionalAzimuth,
                    "Light compass angle",
                    format!("{} deg", sun.azimuth),
                    "Rotate the sky light around the scene.",
                ),
                (
                    Item::DirectionalElevation,
                    "Light elevation",
                    format!("{} deg", sun.elevation),
                    "Higher angles shorten shadows.",
                ),
                (
                    Item::DirectionalRadius,
                    "Shadow coverage radius",
                    format!("{}k", sun.radius),
                    "Larger area trades away nearby shadow detail.",
                ),
                (
                    Item::DirectionalResolution,
                    "Sky shadow resolution",
                    sun.resolution.to_string(),
                    "One map, independent of point-light quality.",
                ),
                (
                    Item::SkyTint,
                    "Cool ambient tint",
                    format!("{}%", s.graphics.lighting.sky_tint),
                    "Color retained baked shading toward blue.",
                ),
                (
                    Item::FogTint,
                    "Distance fog color",
                    ["Original black", "Night blue", "Warm dusk", "Gray mist"]
                        [s.graphics.post.fog_tint as usize]
                        .into(),
                    "Requires Image effects; retains fog distance.",
                ),
                (
                    Item::Haze,
                    "Atmospheric haze",
                    format!("{}%", s.graphics.post.haze),
                    "Image effects: extra gradual distance haze.",
                ),
            ]
        }
        7 => vec![
            (
                Item::SharpText,
                "High-definition text",
                on(s.sharp_text),
                "Sharp HUD numbers, subtitles and menu labels; baked artwork keeps its lettering.",
            ),
            (
                Item::AssetPack,
                "Replacement artwork",
                on(s.graphics.asset_pack),
                "Use textures from TENCHU_ASSET_PACK.",
            ),
            (
                Item::ReloadAssets,
                "Reload asset pack",
                "Enter".into(),
                "Reload edited files; retain last good pack.",
            ),
            (
                Item::Upscale,
                "Texture reconstruction",
                match s.graphics.upscale {
                    2 => "Scale2x",
                    4 => "Scale4x",
                    _ => "Original",
                }
                .into(),
                "Smooth pixel stair-steps; no new detail.",
            ),
            (
                Item::SmoothModels,
                "Smooth model lighting",
                on(s.graphics.smooth_models),
                "For dynamic lights; preserve sharp creases.",
            ),
            (
                Item::ModelDetail,
                "Character contours",
                ["Original", "Subtle", "Rounded"][s.graphics.model_detail as usize].into(),
                "Curve original actors; keep joints in place.",
            ),
        ],
        0 => vec![
            (
                Item::Fps,
                "FPS and tick counter",
                on(s.show_fps),
                "Display frames/second and game tick.",
            ),
            (
                Item::Performance,
                "Performance charts",
                on(s.show_performance),
                "Live CPU/GPU stage times; remains visible during play.",
            ),
            (
                Item::Focus,
                "Pause on defocus",
                on(s.pause_on_defocus),
                "Off: game and audio keep running.",
            ),
            (
                Item::MenuPause,
                "Pause while settings are open",
                on(s.pause_in_menu),
                "Off: watch the game live while adjusting settings.",
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
                Item::Preset,
                "Visual preset",
                super::presets::NAMES[s.graphics.preset() as usize].into(),
                "Graphics only; preserves artwork pack preference.",
            ),
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
                [
                    "Nearest",
                    "Bilinear",
                    "Trilinear",
                    "2x aniso",
                    "4x aniso",
                    "8x aniso",
                    "16x aniso",
                ][usize::from(s.graphics.filtering())]
                .into(),
                "Mipmaps reduce shimmer; aniso keeps detail.",
            ),
            (
                Item::Magnification,
                "Close-up textures",
                ["Smooth", "Crisp", "Cubic", "Pixel"][usize::from(s.graphics.magnification)].into(),
                "With filtering: crisp edges, cubic curves, or original pixels; distant mipmaps stay on.",
            ),
            (
                Item::SurfaceDetail,
                "Surface depth",
                ["Off", "Bump", "Parallax"][usize::from(s.graphics.surface_detail)].into(),
                "Opaque scenery: inferred grain and seams respond to lights. Parallax adds view-dependent relief; no geometry changes.",
            ),
            (
                Item::SurfaceStrength,
                "Depth strength",
                format!("{}%", s.graphics.surface_strength),
                "Keep subtle: painted brightness is only a height estimate. Asset packs can supply height maps or disable individual materials.",
            ),
            (
                Item::Interpolation,
                "Smooth model motion",
                on(s.graphics.interpolation),
                "Visual poses and gameplay camera between 30 Hz ticks; adds up to one tick of display delay.",
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
            (
                Item::PhotoMode,
                "Clean photo view / F10",
                on(d.photo_mode),
                "Hide HUD/helpers; F2 camera, F3 pause, F9 shot.",
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
        5 => vec![
            (
                Item::Pause,
                "Pause simulation / F3",
                on(paused),
                "F3 works with the menu open or closed.",
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
                if d.speed == 0 {
                    "Unlimited".into()
                } else {
                    format!("{}%", d.speed)
                },
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
                Item::KeepEnemiesActive,
                "Keep all enemies active",
                on(s.keep_enemies_active),
                "GAMEPLAY EDIT: distant enemies keep thinking and patrolling; ends recording.",
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
                "Restore original speed, enemy sight and activation.",
            ),
        ],
        _ => {
            let light = &s.graphics.lighting;
            let position = d.light.unwrap_or_default().to_array();
            vec![
                (
                    Item::Lighting,
                    "Dynamic lighting",
                    on(light.enabled),
                    "Master switch: light sources plus ambient.",
                ),
                (
                    Item::Shadows,
                    "Model / weapon shadows",
                    on(light.shadows),
                    "Real geometry blocks each enabled light.",
                ),
                (
                    Item::ShadowPreview,
                    "Preview model shadows",
                    "Enter".into(),
                    "Lit player, dark ambient, blob shadows off.",
                ),
                (
                    Item::BlobShadows,
                    "Original blob shadows",
                    on(s.graphics.blob_shadows),
                    "Original ground decals; separate from models.",
                ),
                (
                    Item::FireLights,
                    "Authored fires",
                    on(light.fires),
                    "Nearest fire/bonfire emitters supply lights.",
                ),
                (
                    Item::LightCount,
                    "Point-light budget",
                    light.count.to_string(),
                    "0-16 nearest sources; shadows cost extra.",
                ),
                (
                    Item::LightRadius,
                    "Light radius",
                    format!("{}k", light.radius),
                    "World units; larger radius softens falloff.",
                ),
                (
                    Item::LightIntensity,
                    "Light intensity",
                    format!("{}%", light.intensity),
                    "Strength of added point lighting.",
                ),
                (
                    Item::LightAmbient,
                    "Ambient brightness",
                    format!("{}%", light.ambient),
                    "Retained baked light in shadowed areas.",
                ),
                (
                    Item::LightResolution,
                    "Shadow resolution",
                    light.resolution.to_string(),
                    "Per face: 256-2048. Higher uses more GPU.",
                ),
                (
                    Item::DebugLight,
                    "Movable light",
                    on(d.light.is_some()),
                    "Session-only light; no gameplay effects.",
                ),
                (
                    Item::PlaceLight,
                    "Place light at camera",
                    "Enter".into(),
                    "Freeze a light at the current view position.",
                ),
                (
                    Item::PlayerLight,
                    "Place light above player",
                    "Enter".into(),
                    "Place above/beside player to inspect shadows.",
                ),
                (
                    Item::FollowLight,
                    "Light follows camera",
                    on(d.light_follow),
                    "Move with free camera to inspect occlusion.",
                ),
                (
                    Item::LightAxis(0),
                    "Light X",
                    format!("{:.0}", position[0]),
                    "Arrows move fixed light by 500 world units.",
                ),
                (
                    Item::LightAxis(1),
                    "Light Y",
                    format!("{:.0}", position[1]),
                    "Negative Y is up; disables camera follow.",
                ),
                (
                    Item::LightAxis(2),
                    "Light Z",
                    format!("{:.0}", position[2]),
                    "Arrows move fixed light by 500 world units.",
                ),
                (
                    Item::LampLights,
                    "Lantern / prop lights",
                    on(light.lamps),
                    "Bright panels in named props emit light.",
                ),
                (
                    Item::ShadowCount,
                    "Shadowed point lights",
                    light.shadow_count.to_string(),
                    "0-4 nearest lights; six views per source.",
                ),
                (
                    Item::LightMarkers,
                    "Light source markers",
                    on(light.markers),
                    "Label active sources and shadow casters.",
                ),
                (
                    Item::FocusLight,
                    "Inspect nearest lantern",
                    "Enter".into(),
                    "Focus free camera and show source labels.",
                ),
                (
                    Item::ShadowSoftness,
                    "Shadow filtering radius",
                    light.softness.to_string(),
                    "0: bilinear; 1-8: nine-tap soft edges.",
                ),
            ]
        }
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
        self.hovered = None;
        self.reveal_row = true;
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
        if matches!(action, Action::Up | Action::Down) {
            self.reveal_row = true;
        }
        None
    }
}
