use super::Settings;
use crate::{campaign::Action, debug::Debug};
use tenchu_core::effects::RumbleAction;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Tenchu2Missions,
    Tenchu2Diagnostics,
    Tenchu2Launcher,
    Dismemberment,
    HeadRest,
    SaExecutionScene,
    SeverStealth,
    SeverDive,
    SeverArms,
    SeverSide,
    PlayerDismemberment,
    PlayerSeverHead,
    PlayerSeverArms,
    PlayerSeverSide,
    SeveredBlood,
    SeveredStyle,
    SeveredSeconds,
    Bleeding,
    BleedChance,
    BleedSeconds,
    BleedDamage,
    BandageLoadout,
    Bandages,
    UseBandage,
    MedicineBleeding,
    StateSlot,
    SaveState,
    LoadState,
    RewindHistory,
    RewindSpeed,
    Rewind,
    CameraRelative,
    GuardControls,
    GuardDraw,
    AnalogWalking,
    WalkThreshold,
    Sheathing,
    UnarmedAttacks,
    UnarmedStyle,
    SheathedSpeed,
    SheathedVisibility,
    SheathedDamage,
    SheathedOneHit,
    StaySheathed,
    AlignAim,
    MouseSensitivity,
    StickSensitivity,
    StickDeadzone,
    InvertCameraY,
    FollowDistance,
    RecenterOrbit,
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
    HeroModels,
    FoliageDetail,
    GrappleDetail,
    ContactSparks,
    Lightning,
    Gunfire,
    SparkDensity,
    ContactSplashes,
    ContactScatter,
    FallingLeaves,
    LeafDensity,
    VegetationVariation,
    Water,
    WaterReflective,
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
    BakedSnowFootprints,
    StormLightning,
    StormBolts,
    StormInterval,
    StormThunder,
    ThunderStyle,
    RainVolume,
    WindVolume,
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
    BloodHit,
    BloodDrips,
    BloodDeath,
    BloodGround,
    BloodRetain,
    BloodConform,
    BloodModels,
    BloodWeapons,
    WeaponBloodMetalOnly,
    WeaponBloodCoverage,
    WeaponBloodFlow,
    WeaponReflections,
    BloodRunoff,
    BloodPlayerDeath,
    BloodAmount,
    BloodDeathSize,
    BloodSeconds,
    BloodSpurts,
    DeathGush,
    GushStyle,
    GushSeconds,
    BloodMarks,
    ClearBlood,
    PlayerModel,
    ModelMoveset,
    MovesetModel,
    ModelPlayerMovement,
    ModelNoIdleTaunts,
    AlternateDeathChance,
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
    MusicWhilePaused,
    CircleConfirm,
    WaitAfterDeath,
    LiveDeathScene,
    ShopLighting,
    Fullscreen,
    Vsync,
    Hud,
    Volume,
    BurningSpatial,
    BurningVolume,
    BurningRange,
    Vibration,
    VibrationIntensity,
    VibrationAction(RumbleAction),
    Reset,
    Fog,
    FogDistance,
    DistantDoors,
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
    FollowActor,
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
    SpawnActor,
    MoveActor,
    RemoveActor,
    ActorPosition,
    PickActorPosition,
    Pause,
    Step,
    Speed,
    SightDistance,
    KeepEnemiesActive,
    IndependentAttacks,
    QuickShurikenDodge,
    StrictCorpseSight,
    DetectionDamage,
    DetectionDamageLimit,
    MultiTargetAttacks,
    AttackAssist,
    BehaviorProfiles,
    BehaviorCombat,
    BehaviorIdle,
    BehaviorQuickTurns,
    BehaviorProgression,
    BehaviorSeed,
    ActorCombat,
    ActorIdle,
    ReinforcementsPerCall,
    RandomReinforcements,
    WitnessedKillReinforcements,
    ReinforcementLimit,
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
    FlameMotion,
    HeatShimmer,
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
    EnglishShuriken,
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
            Self::Tenchu2Missions
                | Self::Tenchu2Diagnostics
                | Self::Tenchu2Launcher
                | Self::UseBandage
                | Self::Reset
                | Self::RecenterOrbit
                | Self::Randomizer
                | Self::SaveState
                | Self::LoadState
                | Self::Rewind
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
    pub touch_controls: bool,
    pub model_picker: bool,
    pub open: bool,
    pub row: usize,
    pub hovered: Option<Item>,
    pub page: usize,
    pub notice: String,
    pub edit: Option<(Item, String)>,
    pub reveal_row: bool,
    pub player_models: Vec<(String, String)>,
    pub moveset_models: Vec<(String, String)>,
    pub moveset_search: String,
    pub model_directory: String,
    pub spawn_model: String,
    pub spawn_search: String,
    pub spawn_yaw: i16,
    pub artwork_status: String,
}

fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

pub const PAGES: [&str; 24] = [
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
    "Movement",
    "Enemy behavior",
    "Bleeding",
    "Tenchu 2",
];

