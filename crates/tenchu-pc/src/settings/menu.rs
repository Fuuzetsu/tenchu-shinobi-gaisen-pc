use super::Settings;
use crate::{campaign::Action, debug::Debug};
use tenchu_core::effects::RumbleAction;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Randomizer,
    GodMode,
    Stealth,
    ClickTeleport,
    Inventory(usize),
    GiveItems,
    ClearItems,
    Armor,
    HairMotion,
    ClothMotion,
    EquipmentMotion,
    MotionStrength,
    MotionResponse,
    MotionWind,
    CharacterRefinement,
    FoliageDetail,
    GrappleDetail,
    ContactSparks,
    Lightning,
    SparkDensity,
    ContactSplashes,
    ContactScatter,
    FallingLeaves,
    LeafDensity,
    VegetationVariation,
    Water,
    WaterRipples,
    WaterReflections,
    WaterFlow,
    WaterFoam,
    WaterSpray,
    WaterDetail,
    Weather,
    WeatherAmount,
    WeatherWind,
    WeatherHeading,
    WeatherQuality,
    WeatherDistance,
    Wetness,
    RainReflections,
    PuddleCoverage,
    PoolingSpeed,
    RainHaze,
    RainStreaks,
    RunoffAmount,
    SnowCover,
    SnowDrift,
    Footprints,
    FootprintSeconds,
    ClearFootprints,
    ClearWeather,
    RoofRunoff,
    TimeClock,
    TimeOfDay,
    TimeSpeed,
    SunStrength,
    MoonStrength,
    DayAmbient,
    NightAmbient,
    SunPath,
    DayRelighting,
    Blood,
    BloodSpray,
    BloodAmount,
    BloodSeconds,
    BloodMarks,
    ClearBlood,
    PlayerModel,
    ModelMoveset,
    ModelPlayerMovement,
    ModelNoIdleTaunts,
    ReloadModels,
    Preset,
    GameplayPreset,
    Retro,
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
    Vibration,
    VibrationIntensity,
    VibrationAction(RumbleAction),
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
    LightSourceSize,
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
                | Self::Randomizer
                | Self::GiveItems
                | Self::ClearItems
                | Self::ClearBlood
                | Self::ClearFootprints
                | Self::ClearWeather
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
    pub artwork_status: String,
}

fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

pub const PAGES: [&str; 20] = [
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
    "Persistent effects",
    "Time of day",
    "Weather",
    "Water",
    "Model refinement",
    "Secondary motion",
    "Experiment",
    "Contact effects",
    "Vibration",
];

