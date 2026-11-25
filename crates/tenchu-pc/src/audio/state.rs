use super::*;
use crate::level_editor::custom::Asset;
use serde::{Deserialize, Serialize};
use tenchu_assets::checkpoint::Capture;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Source {
    Disc(Track),
    Custom(String),
}

impl Source {
    pub fn repeat(&self) -> bool {
        match self {
            Self::Disc(track) => track.repeat,
            Self::Custom(_) => true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct SandboxState {
    pub fallback: Option<Track>,
    pub asset: Option<Arc<Asset>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub(super) current: Option<Source>,
    pub(super) mission: Option<Source>,
    pub(super) playhead: f64,
    pub(super) sandbox: Option<SandboxState>,
}

impl State {
    pub(crate) fn silent_sandbox(campaign: &Campaign) -> Option<Self> {
        let music = &campaign.sandbox.as_ref()?.music;
        let source = music
            .custom
            .as_ref()
            .map(|a| Source::Custom(a.id().into()))
            .or_else(|| music.track().map(Source::Disc));
        Some(Self {
            current: source.clone(),
            mission: source,
            playhead: 0.0,
            sandbox: Some(SandboxState {
                fallback: music.track(),
                asset: music.custom.clone(),
            }),
        })
    }

    pub(crate) fn reuse_theme(&mut self, campaign: &Campaign) {
        if let Some(asset) = self.sandbox.as_mut().and_then(|s| s.asset.as_mut())
            && let Some(previous) = campaign
                .sandbox
                .as_ref()
                .and_then(|s| s.music.custom.as_ref())
        {
            if asset == previous {
                *asset = previous.clone();
            } else {
                Arc::make_mut(asset).reuse_payload(previous);
            }
        }
    }

    pub(crate) fn validate(&self, sandbox: bool) -> Result<()> {
        if let Some(saved) = &self.sandbox {
            if !sandbox {
                return Err("Sandbox music requires a Tenchu 2 scenery sandbox state".into());
            }
            if let Some(asset) = &saved.asset {
                theme::validate(asset)?;
            }
        }
        for source in self.current.iter().chain(&self.mission) {
            if let Source::Custom(id) = source
                && self
                    .sandbox
                    .as_ref()
                    .and_then(|s| s.asset.as_ref())
                    .is_none_or(|a| a.id() != id)
            {
                return Err("Missing custom theme in savestate".into());
            }
        }
        Ok(())
    }

    pub(crate) fn validate_session(&self, campaign: &Campaign) -> Result<()> {
        if let Some(theme) = &self.sandbox {
            let music = &campaign
                .sandbox
                .as_ref()
                .ok_or("Open this theme's sandbox first")?
                .music;
            if theme
                .fallback
                .as_ref()
                .is_some_and(|track| !music.tracks.contains(track))
            {
                return Err("Saved music selection does not belong to this sandbox".into());
            }
        }
        Ok(())
    }

    pub(crate) fn restore_session(&self, campaign: &mut Campaign) {
        if let Some(music) = campaign.sandbox.as_mut().map(|s| &mut s.music) {
            music.custom = self.sandbox.as_ref().and_then(|theme| theme.asset.clone());
            if let Some(theme) = &self.sandbox {
                music.selected = theme
                    .fallback
                    .as_ref()
                    .and_then(|track| music.tracks.iter().position(|t| t == track));
            }
        }
    }

    pub(crate) fn capture(&self, recorder: &mut impl Capture<Error = String>) -> Result<()> {
        recorder.value(&self.current)?;
        recorder.value(&self.mission)?;
        recorder.value(&self.playhead)?;
        recorder.value(&self.sandbox.is_some())?;
        if let Some(theme) = &self.sandbox {
            recorder.value(&theme.fallback)?;
            recorder.value(&theme.asset.is_some())?;
            if let Some(asset) = &theme.asset {
                recorder.shared_arc(asset, true)?;
            }
        }
        Ok(())
    }
}
