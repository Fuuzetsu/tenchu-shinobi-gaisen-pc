use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use tenchu_assets::Result;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    Presets,
    Lanterns,
    FireLights,
    Shadows,
    Moon,
    DayCycle,
    Rain,
    Snow,
    Water,
    Waterfalls,
    Vegetation,
    CharacterRefinement,
    TextureSampling,
    Reconstruction,
    Antialiasing,
    ModelDetail,
    Interpolation,
    PlayerModels,
    ContactShading,
    Bloom,
    Atmosphere,
    ColorGrading,
    Inspection,
    Collision,
    FreeCamera,
    EnemySimulation,
    Simulation,
    Cinematics,
    Performance,
    Artwork,
    DistanceFiltering,
    Surfaces,
    LightRange,
    LightBudget,
    MovableLight,
    ShadowQuality,
    SkyDirection,
    SkyQuality,
    ImageFinish,
    SightDistance,
    Teleport,
    Photo,
    Interface,
    Step,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Shot {
    Player,
    Lamp,
    Fire,
    Guard,
    Wide,
    Cinematic,
    Gameplay,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: String,
    pub title: String,
    pub description: String,
    pub seconds: u16,
    pub mission: usize,
    pub character: i16,
    pub preset: u16,
    pub feature: Feature,
    pub shot: Shot,
    pub artwork: Option<bool>,
    pub fog: Option<bool>,
    pub camera: Option<super::framing::Path>,
    pub recording: Option<Recording>,
    #[serde(default)]
    pub hud: bool,
    #[serde(default)]
    pub sharp_text: bool,
    pub frontend: Option<crate::frontend::showcase::Scene>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub start_tick: u32,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn builtin() -> Result<Vec<Clip>> {
    parse(include_bytes!("../../data/demos.json"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse(bytes: &[u8]) -> Result<Vec<Clip>> {
    let clips: Vec<Clip> =
        serde_json::from_slice(bytes).map_err(|e| format!("Demo catalog: {e}"))?;
    if clips.is_empty() || clips.len() > 100 {
        return Err("Demo catalog needs 1–100 clips".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for c in &clips {
        if c.id.is_empty()
            || c.id.len() > 64
            || !c
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !ids.insert(&c.id)
        {
            return Err(format!(
                "Demo clip ID must be unique, lowercase letters/digits/hyphens: {}",
                c.id
            ));
        }
        if !(2..=120).contains(&c.seconds)
            || c.mission > 10
            || !(0..=1).contains(&c.character)
            || c.preset >= crate::settings::presets::CUSTOM
            || c.title.trim().is_empty()
            || c.description.trim().is_empty()
        {
            return Err(format!(
                "Invalid duration, scene, preset or caption for demo '{}'",
                c.id
            ));
        }
        if (c.feature == Feature::Cinematics) != (c.shot == Shot::Cinematic) {
            return Err(format!(
                "Demo '{}' must pair the cinematics feature and cinematic shot",
                c.id
            ));
        }
        if let Some(camera) = &c.camera {
            camera
                .validate()
                .map_err(|e| format!("Demo '{}': {e}", c.id))?;
        }
        if c.recording.is_some() && (c.feature != Feature::Artwork || c.shot != Shot::Gameplay) {
            return Err(format!(
                "Demo '{}' recordings require the artwork feature and gameplay shot",
                c.id
            ));
        }
        if c.shot == Shot::Gameplay && c.recording.is_none() {
            return Err(format!("Demo '{}' gameplay shot needs a recording", c.id));
        }
        if let Some(scene) = &c.frontend {
            scene.validate()?;
            if c.feature != Feature::Artwork
                || c.shot != Shot::Wide
                || c.camera.is_some()
                || !(1..=10).contains(&c.mission)
            {
                return Err(format!(
                    "Demo '{}' frontend needs artwork, wide, mission 1–10 and no camera",
                    c.id
                ));
            }
        }
    }
    Ok(clips)
}
