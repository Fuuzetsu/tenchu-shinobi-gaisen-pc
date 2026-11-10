use super::{menu::Item, overlay::Edit};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Gaisen,
    Tenchu2,
}

impl Runtime {
    pub fn help(self, item: Item, fallback: &'static str) -> &'static str {
        if self == Self::Tenchu2 && item == Item::CameraRelative {
            "Camera-relative movement with full left-stick direction. Right stick / held right mouse orbits; click right stick to recenter. Attack + direction keeps the current facing for directional attacks. T2 bindings still apply: Circle guards, R1 crouches. Ledges, walls, underwater swimming and item aiming keep their original controls. Off restores classic controls."
        } else if self == Self::Tenchu2 && item == Item::BloodPlayerDeath {
            "Add a pool beneath the player's finished, grounded corpse when the native death has no pool. Uses the death style, size and retention settings. Existing source pools still follow Death pool / star; this option does not duplicate them."
        } else if self == Self::Tenchu2 && item == Item::DistantDoors {
            "Draw doors and collapsing platforms outside the source camera's range when fog is disabled or extended, or while using a custom camera. Current positions and destroyed objects are respected. Collision, triggers and activation stay unchanged. Off restores source drawing."
        } else if self == Self::Tenchu2 && item == Item::SightDistance {
            "GAMEPLAY EDIT: scale native notice, recognition, forgetting and corpse-detection distances, including guards approaching bait. Sight angles, terrain checks and activation remain original. 100% restores source ranges; resets on a new area."
        } else if self == Self::Tenchu2 && item == Item::Stealth {
            "Guards ignore the player and corpses and return to their original idle/patrol behavior. Already committed attacks and projectiles can still hurt; use God mode for protection. Scripted sequences and fights against other actors remain active. Resets on a new area."
        } else if self == Self::Tenchu2 && item == Item::KeepEnemiesActive {
            "GAMEPLAY EDIT: keep all placed native actors simulating, patrolling and animating regardless of distance or the original activation budget. Scripted holds and deaths remain intact. Changes RNG and encounters; can cost more CPU. Off restores source distance/budget activation at its next check."
        } else {
            fallback
        }
    }

    pub fn unavailable(self, item: Item) -> Option<&'static str> {
        use Item::*;
        if matches!(item, Tenchu2Missions | Tenchu2Diagnostics | Tenchu2Launcher) {
            return (self != Self::Tenchu2).then_some(
                "Available during native Tenchu 2 Japan play. Use F6 to select an imported JP mission first.",
            );
        }
        if self == Self::Gaisen {
            return None;
        }
        match item {
            Fullscreen if cfg!(target_os = "android") => {
                Some("Android manages the game window; fullscreen is a desktop setting.")
            }
            Fps | Performance | Focus | MenuPause | MusicWhilePaused | WaitAfterDeath
            | LiveDeathScene | Volume | Fullscreen | Vsync | Hud | Preset | Retro | Fog
            | FogDistance | Textures | Shading | Exposure | Filtering | Interpolation
            | Magnification | SurfaceDetail | SurfaceStrength | Antialiasing | AssetPack
            | Upscale | Post | Filmic | Bloom | BloomThreshold | Occlusion | OcclusionRadius
            | Contrast | Saturation | Temperature | Gamma | Vignette | Sharpen | CutsceneSpeed
            | Pause | Step | Speed | FreeCamera | FlightSpeed | Fov | FocusPlayer | FocusActor
            | Actor | Overhead | SaveCamera | RestoreCamera | PhotoMode | Hitboxes | Labels
            | InspectActor | Xray | Trail | Sight | SightStance | Patrols | Events | Collision
            | CollisionOnly | Target(_) | PlayerPosition | CameraPosition | SnapFloor
            | Teleport | PickTeleport | ClickTeleport => None,
            CameraRelative | AlignAim | FollowActor | FollowDistance | RecenterOrbit
            | MouseSensitivity | StickSensitivity | StickDeadzone | InvertCameraY => None,
            GodMode | Armor | SightDistance | Stealth | KeepEnemiesActive | GiveItems
            | ClearItems | Inventory(_) => None,
            Lighting | Shadows | FireLights | LightRadius | LightIntensity | LightAmbient
            | LightResolution | LightCount | ShadowCount | ShadowSoftness | LightSourceSize => None,
            LightMarkers | FocusLight | PlaceLight | PlayerLight | FollowLight | DebugLight
            | LightAxis(_) | ShadowPreview | FlameMotion | HeatShimmer => None,
            DirectionalIntensity
            | DirectionalColor
            | DirectionalAzimuth
            | DirectionalElevation
            | DirectionalRadius
            | DirectionalResolution
            | SkyTint
            | TimeClock
            | TimeOfDay
            | TimeSpeed
            | SunStrength
            | MoonStrength
            | DayAmbient
            | NightAmbient
            | SunPath
            | DayRelighting => None,
            LampLights | DistantDoors => None,
            Weather | WeatherAmount | WeatherWind | WeatherHeading | WeatherQuality
            | WeatherDistance | Wetness | RainReflections | PuddleCoverage | PoolingSpeed
            | RainHaze | RainStreaks | SnowCover | SnowDrift | Footprints | FootprintSeconds
            | ClearFootprints | RoofRunoff | RunoffAmount | ClearWeather | StormLightning
            | StormBolts | StormInterval | StormThunder | RainVolume | WindVolume
            | ThunderStyle => None,
            ContactSparks | SparkDensity | ContactSplashes | GrappleDetail | BloodHit => None,
            GushStyle | DeathGush | GushSeconds => None,
            BloodDrips | BloodSeconds | BloodSpurts | BloodWeapons | WeaponBloodMetalOnly
            | WeaponBloodCoverage | WeaponBloodFlow => None,
            BloodGround | BloodDeath | BloodPlayerDeath | BloodDeathSize | BloodRetain
            | BloodModels | BloodConform | BloodAmount | BloodMarks | BloodRunoff | ClearBlood => {
                None
            }
            Water | WaterReflective | WaterRipples | WaterReflections | WaterFlow | WaterFoam
            | WaterSpray | WaterDetail => None,
            PlayerModel | ModelMoveset | MovesetModel | ModelPlayerMovement | ModelNoIdleTaunts => {
                None
            }
            ReloadModels | ReloadAssets | SmoothModels | ModelDetail | CharacterRefinement
            | WeaponReflections => None,
            HairMotion | ClothMotion | EquipmentMotion | MotionStrength | MotionResponse
            | MotionWind => None,
            Rewind | RewindHistory | RewindSpeed | SaveState | LoadState | StateSlot => Some(
                "Native Tenchu 2 JP does not yet support mid-mission saves or rewind. Campaign progress still saves normally.",
            ),
            _ => Some(
                "Not implemented in native Tenchu 2 JP yet. Available in Gaisen; saved preferences are preserved.",
            ),
        }
    }

    pub fn unavailable_edit(self, edit: &Edit) -> Option<&'static str> {
        match edit {
            Edit::Change(item, _) | Edit::Reset(item) => self.unavailable(*item),
            _ if self == Self::Gaisen => None,
            Edit::Touch(_) | Edit::Preset(_) | Edit::PresetLibrary | Edit::Controls => None,
            Edit::PlayerModel(_) | Edit::MovesetModel(_) => self.unavailable(Item::PlayerModel),
            Edit::Coordinate(axis, _) => self.unavailable(Item::Target(*axis)),
            Edit::Benchmark => {
                Some("Performance benchmarks are not connected to native Tenchu 2 JP yet.")
            }
            Edit::Diagnostics => None,
            Edit::ImportUnarmed | Edit::ExportUnarmed => Some(
                "Imported Gaisen unarmed moves are not used by native Tenchu 2 JP. Its characters already use their own source movesets.",
            ),
            _ => self.unavailable(Item::Reset),
        }
    }
}
