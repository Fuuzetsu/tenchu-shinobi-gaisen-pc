use super::{menu::Item, overlay::Edit};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Gaisen,
    Tenchu2,
}

impl Runtime {
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
            Fps | Focus | MenuPause | MusicWhilePaused | Volume | Fullscreen | Vsync | Hud
            | Preset | Retro | Fog | FogDistance | Textures | Shading | Exposure | Filtering
            | Interpolation | Magnification | SurfaceDetail | SurfaceStrength | Antialiasing
            | AssetPack | Upscale | Post | Filmic | Bloom | BloomThreshold | Occlusion
            | OcclusionRadius | Contrast | Saturation | Temperature | Gamma | Vignette
            | Sharpen | CutsceneSpeed | Pause | Step | Speed | FreeCamera | FlightSpeed | Fov
            | FocusPlayer | FocusActor | Actor | Overhead | SaveCamera | RestoreCamera
            | PhotoMode | Hitboxes | Labels | InspectActor | Xray | Sight | SightStance
            | Patrols | Events | Collision | CollisionOnly | Target(_) | PlayerPosition
            | CameraPosition | SnapFloor | Teleport | PickTeleport | ClickTeleport => None,
            FollowActor | FollowDistance | RecenterOrbit | MouseSensitivity | StickSensitivity
            | StickDeadzone | InvertCameraY => None,
            GodMode | Armor | GiveItems | ClearItems | Inventory(_) => None,
            Lighting | Shadows | FireLights | LightRadius | LightIntensity | LightAmbient
            | LightResolution | LightCount | ShadowCount | ShadowSoftness | LightSourceSize => None,
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
            LampLights => Some(
                "Native Tenchu 2 currently supplies flame sources. Texture-based lamp discovery still needs its model metadata adapter.",
            ),
            Weather | WeatherAmount | WeatherWind | WeatherHeading | WeatherQuality
            | WeatherDistance | Wetness | RainReflections | PuddleCoverage | PoolingSpeed
            | RainHaze | RainStreaks | SnowCover | SnowDrift | Footprints | FootprintSeconds
            | ClearFootprints | RoofRunoff | RunoffAmount | ClearWeather | StormLightning
            | StormBolts | StormInterval | StormThunder | RainVolume | WindVolume
            | ThunderStyle => None,
            PlayerModel | ModelMoveset | MovesetModel | ModelPlayerMovement | ModelNoIdleTaunts => {
                None
            }
            ReloadModels | ReloadAssets => None,
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
            Edit::Diagnostics => Some(
                "Report export is not connected to native Tenchu 2 JP yet. Use F1 → Tenchu 2 → Integration diagnostics to inspect the current state.",
            ),
            Edit::ImportUnarmed | Edit::ExportUnarmed => Some(
                "Imported Gaisen unarmed moves are not used by native Tenchu 2 JP. Its characters already use their own source movesets.",
            ),
            _ => self.unavailable(Item::Reset),
        }
    }
}
