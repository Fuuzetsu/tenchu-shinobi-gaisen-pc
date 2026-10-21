use serde::Deserialize;
use tenchu_assets::Result;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Feature {
    Presets,
    Lanterns,
    Shadows,
    Moon,
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
    Guard,
    Wide,
    Cinematic,
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
}

pub fn builtin() -> Result<Vec<Clip>> {
    parse(include_bytes!("../../data/demos.json"))
}

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
    }
    Ok(clips)
}