pub fn rows(page: usize, s: &Settings, d: &Debug, paused: bool) -> Vec<Row> {
    let gameplay = || {
        let preset = super::gameplay::current(s, d).map(|i| &super::gameplay::PRESETS[i]);
        (Item::GameplayPreset, "Gameplay preset", preset.map_or("Custom", |p| p.name).into(),
            preset.map_or("Group enemy activation, sight, cutscene shortcut and practice tools. Graphics and inventory stay unchanged.", |p| p.help))
    };
    let entries = match page {
        19 => {
            let mut rows = vec![
                (
                    Item::Vibration,
                    "Controller vibration",
                    on(s.frontend.rumble),
                    "Enable controller motors. Shared with the original menu's vibration option; requires a controller with force feedback.",
                ),
                (
                    Item::VibrationIntensity,
                    "Global intensity",
                    format!("{}%", s.vibration.intensity),
                    "0-200%, multiplied by each action's strength. 100% preserves the original strength; 0 mutes all vibration. Motor output is capped at hardware maximum.",
                ),
            ];
            rows.extend(RumbleAction::ALL.map(|action| {
                let (label, help) = super::vibration::description(action);
                (
                    Item::VibrationAction(action),
                    label,
                    format!("{}%", s.vibration.intensity(action)),
                    help,
                )
            }));
            rows
        }
        18 => vec![
            (
                Item::GrappleDetail,
                "Detailed rope / hook",
                on(s.graphics.models.grapple),
                "Braided round rope and refined hook geometry. The original grapple path, timing and collisions are unchanged.",
            ),
            (
                Item::ContactSparks,
                "Grapple impact sparks",
                on(s.graphics.effects.sparks),
                "Small hot fragments and a short local light at hook contact; cosmetic only.",
            ),
            (
                Item::Lightning,
                "Modern lightning",
                on(s.graphics.effects.lightning),
                "Mei-Oh's bolts gain bright cores, branching arcs and impact sparks. Dynamic lighting adds a brief blue flash; damage and timing stay original.",
            ),
            (
                Item::SparkDensity,
                "Spark amount",
                format!("{}%", s.graphics.effects.spark_density),
                "0-300% visual particles; does not consume gameplay RNG.",
            ),
            (
                Item::ContactSplashes,
                "Water contact splashes",
                on(s.graphics.effects.splashes),
                "Replace the original water sprite with droplets and ripples; nearby dry ground receives fading splash marks.",
            ),
            (
                Item::ContactScatter,
                "Footstep / landing scatter",
                on(s.graphics.effects.scatter),
                "Walk, run and land through snow, puddles and fallen petals/leaves. Amount follows movement; game rules remain unchanged.",
            ),
            (
                Item::FallingLeaves,
                "Falling petals / leaves",
                on(s.graphics.effects.leaves),
                "Seasonal foliage drifts from tagged tree canopies, settles briefly, then fades. Wind redirects it.",
            ),
            (
                Item::LeafDensity,
                "Falling foliage amount",
                format!("{}%", s.graphics.effects.leaf_density),
                "0-200%; airborne and ground litter have bounded budgets.",
            ),
        ],
        17 => {
            let mut rows = vec![
                (
                    Item::GodMode,
                    "God mode",
                    on(d.god_mode),
                    "Prevent combat and fall damage. Hit reactions still play; enabling does not refill health.",
                ),
                (
                    Item::Stealth,
                    "Never noticed",
                    on(d.stealth),
                    "Enemies ignore the player and return to their idle/patrol behavior. Cutscene scripts remain active.",
                ),
                (
                    Item::ClickTeleport,
                    "Click to teleport",
                    on(d.click_teleport),
                    "Close F1, then left-click a collision floor to move there. Works in the normal or free camera; Escape cancels the mode.",
                ),
                (
                    Item::GiveItems,
                    "Give all equipment",
                    "10 each".into(),
                    "Refill mission equipment and the grappling hook. No edits to campaign stock or unlocks.",
                ),
                (
                    Item::ClearItems,
                    "Clear equipment",
                    "Clear".into(),
                    "Empty carried equipment. Already active items finish normally.",
                ),
                (
                    Item::Armor,
                    "Armour protection",
                    on(d.armor),
                    "Toggle the armour damage reduction. The equipped outfit is chosen when the mission loads.",
                ),
            ];
            rows.extend(d.inventory.iter().enumerate().map(|(slot, count)| (
                Item::Inventory(slot), crate::ui::ITEMS[slot], format!("{count}"),
                "Edit the mission quantity: 0 removes the item, 255 is unlimited. Internal item timers are preserved.",
            )));
            rows
        }

        16 => {
            let motion = &s.graphics.motion;
            vec![
                (
                    Item::HairMotion,
                    "Hair motion",
                    on(motion.hair),
                    "Bounded inertia on authored hair regions. The original pose, face, collision and gameplay stay unchanged.",
                ),
                (
                    Item::ClothMotion,
                    "Loose cloth",
                    on(motion.cloth),
                    "Cosmetic follow-through on loose garment hems and Ayame's bow. Anchored portions remain attached.",
                ),
                (
                    Item::EquipmentMotion,
                    "Equipment flex",
                    on(motion.equipment),
                    "Restrained flex of supported carried equipment. Weapon animation and hitboxes remain original.",
                ),
                (
                    Item::MotionStrength,
                    "Motion strength",
                    format!("{}%", motion.strength),
                    "Scales the bounded displacement. Hair, clothing and equipment use separate authored limits.",
                ),
                (
                    Item::MotionResponse,
                    "Spring stiffness",
                    format!("{}%", motion.response),
                    "Higher values settle more quickly; lower values allow softer follow-through.",
                ),
                (
                    Item::MotionWind,
                    "Breeze",
                    format!("{}%", motion.wind),
                    "Gentle wind in tagged vegetation and enabled hair/cloth/equipment. Tree roots stay fixed and shadows follow the same deformation.",
                ),
            ]
        }
        15 => vec![
            (
                Item::CharacterRefinement,
                "Character surfaces",
                ["Original", "Refined", "Fine"][usize::from(s.graphics.models.characters)].into(),
                "Refine coarse character faces with welded subdivision and smooth normals. Open joint boundaries and original UV seams stay attached; custom replacement models are retained.",
            ),
            (
                Item::VegetationVariation,
                "Varied tree shapes",
                on(s.graphics.models.variation),
                "Stable per-tree crown shapes, proportions and branch asymmetry. Trunk bases stay anchored; collision remains original.",
            ),
            (
                Item::FoliageDetail,
                "Vegetation volume",
                [
                    "Original",
                    "Fuller",
                    "Dense",
                    "Botanical",
                    "Botanical dense",
                ][usize::from(s.graphics.models.foliage)]
                .into(),
                "Botanical adds folded leaf sprays around the original canopy and refines shrubs/trunks. Collision and AI visibility remain original.",
            ),
        ],
        14 => {
            let water = &s.graphics.water;
            vec![
                (
                    Item::Water,
                    "Modern water",
                    on(water.enabled),
                    "Optional water shading, waterfall curves and spray. Material tags keep terrain, lava and water separate.",
                ),
                (
                    Item::WaterRipples,
                    "Ripple strength",
                    format!("{}%", water.ripples),
                    "Continuous world-space ripples and falling-water filaments; fades distant detail to avoid shimmer.",
                ),
                (
                    Item::WaterReflections,
                    "Sky reflections",
                    format!("{}%", water.reflections),
                    "View-dependent sky sheen and sun/moon highlights; does not mirror scene objects.",
                ),
                (
                    Item::WaterFlow,
                    "Visual flow",
                    format!("{}%", water.flow),
                    "Speed of extra ripples, foam and spray; original texture animation and gameplay are unchanged.",
                ),
                (
                    Item::WaterFoam,
                    "Foam",
                    format!("{}%", water.foam),
                    "Subtle crest foam and moving aerated waterfall streaks.",
                ),
                (
                    Item::WaterSpray,
                    "Waterfall spray",
                    format!("{}%", water.spray),
                    "Soft droplets and mist at open downstream edges of waterfall geometry.",
                ),
                (
                    Item::WaterDetail,
                    "Waterfall curves",
                    ["Original", "Curved", "Fine curves"][usize::from(water.detail)].into(),
                    "Subdivide and curve connected waterfall faces while pinning open boundaries. Original collision stays unchanged.",
                ),
            ]
        }

        13 => {
            let weather = &s.graphics.weather;
            vec![
                (
                    Item::Weather,
                    "Weather",
                    weather.mode.name().into(),
                    "Original retains the mission weather. Clear, Rain and Snow replace its presentation only.",
                ),
                (
                    Item::ClearWeather,
                    "Clear weather now",
                    "Clear".into(),
                    "Remove snow, wetness, puddles, tracks and particles immediately. Stops precipitation; select Rain or Snow to restart.",
                ),
                (
                    Item::WeatherAmount,
                    "Precipitation",
                    format!("{}%", weather.amount),
                    "Rain or snow density; shelter and surfaces stop falling particles.",
                ),
                (
                    Item::WeatherWind,
                    "Wind strength",
                    format!("{}%", weather.wind),
                    "Wind tilts rain and snow, wetting/dusting windward surfaces while roofs and walls shelter areas downwind. Gameplay is unchanged.",
                ),
                (
                    Item::WeatherHeading,
                    "Wind compass",
                    format!("{} deg", weather.heading),
                    "Direction rain and snow travel around the stage. Turning it changes which walls receive precipitation.",
                ),
                (
                    Item::WeatherQuality,
                    "Weather detail",
                    ["Low", "Medium", "High"][usize::from(weather.quality)].into(),
                    "Particle budget and shelter-map resolution. Low is useful for browsers and large windows.",
                ),
                (
                    Item::WeatherDistance,
                    "Particle distance",
                    format!("{}k units", weather.distance),
                    "Near and distant precipitation share a bounded particle budget. Ground snow/wetness covers the whole stage independently.",
                ),
                (
                    Item::Wetness,
                    "Surface wetness",
                    format!("{}%", weather.wetness),
                    "Maximum surface wetness; builds in rain and dries slowly in clear weather.",
                ),
                (
                    Item::RainReflections,
                    "Wet reflections",
                    format!("{}%", weather.reflections),
                    "Strength of sky, sun/moon and nearby light highlights on wet surfaces and puddles. Lamp highlights need dynamic lighting; these are not mirror images of scenery.",
                ),
                (
                    Item::PuddleCoverage,
                    "Puddle coverage",
                    format!("{}%", weather.puddles),
                    "Shallow reflective patches and rain ripples on exposed, nearly level surfaces. Cosmetic coverage, without flooding or collision changes.",
                ),
                (
                    Item::PoolingSpeed,
                    "Pooling speed",
                    format!("{:.2}x", f32::from(weather.pooling) / 100.0),
                    "How quickly rain wets surfaces and forms puddles. 1x wets in about 20 seconds and pools over a minute at full rain; 10x is ten times faster. Drying stays gradual.",
                ),
                (
                    Item::RainHaze,
                    "Rain haze",
                    format!("{}%", weather.haze),
                    "Rain obscures distant views, independently of black fog. Sheltered sections of the view stay clearer. Visual only: enemy detection is unchanged.",
                ),
                (
                    Item::RainStreaks,
                    "Rain streak visibility",
                    format!("{}%", weather.streaks),
                    "Brightness and width of falling rain and splashes, without increasing the particle budget.",
                ),
                (
                    Item::SnowCover,
                    "Snow cover",
                    format!("{}%", weather.snow),
                    "Snow builds on exposed upward surfaces and dusts windward walls. Sheltered and leeward surfaces stay clear; coverage melts when snow stops.",
                ),
                (
                    Item::SnowDrift,
                    "Snow drift softness",
                    format!("{}%", weather.snow_drift),
                    "Broaden snow shelter edges to approximate varying wind. High weather detail improves accuracy without sharpening tree silhouettes.",
                ),
                (
                    Item::Footprints,
                    "Snow footprints",
                    on(weather.footprints),
                    "Players and humanoid enemies leave grounded tracks in snow. Fresh snowfall gradually covers them.",
                ),
                (
                    Item::FootprintSeconds,
                    "Fresh-snow burial",
                    format!("{} s", weather.footprint_seconds),
                    "Time for full snowfall to cover tracks; lighter snow takes longer. Tracks also fade slowly when snowfall stops.",
                ),
                (
                    Item::ClearFootprints,
                    "Clear footprints",
                    "Clear".into(),
                    "Remove retained tracks without resetting snow coverage.",
                ),
                (
                    Item::RoofRunoff,
                    "Roof runoff",
                    on(weather.runoff),
                    "Raindrops run downhill on sloped roofs and drip from edges.",
                ),
                (
                    Item::RunoffAmount,
                    "Roof drip amount",
                    format!("{}%", weather.runoff_amount),
                    "Density of roof rivulets and recurring drips from exposed eaves. Scales with rainfall and accumulated wetness.",
                ),
            ]
        }

        12 => {
            let sky = &s.graphics.sky;
            vec![
                (
                    Item::TimeClock,
                    "Sun / moon cycle",
                    on(sky.enabled),
                    "Optional moving sky and lighting; replaces the manual sky light while enabled.",
                ),
                (
                    Item::TimeOfDay,
                    "Start time",
                    format!("{:02}:{:02}", sky.time / 60, sky.time % 60),
                    "24-hour clock. Editing restarts the visual clock here; 06:00 sunrise, 18:00 sunset.",
                ),
                (
                    Item::TimeSpeed,
                    "Time lapse",
                    format!("{} min/s", sky.speed),
                    "Game minutes per second of simulation; 0 holds the selected time. Pausing also pauses the sky.",
                ),
                (
                    Item::SunStrength,
                    "Sun strength",
                    format!("{}%", sky.sun),
                    "Daylight intensity; sunrise and sunset fade smoothly.",
                ),
                (
                    Item::MoonStrength,
                    "Moon strength",
                    format!("{}%", sky.moon),
                    "Moonlight is weaker than sunlight at the same percentage.",
                ),
                (
                    Item::DayAmbient,
                    "Day ambient",
                    format!("{}%", sky.day_ambient),
                    "Diffuse sky light at midday; blends through twilight.",
                ),
                (
                    Item::NightAmbient,
                    "Night ambient",
                    format!("{}%", sky.night_ambient),
                    "Diffuse sky light at midnight; local lamps still fill shadows.",
                ),
                (
                    Item::SunPath,
                    "Sun path compass",
                    format!("{} deg", sky.heading),
                    "Rotate the whole sun and moon path around the stage.",
                ),
                (
                    Item::DayRelighting,
                    "Daylight relighting",
                    format!("{}%", sky.relighting),
                    "Fade the original baked night shading in daylight; texture artwork and actual moving shadows remain.",
                ),
                (
                    Item::Shadows,
                    "Dynamic shadows",
                    on(s.graphics.lighting.shadows),
                    "Cast shadows from models, weapons and scenery as the light moves.",
                ),
                (
                    Item::DirectionalResolution,
                    "Sky shadow resolution",
                    s.graphics.lighting.directional.resolution.to_string(),
                    "Shared with the manual sky light; one shadow map for the current sun or moon.",
                ),
                (
                    Item::DirectionalRadius,
                    "Shadow coverage radius",
                    format!("{}k", s.graphics.lighting.directional.radius),
                    "Larger area trades nearby shadow detail for coverage.",
                ),
            ]
        }
        11 => vec![
            (
                Item::Blood,
                "Persistent blood",
                on(s.graphics.blood.enabled),
                "Wounds stain bodies and the environment without fading over time. Marks last until cleared, their actor/mission is unloaded, or the retention budget fills. Off clears them.",
            ),
            (
                Item::BloodSpray,
                "Modern blood spray",
                on(s.graphics.blood.spray),
                "Replace airborne blood balls with soft, stretched droplets following the original trajectories. Cosmetic only.",
            ),
            (
                Item::BloodAmount,
                "Blood amount",
                format!("{}%", s.graphics.blood.amount),
                "Size of persistent stains and frequency of wound drips; does not affect damage or the original effect lifetime.",
            ),
            (
                Item::BloodSeconds,
                "Wound bleeding",
                format!("{} s", s.graphics.blood.seconds),
                "How long new wounds drip; 0 keeps stains without extra bleeding.",
            ),
            (
                Item::BloodMarks,
                "Retained stains",
                s.graphics.blood.marks.to_string(),
                "Oldest marks are removed at this limit. A separate geometry budget bounds memory.",
            ),
            (
                Item::ClearBlood,
                "Clear blood",
                "Clear".into(),
                "Remove cosmetic stains and stop current bleeding without changing health.",
            ),
        ],
        10 => vec![
            (
                Item::PlayerModel,
                "Player body",
                s.player_model.clone(),
                "Appearance only; weapons, animation and gameplay remain unchanged.",
            ),
            (
                Item::ModelMoveset,
                "Use model's moveset",
                on(s.model_moveset),
                "Use this actor's animations, attacks and weapon profile. Missing player actions fall back to your protagonist. Original and custom folders keep player moves.",
            ),
            (
                Item::ModelPlayerMovement,
                "Use player movement",
                on(s.model_moveset_options.player_movement),
                "With model moveset enabled, use Ayame's or Rikimaru's walking, running, dashes, crouching, jumping and climbing motions and speeds. Keep the selected actor's attacks.",
            ),
            (
                Item::ModelNoIdleTaunts,
                "Disable idle taunts",
                on(s.model_moveset_options.no_idle_taunts),
                "With model moveset enabled, prevent the player's random idle fidgets and combat taunts from locking movement. Enemies and scripted actions are unchanged.",
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
                "Sharp HUD numbers, subtitles, mission titles, ranks and menu labels.",
            ),
            (
                Item::AssetPack,
                "Replacement artwork",
                on(s.graphics.asset_pack),
                "Use the configured texture pack, including world, HUD and menu artwork.",
            ),
            (
                Item::ReloadAssets,
                "Reload asset pack",
                "Enter".into(),
                "Reload the configured pack; browsers use the site's cached export. Retain the last good pack on errors.",
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
                Item::Randomizer,
                "Randomizer (F7)",
                "Open…".into(),
                "Configure a separate seeded run with full-disc player/enemy pools, stages, HP and equipment.",
            ),
            gameplay(),
            (
                Item::Preset,
                "Graphics preset",
                super::presets::NAMES[s.graphics.preset() as usize].into(),
                "High, Ultra and Showcase enable the visual improvements together. Showcase emphasizes lighting and effects. Weather type and gameplay remain your choice.",
            ),
            (
                Item::Retro,
                "Retro comparison (F8)",
                on(s.retro),
                "Preview Original graphics, textures, fonts and models. Toggle back to your exact visual settings; gameplay settings stay unchanged.",
            ),
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
                Item::Retro,
                "Retro comparison (F8)",
                on(s.retro),
                "Preview Original graphics, textures, fonts and models. Toggle back to your exact visual settings; gameplay settings stay unchanged.",
            ),
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
                "Calm-state sight bounds clipped by collision walls, floors and ceilings. No AI edits.",
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
            gameplay(),
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
                    "Fire / explosion lights",
                    on(light.fires),
                    "Fires, grenades, burning embers and kaen flames illuminate nearby surfaces. Transient lights share the light and shadow budgets.",
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
                    "0-8 nearby lights; gradual handovers, six views per source. High/Ultra cover more lights before you approach.",
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
                    Item::LightSourceSize,
                    "Light source size",
                    light.source_size.to_string(),
                    "Finite emitter radius in world units. Softens grating shadows as they travel; 0 restores point sources.",
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
