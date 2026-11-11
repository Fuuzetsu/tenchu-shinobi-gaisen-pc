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
    crate::savestate::encode(&snapshot(playback, drawing, settings, tools)?)
}

fn snapshot<'a>(
    playback: &'a Playback<impl ReadAt>,
    drawing: &ContentDrawing,
    settings: &Settings,
    tools: &Tools,
) -> Result<State<campaign::state::Borrowed<'a>>> {
    let visuals = drawing
        .capture_state()
        .ok_or("Start a mission or 3D cutscene before saving a state")?;
    visuals.validate(&playback.content)?;
    Ok(State {
        signature: SIGNATURE.into(),
        campaign: playback.snapshot()?,
        preferences: settings.clone(),
        orbit: tools.orbit().clone(),
        actors_edited: tools.debug.actors_edited,
        visuals,
    })
}

pub(super) fn capture_rewind(
    playback: &Playback<impl ReadAt>,
    drawing: &ContentDrawing,
    settings: &Settings,
    tools: &Tools,
    recorder: &mut crate::rewind::Recorder,
) -> Result<crate::rewind::Snapshot> {
    use tenchu_assets::checkpoint::Capture;
    let State {
        signature,
        campaign,
        preferences,
        orbit,
        actors_edited,
        visuals,
    } = snapshot(playback, drawing, settings, tools)?;
    recorder.begin();
    recorder.value(&signature)?;
    campaign.capture(recorder)?;
    recorder.shared(&serde_json::to_string(&preferences).map_err(|e| e.to_string())?)?;
    recorder.value(&orbit)?;
    recorder.value(&actors_edited)?;
    visuals.capture(recorder)?;
    recorder.finish()
}

pub fn restore(
    playback: &mut Playback<impl ReadAt>,
    bytes: &[u8],
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
) -> std::result::Result<ContentDrawing, RestoreError> {
    restore_snapshot(
        playback,
        crate::savestate::decode(bytes)?,
        settings,
        tools,
        renderer,
        None,
        false,
    )
    .map(|drawing| drawing.expect("new drawing for a disk state"))
}

#[cfg(test)]
fn restore_rewind(
    playback: &mut Playback<impl ReadAt>,
    snapshot: &crate::rewind::Snapshot,
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
    drawing: &mut ContentDrawing,
) -> std::result::Result<(), RestoreError> {
    restore_recorded(
        playback, snapshot, settings, tools, renderer, drawing, false,
    )
}

pub(super) fn restore_recorded(
    playback: &mut Playback<impl ReadAt>,
    snapshot: &crate::rewind::Snapshot,
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
    drawing: &mut ContentDrawing,
    recorded_visuals: bool,
) -> std::result::Result<(), RestoreError> {
    if let Some(replacement) = restore_snapshot(
        playback,
        snapshot.decode()?,
        settings,
        tools,
        renderer,
        Some(drawing),
        recorded_visuals,
    )? {
        *drawing = replacement;
    }
    Ok(())
}

fn restore_snapshot(
    playback: &mut Playback<impl ReadAt>,
    mut saved: State<campaign::state::Owned>,
    settings: &mut Settings,
    tools: &mut Tools,
    renderer: &mut Renderer,
    drawing: Option<&mut ContentDrawing>,
    recorded_visuals: bool,
) -> std::result::Result<Option<ContentDrawing>, RestoreError> {
    if saved.signature != SIGNATURE {
        return Err("This is not a native T2 JP savestate".to_string().into());
    }
    saved.preferences.validate();
    let graphics = super::preferences::graphics(if recorded_visuals {
        &saved.preferences
    } else {
        settings
    });
    let mut drawing_changed = false;
    let replacement = playback
        .restore_snapshot_with(saved.campaign, |content, reused| {
            saved.visuals.validate(content)?;
            if reused
                && renderer.graphics() == &graphics
                && let Some(drawing) = drawing
            {
                drawing_changed = true;
                drawing.restore_state(saved.visuals, content, renderer)?;
                return Ok(None);
            }
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
            Ok(Some(drawing))
        })
        .map_err(|message| RestoreError {
            message,
            drawing_changed,
        })?;
    settings.restore_playback_preferences(&saved.preferences);
    if recorded_visuals {
        settings.graphics = saved.preferences.graphics;
        settings.retro = saved.preferences.retro;
    }
    playback.restore_preferences(settings);
    tools.restore_checkpoint(
        &playback.content,
        playback.generation,
        saved.orbit,
        saved.actors_edited,
    );
    Ok(replacement)
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