pub fn rows(page: usize, s: &Settings, d: &Debug, paused: bool) -> Vec<Row> {
    let gameplay = || {
        let preset = super::gameplay::current(s, d).map(|i| &super::gameplay::PRESETS[i]);
        (Item::GameplayPreset, "Gameplay preset", preset.map_or("Custom", |p| p.name).into(),
            preset.map_or("Group enemy activation, sight, cutscene shortcut and practice tools. Graphics and inventory stay unchanged.", |p| p.help))
    };
    let entries = match page {
        23 => vec![
            (
                Item::Tenchu2Missions,
                "JP mission selection",
                "Open".into(),
                "Leave the current mission and open Tenchu 2 Japan's original mission selector.",
            ),
            (
                Item::Tenchu2Diagnostics,
                "Integration diagnostics",
                "Open".into(),
                "Inspect the native JP objective, actors, current scene and integration status.",
            ),
            (
                Item::Tenchu2Launcher,
                "Quick launcher · F6",
                "Open".into(),
                "Leave native JP play and return to the quick launcher. Closes the standalone JP player.",
            ),
        ],
        22 => vec![
            (
                Item::Bleeding,
                "Player bleeding",
                on(s.bleeding.enabled),
                "Optional gameplay: enemy melee and piercing projectiles can cause blood loss. Refreshes duration instead of stacking, leaves at least 1 HP, and pauses during scripted scenes. Cosmetic blood settings are independent. Disables input-only recordings and TRIAL records.",
            ),
            (
                Item::BleedChance,
                "Chance per damaging hit",
                format!("{}%", s.bleeding.chance),
                "0–100%. Blocked hits, god mode, fire, explosions and fall damage do not start bleeding. Uses a separate seeded generator.",
            ),
            (
                Item::BleedSeconds,
                "Bleeding duration",
                if s.bleeding.seconds == 0 {
                    "Until treated".into()
                } else {
                    format!("{} s", s.bleeding.seconds)
                },
                "0 means until treated; otherwise 1–120 seconds. Further wounds refresh this duration without increasing damage per second.",
            ),
            (
                Item::BleedDamage,
                "Blood loss per second",
                format!("{} HP", s.bleeding.damage),
                "0–20 HP per second; never reduces health below 1. God mode prevents blood loss. Changing cosmetic blood does not change this.",
            ),
            (
                Item::BandageLoadout,
                "Bandages per mission",
                s.bleeding.bandages.to_string(),
                "0–99 carried bandages on mission start, selectable with normal item controls. Does not replace any disc item or change campaign stock; does not refill the current mission.",
            ),
            (
                Item::MedicineBleeding,
                "Medicine also stops bleeding",
                on(s.bleeding.medicine),
                "Cures bleeding when the healing potion actually takes effect. Resurrection always cures bleeding.",
            ),
            (
                Item::Bandages,
                "Carried bandages",
                d.bandages
                    .map_or_else(|| "Disabled".into(), |n| n.to_string()),
                "Edit the current mission count for experiments (0–99). Only available with Player bleeding enabled.",
            ),
            (
                Item::UseBandage,
                "Use bandage",
                "Treat".into(),
                "Stops bleeding without healing; consumes one only when bleeding. Select Bandage with L2/R2 and use Triangle, or quick-use with B (rebind in Controls). Must be grounded and free to act. From here, treatment happens on the next gameplay tick.",
            ),
        ],
        21 => vec![
            (
                Item::BehaviorProfiles,
                "Enemy behavior profiles",
                on(s.behavior_profiles.enabled),
                "GAMEPLAY EDIT: optional decisions for ordinary guards. Original AI is the default. Story actors, bosses, civilians and animals are protected. Perception, damage and collision rules remain; disables input recording and TRIAL records.",
            ),
            (
                Item::BehaviorCombat,
                "Combat profile",
                s.behavior_profiles.combat.name().into(),
                "Coward retreats to seek one bounded reinforcement group per alert. Reckless rushes and attacks; Cautious retreats and waits for openings; Flanker sidesteps; Watchman adapts to distance, health and attacks. Seeded mix chooses per actor. These profiles use no items.",
            ),
            (
                Item::BehaviorIdle,
                "Undetected behavior",
                s.behavior_profiles.idle.name().into(),
                "Keep authored behavior, follow an existing patrol route (look out if none), watch from one spot, or turn periodically toward the player. Sleeping guards and scripted actions retain their authored behavior.",
            ),
            (
                Item::BehaviorQuickTurns,
                "Patrol quick turns",
                on(s.behavior_profiles.quick_turns),
                "Allow the original rapid turnaround at the end of a patrol loop when Follow patrol is selected. Off turns naturally toward the next point.",
            ),
            (
                Item::BehaviorProgression,
                "Later-mission progression",
                on(s.behavior_profiles.progression),
                "Optional starting thresholds: Mission 6 on Easy, 4 on Normal, 2 on Hard. Earlier missions retain authored AI. Off applies to all missions; TRIAL maps always allow profiles.",
            ),
            (
                Item::BehaviorSeed,
                "Assignment seed",
                s.behavior_profiles.seed.to_string(),
                "Seeded mix uses actor ID, kind and this seed; it does not consume the game's RNG. The same stage/layout and seed reproduce the assignment. Savestates retain profiles and individual overrides.",
            ),
            (
                Item::Actor,
                "Selected actor",
                format!(
                    "#{} · {}",
                    d.selected_actor,
                    d.actor_combat.map_or("Inactive / protected", |p| p.name())
                ),
                "Choose a guard for individual overrides, or click an actor in the free camera. Global profiles must be enabled. Overrides last until retry and are included in savestates/rewind.",
            ),
            (
                Item::ActorCombat,
                "Guard combat override",
                d.actor_behavior
                    .map_or("Unavailable", |o| {
                        o.combat.map_or("Follow global", |p| p.name())
                    })
                    .into(),
                "Override only the selected living, non-story guard. Original keeps its authored combat; Follow global removes the override. R resets this guard to Follow global.",
            ),
            (
                Item::ActorIdle,
                "Guard idle override",
                d.actor_behavior
                    .map_or("Unavailable", |o| {
                        o.idle.map_or("Follow global", |p| p.name())
                    })
                    .into(),
                "Override only the selected guard's undetected behavior. Does not replace sleeping or scripted actions. R resets this guard to Follow global.",
            ),
            (
                Item::InspectActor,
                "Inspect selected actor",
                on(d.inspect_actor),
                "Show the selected actor's current state, motion and position.",
            ),
            (
                Item::FocusActor,
                "Focus selected actor",
                "Enter".into(),
                "Move the inspection camera to the selected guard. F2 returns to the game camera.",
            ),
        ],
        20 => vec![
            (
                Item::AnalogWalking,
                "Analog / modifier walking",
                on(s.walking.enabled),
                "GAMEPLAY EDIT: lightly tilt the movement stick to walk with NinjaA/Kunoichi motions; full tilt runs. Hold the rebindable Walk modifier for keyboard or D-pad walking (new bindings: Left Alt / left-stick click). Original weapons and combat remain. Forward movement only in classic controls; camera-relative walking supports any direction. Disables input recording and TRIAL records.",
            ),
            (
                Item::WalkThreshold,
                "Walk / run threshold",
                format!("{}%", s.walking.threshold),
                "Stick travel after the circular deadzone. Below this walks, above it runs, with a small margin to prevent jitter. The Walk modifier always walks. NPC walk speeds: Rikimaru 38, Ayame 31 units/tick; no extra sight or damage bonuses.",
            ),
            (
                Item::Sheathing,
                "Draw / sheathe weapons",
                on(s.sheathing.enabled),
                "GAMEPLAY EDIT: press the rebindable Draw / sheathe action (H by default; assign a controller button under General > Controls). Uses NinjaB/Kunoichi animations; cannot interrupt attacks, injuries or jumps. Grounded attacks draw first; stealth kills and air attacks equip automatically. Off by default; disables input recording and TRIAL records.",
            ),
            (
                Item::UnarmedAttacks,
                "Unarmed attacks while sheathed",
                on(s.sheathing.unarmed),
                "GAMEPLAY EDIT: use the selected grounded unarmed moves while sheathed. Draw explicitly for blades; stealth kills and aerial attacks still draw. Existing armed movesets are retained. Off by default.",
            ),
            (
                Item::UnarmedStyle,
                "Unarmed style",
                s.sheathing.unarmed_style.name().into(),
                "Onikage uses your Gaisen disc. Tatsumaru imports punches and kicks from Tenchu 2 Europe or Japan, with that edition's hit frames and striking limbs adapted to Gaisen combat. Import below first. Both styles work with either hero; finish an unarmed attack before switching styles.",
            ),
            (
                Item::SheathedSpeed,
                "Sheathed movement speed",
                format!("{}%", s.sheathing.speed),
                "Only while weapons are sheathed: scale forward/backward locomotion, including optional walking. 100% keeps original hero speeds; jumps, dashes, attacks and collision rules are unchanged.",
            ),
            (
                Item::SheathedVisibility,
                "Sheathed detection range",
                format!("{}%", s.sheathing.visibility),
                "Only while sheathed: multiply guards' direct player sight range. 100% adds no benefit. Stacks with enemy sight-distance settings; shared alerts, corpse sight and line-of-sight rules are unchanged.",
            ),
            (
                Item::SheathedDamage,
                "Sheathed damage taken",
                format!("{}%", s.sheathing.damage),
                "Only while sheathed: multiply health damage from melee and item/projectile hits. 100% adds no penalty. Does not amplify recoil, falling or ongoing poison; god mode still protects you.",
            ),
            (
                Item::SheathedOneHit,
                "Sheathed one-hit death",
                on(s.sheathing.one_hit),
                "Optional challenge: any damaging melee or item/projectile hit is fatal while sheathed. Replaces the damage multiplier; god mode still protects you. Off by default.",
            ),
            (
                Item::StaySheathed,
                "Stay sheathed after damage",
                on(s.sheathing.stay_sheathed),
                "Keep a stowed weapon sheathed after recovering from a hit or knockdown. Injury animations and damage still apply. Already-drawn weapons stay drawn. Off restores automatic drawing after recovery.",
            ),
            (
                Item::GuardControls,
                "R1 guard / dash / roll",
                on(s.guard_controls),
                "GAMEPLAY EDIT: hold the R1 binding to stand and block frontal melee attacks. Press a direction once to dash; hold crouch for a roll. Directions are relative to the character, including with camera-relative movement. Release and press again to repeat; cannot interrupt attacks, recoil or injuries. Circle still crouches. Rebind R1 under General > Controls (default controller RB, new keyboard bindings K). Off by default; disables input recording and TRIAL records.",
            ),
            (
                Item::GuardDraw,
                "Guard stance draws weapon",
                on(s.guard_draw),
                "With optional R1 guard controls, automatically draw a sheathed weapon. Off keeps it sheathed; blocking still requires a drawn weapon. Dashes and crouching rolls remain available.",
            ),
            (
                Item::CameraRelative,
                "Camera-relative controls",
                on(s.navigation.camera_relative),
                "Camera-relative movement with full left-stick direction. Right stick / held right mouse orbits; click right stick to recenter. Attack + direction selects a slash relative to your current facing, without turning first. Hold Left Ctrl / R1 and back to guard with a drawn weapon; modifier + left/right + attack selects side slashes regardless of camera angle. Shift / Circle still crouches. Recorded input tapes require classic controls.",
            ),
            (
                Item::AlignAim,
                "Start aiming toward camera",
                on(s.navigation.align_aim),
                "With camera-relative controls, turn toward the camera view before entering grapple, item aim or look, and start at its vertical angle. Existing attacks and ongoing aim retain their controls.",
            ),
            (
                Item::MouseSensitivity,
                "Mouse sensitivity",
                format!("{}%", s.navigation.mouse_sensitivity),
                "Camera rotation while holding right mouse. 100% turns roughly 86 degrees per 1000 mouse counts.",
            ),
            (
                Item::StickSensitivity,
                "Right-stick sensitivity",
                format!("{}%", s.navigation.stick_sensitivity),
                "100% turns about 172 degrees per second at full deflection. Small deflections allow precise adjustment.",
            ),
            (
                Item::StickDeadzone,
                "Stick deadzone",
                format!("{}%", s.navigation.deadzone),
                "Circular deadzone for movement and camera sticks. Increase if the controller drifts.",
            ),
            (
                Item::InvertCameraY,
                "Invert camera vertical axis",
                on(s.navigation.invert_y),
                "Invert vertical camera movement for both mouse and right stick.",
            ),
            (
                Item::FollowDistance,
                "Follow distance",
                s.navigation.distance.to_string(),
                "Preferred orbit distance. The camera pulls in around walls and eases out when clear.",
            ),
            (
                Item::RecenterOrbit,
                "Recenter behind player",
                "Recenter".into(),
                "Face the player's direction again. Shortcut: middle-click or click the right stick. Aim, grappling and cinematic cameras retain their original behavior; either stick can aim.",
            ),
        ],
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
                "Modern impact sparks",
                on(s.graphics.effects.sparks),
                "Sharp impact bursts and hot trails for weapon blocks, clashes, metal impacts and grappling. Moving sparks illuminate nearby surfaces when dynamic lighting is on. Cosmetic only.",
            ),
            (
                Item::Gunfire,
                "Modern gunfire",
                on(s.graphics.effects.gunfire),
                "Echigoya's gun: a brief muzzle flash and light, drifting smoke and impact sparks. Shot timing and damage stay unchanged.",
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
                Item::HeroModels,
                "High-detail heroes",
                on(s.graphics.models.heroes),
                "Smooth subdivided hero silhouettes with finger creases, without inflating the body or shortening limbs. Covers Rikimaru and both Ayame outfits; uses installed HD artwork. Original rigs, attachments and gameplay stay unchanged. Custom models are retained.",
            ),
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
                    Item::WaterReflective,
                    "Reflective water",
                    on(water.reflective),
                    "Stronger angle-dependent sky sheen and reflections of local lights. Requires Modern water; does not mirror scene objects. Optional, off in all presets.",
                ),
                (
                    Item::WaterRipples,
                    "Ripple strength",
                    format!("{}%", water.ripples),
                    "Continuous world-space ripples and falling-water filaments; fades distant detail to avoid shimmer.",
                ),
                (
                    Item::WaterReflections,
                    "Reflection strength",
                    format!("{}%", water.reflections),
                    "Sky sheen and sun/moon highlights, plus local lights in Reflective water mode; does not mirror scene objects.",
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
                    "Moving aerated waterfall streaks and spray; flat water has no artificial foam bands.",
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
                    "Original retains the mission weather. Other choices replace presentation only. Rainstorm and Snowstorm add lightning and delayed thunder. Blizzard adds dense, strongly wind-driven snow and howling wind without lightning; gameplay is unchanged.",
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
                    Item::StormLightning,
                    "Storm lightning strength",
                    format!("{}%", weather.lightning),
                    "Cloud flashes and optional visible bolts in Rainstorm/Snowstorm. Shelter blocks their light; 0 disables both without muting thunder.",
                ),
                (
                    Item::StormBolts,
                    "Visible storm bolts",
                    on(weather.storm_bolts),
                    "Distant branching bolts in the sky, occluded by roofs and terrain. Off keeps cloud flashes only; no gameplay damage or effects RNG changes.",
                ),
                (
                    Item::StormInterval,
                    "Storm lightning interval",
                    format!("{} s", weather.lightning_interval),
                    "Average time between storm flashes, with small independent random variation. Thunder follows each flash after a short distance delay.",
                ),
                (
                    Item::WindVolume,
                    "Blizzard wind volume",
                    format!("{}%", weather.wind_volume),
                    "Continuous gusting wind in Blizzard mode, following wind strength and muffling under shelter. No thunder or lightning; choose Snowstorm for those. Follows master/effects volume and pause. 0 disables.",
                ),
                (
                    Item::RainVolume,
                    "Rain ambience volume",
                    format!("{}%", weather.rain_volume),
                    "Continuous stereo rainfall in Rain/Rainstorm, following precipitation intensity and wind-driven shelter at the camera. Smoothly muffles under cover; follows master/effects volume and pause. 0 disables.",
                ),
                (
                    Item::ThunderStyle,
                    "Thunder sound",
                    match weather.thunder_style {
                        tenchu_audio::ThunderStyle::Simple => "Existing",
                        tenchu_audio::ThunderStyle::Rolling => "Rolling",
                        tenchu_audio::ThunderStyle::Retro => "Retro rolling",
                        tenchu_audio::ThunderStyle::Natural => "Natural",
                    }
                    .into(),
                    "Natural adds irregular cracks that recede into low rolling echoes. Existing and Rolling preserve earlier sounds; Retro rolling adds 8-bit/11 kHz coloration. Timing and volume stay separate.",
                ),
                (
                    Item::StormThunder,
                    "Storm thunder volume",
                    format!("{}%", weather.thunder),
                    "Synthesized thunder in Rainstorm/Snowstorm. Also follows the master/effects volume; 0 mutes it. Pauses with the game.",
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
                    "Players and active humanoid enemies leave grounded tracks in snow at any camera distance. Fresh snowfall gradually covers them.",
                ),
                (
                    Item::BakedSnowFootprints,
                    "Tracks on existing snow",
                    on(weather.baked_snow),
                    "Allow footprints on the original snowy terrain in Missions 5 and 10, including TRIAL maps using those materials. Works with Original weather and replacement artwork; bare soil, stone and indoor floors stay clear. Requires Snow footprints. Enabled by modern presets.",
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
                Item::Dismemberment,
                "Stealth Assassins dismemberment",
                on(s.dismemberment.enabled),
                "Optional USA Rev 1 rules using Gaisen models: rear stealth kills / dive attacks can remove heads, ordinary deaths can remove a forearm. Excludes players, animals and bosses (except Kataoka's execution). Changes gameplay RNG; off by default.",
            ),
            (
                Item::SeverStealth,
                "Rear stealth kill: head",
                format!("{}%", s.dismemberment.stealth_head),
                "Chance at the original SA animation frame. Default 4%; other stealth-kill directions never sever limbs.",
            ),
            (
                Item::SeverDive,
                "Dive attack kill: head",
                format!("{}%", s.dismemberment.dive_head),
                "Default 4%. A failed head roll can still sever an arm.",
            ),
            (
                Item::SeverArms,
                "Ordinary kill: forearm",
                format!("{}%", s.dismemberment.arms),
                "Default 50%, after any unsuccessful dive-attack head roll. No arm roll during stealth kills.",
            ),
            (
                Item::SeverSide,
                "Right arm preference",
                format!("{}%", s.dismemberment.right_arm),
                "Conditional on severing an arm: default 50% chooses either side equally; 0 always left, 100 always right.",
            ),
            (
                Item::PlayerDismemberment,
                "Player dismemberment",
                on(s.dismemberment.player),
                "Allow the player to lose a head or forearm on a combat death. Separate from enemy SA rules; off by default. Does not change damage or cause death early.",
            ),
            (
                Item::PlayerSeverHead,
                "Player death: head",
                format!("{}%", s.dismemberment.player_head),
                "Chance on combat death, before the arm roll. Defaults to 4%; this is a port extension, not a retail SA rule.",
            ),
            (
                Item::PlayerSeverArms,
                "Player death: forearm",
                format!("{}%", s.dismemberment.player_arms),
                "Chance after a failed head roll. Default 50%; 100 always removes an arm when the head stays attached.",
            ),
            (
                Item::PlayerSeverSide,
                "Player right arm preference",
                format!("{}%", s.dismemberment.player_right_arm),
                "Default 50% chooses either arm equally; 0 always left, 100 always right.",
            ),
            (
                Item::HeadRest,
                "Severed head resting pose",
                s.dismemberment.head_rest.name().into(),
                "After landing: face up, either side, a mixture of those three, or the original tumble orientation. Both player and enemies; leaves severing chances and gameplay RNG alone. Applies to the next landing.",
            ),
            (
                Item::SaExecutionScene,
                "SA Kataoka execution scene",
                on(s.dismemberment.execution_scene),
                "Restore Stealth Assassins' Rikimaru/Kataoka execution camera sequence in Mission 5, including the severed head. Keeps Gaisen's localized dialogue and audio; no additional disc required. Enable before the scene starts; off by default. Independent of random enemy/player severing chances.",
            ),
            (
                Item::SeveredStyle,
                "Severed blood style",
                s.graphics.blood.severed_style.label().into(),
                "Original disc droplets, smoothed Original HD, or modern jets from detached parts and their body wounds. Off hides both. Independent of wound drips, hit spurts and ground blood.",
            ),
            (
                Item::SeveredBlood,
                "Severed wound bleeding",
                format!("{}%", s.graphics.blood.severed),
                "0 disables. Detached heads/arms drip and their wounds on the body spurt, with weakening pressure. Cosmetic only. Ground marks use Ground blood spots and its retention setting.",
            ),
            (
                Item::SeveredSeconds,
                "Severed wound duration",
                format!("{} s", s.graphics.blood.severed_seconds),
                "How long the detached part and body wound bleed after severing. Default 3 seconds; 0 stops extra bleeding.",
            ),
            (
                Item::BloodHit,
                "Blood spurts on hit",
                s.graphics.blood.hit.label().into(),
                "Off, Original, smoother Original HD, or soft Modern droplets. Original HD keeps the retail spatter shapes, colors and timing; gameplay stays unchanged.",
            ),
            (
                Item::BloodDrips,
                "Wound drips",
                s.graphics.blood.drips.label().into(),
                "Extra bleeding after damage: Off, original square particles, rounded Original HD drops, or soft Modern droplets. Duration is adjustable below.",
            ),
            (
                Item::BloodDeath,
                "Death pool / star",
                s.graphics.blood.death.label().into(),
                "Off, Original, Original HD with smoother retail patterns, or a Modern pool. Appears after death and landing. Changing style clears existing death pools only.",
            ),
            (
                Item::BloodPlayerDeath,
                "Death pool beneath player",
                on(s.graphics.blood.player_death),
                "Apply the selected death pool style to the player too. Retail only has enemy pools.",
            ),
            (
                Item::BloodDeathSize,
                "Death pool size",
                format!("{}%", s.graphics.blood.death_size),
                "Scale original death stars and modern pools beneath players and enemies, independently of wound drips and ground spots. Existing cosmetic pools regrow at the new size.",
            ),
            (
                Item::BloodGround,
                "Ground blood spots",
                s.graphics.blood.ground.label().into(),
                "Off, Original, smoother Original HD artwork, or Modern surface stains. Retain either original style below; Modern spots persist. Changing style clears existing spots only.",
            ),
            (
                Item::BloodRetain,
                "Retain original blood",
                on(s.graphics.blood.retain_original),
                "Extend the lifetime of Original / Original HD ground splats, keeping their chosen artwork. Also retains original death stars. Ground style is independent of hit spurts; the shared blood limit still applies.",
            ),
            (
                Item::BloodConform,
                "Retained original blood follows surfaces",
                on(s.graphics.blood.conform_retained),
                "Lay retained Original / Original HD splats onto the floor and slopes instead of leaving camera-facing sprites floating in space. Preserves pixel artwork and size. Off keeps retail sprite placement. Changing clears existing ground splats.",
            ),
            (
                Item::BloodModels,
                "Stain wounded models",
                on(s.graphics.blood.model_stains),
                "Retain soft wound stains on bodies independently of spray, drips and ground spots.",
            ),
            (
                Item::BloodWeapons,
                "Blood on weapons",
                on(s.graphics.blood.weapons),
                "Successful melee hits and stealth kills stain metal on player and enemy weapons. Small drips gradually remove wet blood; swinging sheds more. Dry traces remain until Clear blood or mission change. Cosmetic only.",
            ),
            (
                Item::WeaponBloodMetalOnly,
                "Weapon blood on metal only",
                on(s.graphics.blood.weapon_metal_only),
                "Restrict coating to silver-colored faces when available. Off stains the whole striking weapon, including colored blades and handles. Scabbards never receive weapon-hit blood.",
            ),
            (
                Item::WeaponBloodCoverage,
                "Weapon blood coverage",
                format!("{}%", s.graphics.blood.weapon_coverage),
                "Make coatings more visible without changing body/ground blood. 100% keeps the previous coverage. Requires Blood on weapons; 0 hides the coating.",
            ),
            (
                Item::WeaponBloodFlow,
                "Wet weapon blood flow",
                format!("{}%", s.graphics.blood.weapon_flow),
                "Gravity moves wet streaks across the blade and droplets fall from its lowest edge. Higher values flow and drip faster. Swings still shed blood; drying stops the flow. 0 keeps the previous static coating and tip drips.",
            ),
            (
                Item::WeaponReflections,
                "Metal weapon reflections",
                format!("{}%", s.graphics.weapon_reflections),
                "0 disables. Metal reflects the changing sky and nearby lights as the camera/blade moves, with shadow-aware highlights. Wood and cloth stay matte. This does not render mirror images of nearby objects.",
            ),
            (
                Item::BloodAmount,
                "Blood amount",
                format!("{}%", s.graphics.blood.amount),
                "Size of persistent stains and frequency of wound drips; does not affect damage or the original effect lifetime.",
            ),
            (
                Item::BloodRunoff,
                "Blood runs down surfaces",
                on(s.graphics.blood.runoff),
                "Let retained blood trickle down walls and slopes. Original and Original HD need Retain original blood enabled. Turning off stops new trails without clearing stains.",
            ),
            (
                Item::BloodSeconds,
                "Wound bleeding",
                format!("{} s", s.graphics.blood.seconds),
                "How long each wound drips or spurts (default 3 seconds). Changes also affect active wounds; 0 stops extra bleeding. Retained stains and death pools remain.",
            ),
            (
                Item::BloodSpurts,
                "Wound spurt strength",
                format!("{}%", s.graphics.blood.spurts),
                "0 keeps drips only. Add pulsing jets from animated wounds using the selected Wound drips style. Strength and frequency weaken over the Wound bleeding duration. Cosmetic only; no health loss.",
            ),
            (
                Item::GushStyle,
                "Death gush style",
                s.graphics.blood.gush_style.label().into(),
                "Choose Original, Original HD or Modern for the optional upward death bursts. Off disables the extra gush without changing the death pool or ground spots.",
            ),
            (
                Item::DeathGush,
                "Death gush strength",
                format!("{}%", s.graphics.blood.death_gush),
                "Optional Onikage-inspired upward bursts using Death gush style. Starts with the original death blood and follows the body. Death pools must not be Off; Player death pool also controls the player. Cinematics and gameplay stay unchanged.",
            ),
            (
                Item::GushSeconds,
                "Death gush duration",
                format!("{} s", s.graphics.blood.gush_seconds),
                "How long a death gush lasts, with weakening pressure. Independent of wound bleeding; 0 stops it. Clear blood also stops active gushes.",
            ),
            (
                Item::BloodMarks,
                "Retained stains",
                s.graphics.blood.marks.to_string(),
                "Retain up to 65,535 stains. Oldest marks are removed at this limit; a 256 MiB geometry budget also bounds memory. Ground stains are cached on the GPU.",
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
                Item::AlternateDeathChance,
                "Alternate player deaths",
                format!("{}%", s.alternate_death_chance),
                "Chance of a different collapse on your next ordinary death. Both heroes share all four player/ninja death motions, without gender restrictions. Blood follows the chosen pose. Try 50% for variety; 0% keeps retail. Visual only: failure timing, retry and RNG are unchanged. Special deaths and model movesets keep their own animations.",
            ),
            (
                Item::PlayerModel,
                "Player body",
                s.player_model.clone(),
                "Appearance only; weapons, animation and gameplay remain unchanged.",
            ),
            (
                Item::ModelMoveset,
                "Override moveset",
                on(s.model_moveset),
                "Use the selected moveset's animations, attacks and weapons, independently of your body. Missing actions fall back to your protagonist. Turn off to restore player moves.",
            ),
            (
                Item::MovesetModel,
                "Moveset source",
                s.moveset_model.clone(),
                "Choose an available actor's moves while keeping your current body. Imported T2 Rikimaru, Ayame and Tatsumaru include their locomotion and melee combos; paired kills, items and unmapped actions use Gaisen motions. Other T2 actors and custom bodies currently retain player moves.",
            ),
            (
                Item::ModelPlayerMovement,
                "Use player movement",
                on(s.model_moveset_options.player_movement),
                "With a moveset override enabled, use Ayame's or Rikimaru's walking, running, dashes, crouching, jumping and climbing motions and speeds. Keep the selected actor's attacks.",
            ),
            (
                Item::ModelNoIdleTaunts,
                "Disable idle taunts",
                on(s.model_moveset_options.no_idle_taunts),
                "With a moveset override enabled, prevent the player's random idle fidgets and combat taunts from locking movement. Enemies and scripted actions are unchanged.",
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
                "Sharp HUD, subtitles, menu labels and clearer disc-derived briefing calligraphy.",
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
                Item::CircleConfirm,
                "Circle confirms in menus",
                on(s.circle_confirm),
                "Japanese: East / Circle confirms, South / Cross goes back. Off: Western (South confirms). Gameplay bindings and Enter/Esc stay unchanged. Menu buttons use their physical positions, independently of gameplay remapping.",
            ),
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
                Item::MusicWhilePaused,
                "Music while paused",
                on(s.music_while_paused),
                "Keep in-mission music playing during F3/F4, settings-menu and defocus pauses. Gameplay sounds stay paused; movies and narration still pause normally.",
            ),
            (
                Item::WaitAfterDeath,
                "Wait after death",
                on(s.wait_after_death),
                "Hold the completed death scene until Enter, the menu confirm button or Start. Off: automatically show mission failure, as in retail.",
            ),
            (
                Item::LiveDeathScene,
                "Keep world running after death",
                on(s.live_death_scene),
                "Guards return to patrol and effects keep running after the campaign death animation. Also enables Wait after death. Off: freeze the final scene. Waiting longer does not change your retry seed.",
            ),
            (
                Item::ShopLighting,
                "Shop window and lighting",
                on(s.graphics.shop_lighting),
                "Show the configured sky, wind-driven rain/snow and the mission's original snowfall outside the window. Moving window-grid light and soft shadows relight the painted room. Off keeps the painting unchanged.",
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
                Item::BurningSpatial,
                "Local burning-body sound",
                on(s.burning_sound.spatial),
                "Fade burning-body crackles with distance. Off restores the previous map-wide playback; lightning item casting remains global.",
            ),
            (
                Item::BurningVolume,
                "Burning-body volume",
                format!("{}%", s.burning_sound.volume),
                "Volume of crackles from burning actors, in addition to master/effects volume. 0 mutes them.",
            ),
            (
                Item::BurningRange,
                "Burning-body hearing range",
                s.burning_sound.range.to_string(),
                "World units; local burning sounds fade smoothly to silence at this distance (6000 by default).",
            ),
            (
                Item::EnglishShuriken,
                "English item name: Shuriken",
                on(s.english_shuriken),
                "Use Shuriken in English shop help to match the throwing-star icon. Off uses the disc's Crimson Blade title. Works with original and replacement artwork.",
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
                Item::DistantDoors,
                "Distant doors without fog",
                on(s.graphics.distant_doors),
                "Show doors and trapdoors beyond retail range when fog is disabled or extended. Collision and activation stay unchanged. Off restores retail drawing.",
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
                if cfg!(target_os = "android") {
                    "Move/Camera sticks fly/look. Rise, Descend and Boost change movement. Back returns to the player."
                } else {
                    "WASD/QE fly, RMB look; no player input."
                },
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
                "Move to the selected actor without following.",
            ),
            (
                Item::Actor,
                "Selected actor",
                format!("#{}", d.selected_actor),
                "Choose the actor to focus or follow; changes the subject while following.",
            ),
            (
                Item::FollowActor,
                "Follow selected actor",
                on(d.camera_actor.is_some()),
                "Orbit the selected actor with RMB/right stick, including after death. Player input stays enabled; F2 returns to the game camera.",
            ),
            (
                Item::FollowDistance,
                "Follow distance",
                s.navigation.distance.to_string(),
                "Orbit distance; walls shorten it. Sensitivity is shared with Movement settings.",
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
                "Hold Backspace for Max during cutscenes. Accelerated playback mutes audio, then resumes it at the matching position. Gameplay and menus stay at normal speed.",
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
                "GAMEPLAY EDIT: distant enemies keep thinking and patrolling. Each placed guard can call once per alert; recruits cannot call. Reinforcements need terrain cover. Ends recording.",
            ),
            (
                Item::IndependentAttacks,
                "Independent enemy attacks",
                on(s.independent_attacks),
                "GAMEPLAY EDIT: each enemy uses its own attack cooldown instead of sharing a timer. More enemies can attack together; individual attack speed is unchanged. Ends recording.",
            ),
            (
                Item::MultiTargetAttacks,
                "Multi-target player attacks",
                on(s.multi_target_attacks),
                "New melee swings can hit several targets, once per target with each weapon hitbox. Keeps normal damage, blocking, weapon clashes and terrain checks. Items and enemy attacks are unchanged. Gameplay option; disables input recording and TRIAL records.",
            ),
            (
                Item::AttackAssist,
                "Forward melee assist",
                on(s.attack_assist),
                "Prefer a nearby enemy in front with a clear terrain path, then retain it through a combo. Turns smoothly during the original wind-up only; no extra reach, damage or hit time. Stealth kills, ranged attacks and backward attacks stay original. Gameplay option; off by default.",
            ),
            (
                Item::StrictCorpseSight,
                "Strict corpse visibility",
                on(s.strict_corpse_sight),
                "Require clear terrain sight from a guard's eyes to the body's animated position before discovering it. Prevents the original feet-to-air ray from seeing bodies behind cover. Visible bodies still raise shared alerts. Gameplay option; off by default.",
            ),
            (
                Item::QuickShurikenDodge,
                "Quick shuriken evasion",
                on(s.quick_shuriken_dodge),
                "Alert armed enemies immediately sidestep when you aim shuriken at them, at most once a second. Requires clear sight and their own dodge motion; attacks, injuries and scripted scenes are not interrupted. Changes gameplay; off by default.",
            ),
            (
                Item::DetectionDamage,
                "Damage per repeat detection",
                if s.damage_escalation.increment == 0 {
                    "Off".into()
                } else {
                    format!("+{}%", s.damage_escalation.increment)
                },
                "GAMEPLAY EDIT: first sighting does normal damage. Each time this guard sees you again after fully calming down, increase its damage to you. Shared alarms and brief sight loss do not count. Turning off or starting a mission resets encounters. Disables input recording and TRIAL records.",
            ),
            (
                Item::DetectionDamageLimit,
                "Maximum detection damage bonus",
                format!("+{}%", s.damage_escalation.maximum),
                "Cap the extra health damage from repeat detections. +100% doubles damage; 0 disables escalation. Attack speed, knockback and hit reactions stay unchanged.",
            ),
            (
                Item::ReinforcementsPerCall,
                "Guards per reinforcement call",
                s.reinforcement_call.count.to_string(),
                "All-active mode only: each placed guard can call this many guards once per alert. Resets after returning to calm with no alert. Recruits never call. 0 disables extra guards; civilian replacements still work. Ends recording.",
            ),
            (
                Item::RandomReinforcements,
                "Random reinforcement count",
                on(s.reinforcement_call.random),
                "All-active mode only: randomly choose 1 through Guards per reinforcement call, instead of a fixed count. 0 still disables extra guards. Ends recording.",
            ),
            (
                Item::WitnessedKillReinforcements,
                "Call aid after witnessed kills",
                on(s.reinforcement_call.witnessed_kills),
                "All-active mode only: the nearest guard who sees a non-stealth guard kill retreats to seek reinforcements. Requires range, facing and terrain line of sight; respects call counts and population limits. Recruits never call. Off by default.",
            ),
            (
                Item::ReinforcementLimit,
                "Reinforcement population limit",
                if s.reinforcement_limit == 0 {
                    "Unlimited".into()
                } else {
                    s.reinforcement_limit.to_string()
                },
                "All-active mode only: stop calling extra guards at this total actor count, including the player and corpses. 0 = Unlimited. Existing actors and civilian replacements are kept. Ends recording.",
            ),
            (
                Item::RewindHistory,
                "Rewind history (seconds)",
                s.rewind_seconds.to_string(),
                "0 disables capture. Keep every game update for up to 30 seconds; 96 MiB maximum. Heavy effects can shorten the history. Cleared on retry, mission change or loading a file.",
            ),
            (
                Item::RewindSpeed,
                "Rewind speed (%)",
                s.rewind_speed.to_string(),
                "Backward playback speed: 100 = real time, 200 = twice as fast. No forward simulation runs during rewind. Audio resumes from the restored point.",
            ),
            (
                Item::Rewind,
                "Rewind / Ctrl+F12",
                "Rewind".into(),
                "Play backward, then use the timeline to seek either way. Ctrl+F12 stops; F3 resumes here and discards only the recorded future. Restores gameplay and effects; ends input recording and disables TRIAL records.",
            ),
            (
                Item::StateSlot,
                "Savestate slot",
                (d.state_slot + 1).to_string(),
                "Five separate mid-mission slots. F11 saves; F12 loads. Same game version required.",
            ),
            (
                Item::SaveState,
                "Save state / F11",
                "Save".into(),
                "Replace the selected slot. Includes RNG, enemies, items, effects and blood; does not change campaign progress saves.",
            ),
            (
                Item::LoadState,
                "Load state / F12",
                "Load".into(),
                "Restores gameplay and pauses. F3 resumes. Ends input recording; disables TRIAL records. Keeps current graphics preferences.",
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
                "Restore original speed, enemy sight, activation and attack coordination.",
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
                    "Fires, burning characters, grenades, embers and kaen flames illuminate nearby surfaces. Transient lights share the light and shadow budgets.",
                ),
                (
                    Item::LightCount,
                    "Lights per region",
                    light.count.to_string(),
                    "0-16 overlapping lights per world region. Distant visible lamps stay lit independently of player/camera distance. Shadows have a separate budget and cost extra.",
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
                    "0-8 nearby shadow casters with gradual handovers; six views per source. Other lamps remain lit without dynamic shadows. High/Ultra cover more shadow casters before you approach.",
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
                (
                    Item::FlameMotion,
                    "Flame light / shadow motion",
                    format!("{}%", light.flame_motion),
                    "Gentle flame flicker and moving shadow edges. Reuses cached shadow maps; does not move lights or affect gameplay. 0: steady lights.",
                ),
                (
                    Item::HeatShimmer,
                    "Flame heat shimmer",
                    format!("{}%", light.heat_shimmer),
                    "Local heat distortion above flames and lanterns. Requires dynamic lighting and HDR image finishing. Foreground objects and the interface stay clear. 0: off.",
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
            Action::Right | Action::Confirm | Action::Start => return Some(1),
            Action::Replay => {}
        }
        if matches!(action, Action::Up | Action::Down) {
            self.reveal_row = true;
        }
        None
    }
}
