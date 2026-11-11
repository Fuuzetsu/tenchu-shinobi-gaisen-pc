use super::{menu::Item, overlay::Edit};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    Gaisen,
    Tenchu2,
}

impl Runtime {
    pub fn help(self, item: Item, fallback: &'static str) -> &'static str {
        if self == Self::Tenchu2
            && item == Item::VibrationAction(tenchu_core::effects::RumbleAction::HeavyLanding)
        {
            "Native T2's landing pulse after an aerial attack. Ordinary jumps and flips do not request vibration."
        } else if self == Self::Tenchu2
            && item == Item::VibrationAction(tenchu_core::effects::RumbleAction::Knockdown)
        {
            "Native T2's knockback, ground impact and damaging-fall pulses."
        } else if self == Self::Tenchu2
            && item == Item::VibrationAction(tenchu_core::effects::RumbleAction::TargetHit)
        {
            "Native T2's pulses when hitting animals or breakable scenery."
        } else if self == Self::Tenchu2 && item == Item::CircleConfirm {
            "Circle / East confirms in F1, mission selection, results and retry/death prompts; Cross / South goes back. Off reverses them. Equipment uses confirm to add and back to remove; Start/Enter begins. Physical menu buttons ignore gameplay remapping. Enter/Esc keep their keyboard actions; gameplay and cutscene skip buttons retain their source controls."
        } else if self == Self::Tenchu2 && item == Item::CircleAction {
            "Default keeps native T2 controls for every model and moveset: Circle holds the guard stance, directions dodge, and R1 crouches. Blocking needs a drawn weapon and a frontal attack. Crouch explicitly swaps Circle and R1. Rebind physical buttons under General > Controls."
        } else if self == Self::Tenchu2 && item == Item::AnalogWalking {
            "GAMEPLAY EDIT: lightly tilt the movement stick or hold the rebindable Walk modifier to use T2's original ninja/kunoichi walking cycle. Full tilt runs. Classic controls walk forward; camera-relative controls walk in any direction. Weapons, attacks, crouching, swimming and collision keep their native rules. Off restores the selected moveset's running clips."
        } else if self == Self::Tenchu2 && item == Item::WalkThreshold {
            "Stick travel after the circular deadzone. Below this walks, above it runs, with a small margin to prevent jitter. The Walk modifier always walks. Original T2 ninja/kunoichi speeds are 38/31 units per tick. No extra sight or damage bonus."
        } else if self == Self::Tenchu2 && item == Item::Sheathing {
            "GAMEPLAY EDIT: press Draw / sheathe (H by default; rebind under General > Controls) to play the active moveset's native draw/stow transition and weapon sounds. Available in grounded free stances; cannot cancel attacks, injuries, jumps or scripts. Enables the sheathed speed, detection and damage options. Off removes these edits and leaves T2's current weapon state and original controls intact."
        } else if self == Self::Tenchu2
            && matches!(
                item,
                Item::SheathedSpeed
                    | Item::SheathedVisibility
                    | Item::SheathedDamage
                    | Item::SheathedOneHit
            )
        {
            match item {
                Item::SheathedSpeed => {
                    "With Draw / sheathe enabled and the weapon stowed, scale forward/backward running and optional walking. 100% keeps source speed. Turning, crouching, dodges, jumps, attacks and collision rules are unchanged."
                }
                Item::SheathedVisibility => {
                    "With Draw / sheathe enabled and the weapon stowed, scale direct player sight distances, including guards approaching bait. Stacks with enemy sight-distance settings. Shared alerts, corpse sight and line-of-sight rules are unchanged."
                }
                Item::SheathedDamage => {
                    "With Draw / sheathe enabled and the weapon stowed, multiply health damage from melee and item/projectile hits. 100% keeps source damage. Recoil, falling and ongoing poison retain their original rules; God mode still protects you."
                }
                _ => {
                    "With Draw / sheathe enabled and the weapon stowed, any damaging melee or item/projectile hit is fatal. Replaces the damage multiplier; God mode and resurrection items retain their original rules."
                }
            }
        } else if self == Self::Tenchu2 && item == Item::Dismemberment {
            "GAMEPLAY EDIT: adapts the optional Stealth Assassins head/forearm rules to T2's skeleton and terrain. Rear stealth kills roll at their first authored blood event; combat deaths roll during recovery. Excludes bosses and animals. Changes gameplay RNG; off by default."
        } else if self == Self::Tenchu2 && item == Item::SeverStealth {
            "Chance at the first source blood event in a rear stealth kill. Default 4%; other stealth-kill directions do not sever limbs."
        } else if self == Self::Tenchu2 && item == Item::SaExecutionScene {
            "Only applies to Rikimaru's Kataoka execution in imported T1 Mission 5. Uses the existing SA camera/head sequence with Gaisen dialogue and audio. Enable before the scene starts; native T2 campaign scenes are unchanged."
        } else if self == Self::Tenchu2 && item == Item::UnarmedAttacks {
            "GAMEPLAY EDIT: with Draw / sheathe enabled and weapons stowed, use the selected grounded punches/kicks and combos. Draw explicitly to use your armed moveset. Existing weapons, voices and source stealth kills are retained. Disabling lets the active attack finish. Off by default."
        } else if self == Self::Tenchu2 && item == Item::UnarmedStyle {
            "Tatsumaru uses the current Japanese T2 disc, including its original hit frames and striking limbs. Onikage uses your connected Gaisen disc. The style changes after the active attack finishes; your armed moveset is retained."
        } else if self == Self::Tenchu2 && item == Item::CameraRelative {
            "Camera-relative movement with full left-stick direction. Right stick / held right mouse orbits; click right stick to recenter. Attack + direction keeps the current facing for directional attacks. By default Circle guards and R1 crouches; Shift / Circle action can change this. Ledges, walls, underwater swimming and item aiming keep their original controls. Off restores classic controls."
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
        } else if self == Self::Tenchu2 && item == Item::QuickShurikenDodge {
            "GAMEPLAY EDIT: alert armed enemies use their own sidestep as soon as the player aims a shuriken at them, at most once per second. Requires clear terrain sight and an available dodge; does not interrupt attacks, injuries, scripts or forced navigation. Off keeps T2's original chance to evade thrown projectiles."
        } else if self == Self::Tenchu2 && item == Item::StrictCorpseSight {
            "GAMEPLAY EDIT: corpse discoveries also require clear terrain sight from the guard's eye height to the animated corpse root. T2's original half-height-to-position ray, scan cadence, range, facing and shared alerts still apply. Off restores the original single ray."
        } else if self == Self::Tenchu2 && item == Item::MultiTargetAttacks {
            "GAMEPLAY EDIT: player melee swings can keep hitting new enemies throughout their active frames. Each striking limb can hit a given enemy once per swing. Damage, blocks, recoil, weapon clashes and swing timing stay native. Off restores T2's original hit consumption."
        } else if self == Self::Tenchu2 && item == Item::SharpText {
            "Sharp health, Ki and item digits, equipment counts, results numbers and clearer disc-derived briefing calligraphy. Story dialogue and menu descriptions already use outline text. Authored title and rank artwork stays original."
        } else if self == Self::Tenchu2 && item == Item::SaveState {
            "F11 saves native T2 gameplay or a 3D cutscene, including imported T1 stages. Overwrites the selected T2 slot, separate from Gaisen slots. Retains campaign/retry context, overrides and presentation effects."
        } else if self == Self::Tenchu2 && item == Item::LoadState {
            "F12 loads the selected T2 state and pauses until F3. Open native T2 first; imported T1 states require the same stage, source character, layout and language. Current graphics remain in use. Loading does not write campaign progress."
        } else if self == Self::Tenchu2 && item == Item::StateSlot {
            "Select one of five native T2 save slots. These are separate from Gaisen slots and require the same game version."
        } else if self == Self::Tenchu2 && matches!(item, Item::Rewind | Item::RewindHistory) {
            "Record up to 30 seconds of native T2 gameplay and 3D cutscenes, including imported T1 stages. Ctrl+F12 plays backward; the timeline seeks or plays forward, and F3 resumes from the selected state. Menus and movie playback are omitted. Export selected frames or import a T2 recording under General. History uses at most 96 MiB and may be shorter for large scenes."
        } else if self == Self::Tenchu2 && item == Item::ResetTweaks {
            "Restore normal simulation speed, original sight and activation, native stance buttons, and disable optional walking/sheathing, dismemberment, quick shuriken evasion, strict corpse sight, God mode, never noticed, bleeding and click tools. Keeps graphics, character overrides, armour, inventory and the current mission state."
        } else if self == Self::Tenchu2 && item == Item::Reset {
            "Reset all saved preferences and debug tools, including character overrides and simulation tweaks. Keeps the current mission, armour and inventory; does not undo actor edits, kills or campaign progress."
        } else {
            fallback
        }
    }

    pub fn unavailable(self, item: Item) -> Option<&'static str> {
        use Item::*;
        use tenchu_core::effects::RumbleAction;
        if self == Self::Gaisen
            && matches!(
                item,
                VibrationAction(
                    RumbleAction::Carry
                        | RumbleAction::Drowning
                        | RumbleAction::Scenery
                        | RumbleAction::Detection
                )
            )
        {
            return Some("This source vibration event is specific to native Tenchu 2.");
        }
        if matches!(item, Tenchu2Missions | Tenchu2Diagnostics | Tenchu2Launcher) {
            return (self != Self::Tenchu2).then_some(
                "Available during native Tenchu 2 Japan play. Use F6 to select an imported JP mission first.",
            );
        }
        if self == Self::Gaisen {
            return None;
        }
        match item {
            IndependentAttacks => Some(
                "Native T2 already gives each fighter its own attack cooldown. This option removes T1's shared cooldown and is not needed here.",
            ),
            StaySheathed => Some(
                "Native T2 already preserves the current weapon state after injury and knockdown. This option changes T1's automatic draw after recovery; it is not needed here.",
            ),
            GuardControls => Some(
                "Native T2 already has its original guard/dodge stance. Use Shift / Circle action to choose Circle or R1; this extra T1 R1 controller does not apply.",
            ),
            GuardDraw => Some(
                "This automatic draw option belongs to T1's extra R1 guard controller. Native T2 keeps its original weapon state when guarding; draw with an attack before blocking.",
            ),
            VibrationAction(
                RumbleAction::WeaponClash
                | RumbleAction::JumpCollision
                | RumbleAction::Landing
                | RumbleAction::FlipLanding
                | RumbleAction::WaterEntry,
            ) => Some(
                "Native T2 does not emit a vibration request for this action. Its original motor events are controlled by the other rows.",
            ),
            Vibration | VibrationIntensity | VibrationAction(_) => None,
            Fullscreen if cfg!(target_os = "android") => {
                Some("Android manages the game window; fullscreen is a desktop setting.")
            }
            Fps | Performance | Focus | MenuPause | MusicWhilePaused | WaitAfterDeath
            | LiveDeathScene | Volume | Fullscreen | Vsync | Hud | SharpText | Preset | Retro
            | Fog | FogDistance | Textures | Shading | Exposure | Filtering | Interpolation
            | Magnification | SurfaceDetail | SurfaceStrength | Antialiasing | AssetPack
            | Upscale | Post | Filmic | Bloom | BloomThreshold | Occlusion | OcclusionRadius
            | Contrast | Saturation | Temperature | Gamma | Vignette | Sharpen | FogTint | Haze
            | CutsceneSpeed | Pause | Step | Speed | FreeCamera | FlightSpeed | Fov
            | FocusPlayer | FocusActor | Actor | Overhead | SaveCamera | RestoreCamera
            | PhotoMode | Hitboxes | Labels | InspectActor | Xray | Trail | Sight | SightStance
            | Patrols | Events | Collision | CollisionOnly | Target(_) | PlayerPosition
            | CameraPosition | SnapFloor | Teleport | PickTeleport | ClickTeleport | KillActor
            | ClickKill | Screenshot | ActorPosition | PickActorPosition | MoveActor
            | RemoveActor | SpawnActor => None,
            CircleConfirm | CircleAction | CameraRelative | AlignAim | FollowActor
            | FollowDistance | RecenterOrbit | MouseSensitivity | StickSensitivity
            | StickDeadzone | InvertCameraY | AnalogWalking | WalkThreshold => None,
            Sheathing | UnarmedAttacks | UnarmedStyle | SheathedSpeed | SheathedVisibility
            | SheathedDamage | SheathedOneHit => None,
            Dismemberment | HeadRest | SaExecutionScene | SeverStealth | SeverDive | SeverArms
            | SeverSide | PlayerDismemberment | PlayerSeverHead | PlayerSeverArms
            | PlayerSeverSide | SeveredBlood | SeveredStyle | SeveredSeconds => None,
            GodMode | Armor | SightDistance | Stealth | KeepEnemiesActive | GiveItems
            | ClearItems | Inventory(_) | Reset | ResetTweaks | QuickShurikenDodge
            | StrictCorpseSight | MultiTargetAttacks => None,
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
            FoliageDetail | VegetationVariation | FallingLeaves | LeafDensity | ContactScatter
            | BakedSnowFootprints => None,
            GushStyle | DeathGush | GushSeconds => None,
            BloodDrips | BloodSeconds | BloodSpurts | BloodWeapons | WeaponBloodMetalOnly
            | WeaponBloodCoverage | WeaponBloodFlow => None,
            BloodGround | BloodDeath | BloodPlayerDeath | BloodDeathSize | BloodRetain
            | BloodModels | BloodConform | BloodAmount | BloodMarks | BloodRunoff | ClearBlood => {
                None
            }
            Water | WaterReflective | WaterRipples | WaterReflections | WaterFlow | WaterFoam
            | WaterSpray | WaterDetail => None,
            PlayerModel | ModelMoveset | MovesetModel | WeaponModel | VoiceModel
            | ModelPlayerMovement | ModelNoIdleTaunts => None,
            Bleeding | BleedChance | BleedSeconds | BleedDamage | BandageLoadout | Bandages
            | UseBandage | MedicineBleeding => None,
            ReloadModels | ReloadAssets | SmoothModels | ModelDetail | CharacterRefinement
            | WeaponReflections => None,
            HairMotion | ClothMotion | EquipmentMotion | MotionStrength | MotionResponse
            | MotionWind => None,
            SaveState | LoadState | StateSlot => None,
            Rewind | RewindHistory | RewindSpeed => None,
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
            Edit::PlayerModel(_)
            | Edit::MovesetModel(_)
            | Edit::WeaponModel(_)
            | Edit::VoiceModel(_) => self.unavailable(Item::PlayerModel),
            Edit::Coordinate(axis, _) => self.unavailable(Item::Target(*axis)),
            Edit::Benchmark => {
                Some("Performance benchmarks are not connected to native Tenchu 2 JP yet.")
            }
            Edit::Diagnostics => None,
            Edit::ImportUnarmed | Edit::ExportUnarmed => Some(
                "Native T2 currently reads Tatsumaru from the running JP disc and Onikage from the connected Gaisen disc. Portable unarmed import/export is not connected here yet.",
            ),
            Edit::GameplayPreset(_) => self.unavailable(Item::GameplayPreset),
            Edit::Randomizer => self.unavailable(Item::Randomizer),
            Edit::SeekRewind(_)
            | Edit::ResumeRewind
            | Edit::PlayRewindForward
            | Edit::SelectRewind(_)
            | Edit::ExportRewind
            | Edit::ImportRewind
            | Edit::RecordedVisuals(_) => self.unavailable(Item::Rewind),
        }
    }
}
