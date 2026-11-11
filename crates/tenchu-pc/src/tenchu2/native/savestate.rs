use super::{
    campaign::{self, Content, Playback},
    presentation::ContentDrawing,
    render,
    tools::Tools,
};
use crate::{renderer::Renderer, settings::Settings};
use serde::{Deserialize, Serialize};
use tenchu_assets::{ReadAt, Result};

const SIGNATURE: &str = "native-tenchu2-jp-state-1";

#[derive(Debug)]
pub struct RestoreError {
    pub message: String,
    pub drawing_changed: bool,
}

impl From<String> for RestoreError {
    fn from(message: String) -> Self {
        Self {
            message,
            drawing_changed: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct State<C> {
    signature: String,
    campaign: C,
    #[serde(with = "crate::savestate::json")]
    preferences: Settings,
    orbit: crate::navigation::Orbit,
    actors_edited: bool,
    visuals: render::state::State,
}

pub fn path(slot: u8) -> Result<std::path::PathBuf> {
    if slot >= 5 {
        return Err("Invalid native T2 savestate slot".into());
    }
    Ok(crate::storage::UserFile::States
        .path()?
        .join(format!("t2-jp-slot-{}.tstate", slot + 1)))
}

pub fn capture(
    playback: &Playback<impl ReadAt>,
    drawing: &ContentDrawing,
    settings: &Settings,
    tools: &Tools,
) -> Result<Vec<u8>> {
    let visuals = drawing
        .capture_state()
        .ok_or("Start a mission or 3D cutscene before saving a state")?;
    visuals.validate(&playback.content)?;
    crate::savestate::encode(&State {
        signature: SIGNATURE.into(),
        campaign: playback.snapshot()?,
        preferences: settings.clone(),
        orbit: tools.orbit().clone(),
        actors_edited: tools.debug.actors_edited,
        visuals,
    })
}

pub fn restore(
    playback: &mut Playback<impl ReadAt>,
    bytes: &[u8],
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
) -> std::result::Result<ContentDrawing, RestoreError> {
    let mut saved: State<campaign::state::Owned> = crate::savestate::decode(bytes)?;
    if saved.signature != SIGNATURE {
        return Err("This is not a native T2 JP savestate".to_string().into());
    }
    saved.preferences.validate();
    let graphics = super::preferences::graphics(settings);
    let mut drawing_changed = false;
    let drawing = playback
        .restore_snapshot(saved.campaign, |content| {
            saved.visuals.validate(content)?;
            let vram = match content {
                Content::Gameplay(session) => &session.assets.sprites.vram,
                Content::Story { story, .. } => &story.sprites.vram,
                _ => return Err("Saved native T2 scene cannot be restored".into()),
            };
            drawing_changed = true;
            renderer.configure(&graphics);
            renderer.reset_vram(vram);
            let mut drawing = ContentDrawing::new(content, &graphics, renderer)?;
            drawing.restore_state(saved.visuals, content, renderer)?;
            Ok(drawing)
        })
        .map_err(|message| RestoreError {
            message,
            drawing_changed,
        })?;
    settings.restore_playback_preferences(&saved.preferences);
    playback.restore_preferences(settings);
    tools.restore_checkpoint(
        &playback.content,
        playback.generation,
        saved.orbit,
        saved.actors_edited,
    );
    Ok(drawing)
}

impl Content {
    pub fn current_model(&self, hero: usize, settings: &Settings) -> bool {
        match self {
            Self::Gameplay(session) => session.models.current(settings),
            Self::Story { story, .. } => story.models.current(hero, &settings.player_model),
            _ => true,
        }
    }

    pub fn embedded_bodies(&self) -> Option<std::sync::Arc<crate::visuals::pack::Pack>> {
        let custom = match self {
            Self::Gameplay(session) => session.models.custom.as_ref(),
            Self::Story { story, .. } => story.models.custom.as_ref(),
            _ => None,
        };
        custom.map(|custom| custom.pack.clone())
    }
}

#[cfg(test)]
mod tests;
