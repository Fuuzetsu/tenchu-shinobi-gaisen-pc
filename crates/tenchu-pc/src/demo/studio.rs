#[derive(Clone, Copy, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    Workspace,
    Terrain,
    Stamps,
    Scenery,
    Characters,
    Patrols,
    Lighting,
    Weather,
    Regions,
    Sounds,
    Playtest,
    Projects,
    Views,
    Mixed,
}

impl Feature {
    pub fn readout(self) -> &'static str {
        match self {
            Self::Workspace => "Mouse look / fly / pan · zoomable map · G hides guides",
            Self::Terrain => "Area fill → Undo → Redo · I samples tiles and rotation",
            Self::Stamps => "Copy both layers · rotate · paste · keep named stamps",
            Self::Scenery => "T1 + T2 source materials · snapping · nudge · ground on terrain",
            Self::Characters => "T1 Rikimaru + T2 Rikimaru · Gaisen gameplay rules",
            Self::Patrols => "Adjacent patrols · facing and waits · 64 waypoints",
            Self::Lighting => "Day → dusk → night · project settings leave game preferences intact",
            Self::Weather => "Clear → rainstorm → snow · gameplay remains at tick zero",
            Self::Regions => "Drag an area → create a region event · ordered cue queues",
            Self::Sounds => "Hear a T1 cue, then a T2 cue · independent source banks",
            Self::Playtest => "F5: isolated playtest · timed message and storm · return to editing",
            Self::Projects => "Save / copy · .tenchu projects · retail .msn / .mcs and share codes",
            Self::Views => "Named camera views saved with the project · clean preview",
            Self::Mixed => "Staged editor showcase · decorative scenery · both discs required",
        }
    }
}
