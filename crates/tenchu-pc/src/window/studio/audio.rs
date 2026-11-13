use super::*;
use crate::level_editor::AssetGame;
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::audio::{bank::Bank, stream::Clip};

#[derive(Default)]
pub(super) struct Sounds {
    banks: BTreeMap<(AssetGame, usize), Arc<Bank>>,
    music: BTreeMap<(AssetGame, usize), Arc<Clip>>,
    #[cfg(target_arch = "wasm32")]
    pending: BTreeMap<usize, crate::audio::loader::Loader>,
}

impl Application {
    pub(super) fn prepare_studio_audio(&mut self) -> Result<bool> {
        let cues: Vec<_> = self
            .studio
            .project
            .events
            .iter()
            .filter(|e| e.enabled)
            .flat_map(|e| &e.cues)
            .map(|c| c.action.clone())
            .collect();
        let sounds = &mut self
            .studio_previous
            .as_mut()
            .ok_or("Open Level Studio first")?
            .sounds;
        sounds
            .music
            .retain(|&(game, track), _| cues.contains(&CueAction::Music { game, track }));
        let mut ready = true;
        for cue in cues {
            ready &= self.load_studio_audio(&cue)?;
        }
        Ok(ready)
    }

    pub(in crate::window) fn studio_bank(
        &mut self,
        game: AssetGame,
        bank: usize,
    ) -> Result<Arc<Bank>> {
        let cache = &mut self
            .studio_previous
            .as_mut()
            .ok_or("Open Level Studio first")?
            .sounds;
        if let std::collections::btree_map::Entry::Vacant(entry) = cache.banks.entry((game, bank)) {
            let source = match game {
                AssetGame::Tenchu => Bank::load(
                    &self.assets,
                    bank,
                    0,
                    tenchu_assets::script::Language::Japanese,
                )?,
                AssetGame::Tenchu2 => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let disc = crate::source::open_disc(
                            &crate::tenchu2::menu::disc_path()
                                .ok_or("Connect a Tenchu 2 disc from the game launcher")?,
                        )?;
                        tenchu_assets::tenchu2::audio::sound_bank(&disc, 0, bank)?
                    }
                    #[cfg(target_arch = "wasm32")]
                    return Err(
                        "Connect Tenchu 2 assets in the desktop game to audition this cue".into(),
                    );
                }
            };
            entry.insert(Arc::new(source));
        }
        Ok(cache.banks[&(game, bank)].clone())
    }

    fn load_studio_audio(&mut self, action: &CueAction) -> Result<bool> {
        match *action {
            CueAction::CustomSound { ref asset, .. } | CueAction::CustomMusic { ref asset } => {
                let _ = self
                    .studio
                    .project
                    .assets
                    .get(asset)
                    .ok_or("Missing custom audio asset")?
                    .clip()?;
            }
            CueAction::Sound { game, bank, id, .. } => {
                let source = self.studio_bank(game, bank)?;
                if source
                    .programs
                    .get(usize::from(id >> 4))
                    .and_then(Option::as_ref)
                    .and_then(|p| p.tones.get(usize::from(id & 15)))
                    .is_none_or(|tone| tone.sample == 0)
                {
                    return Err(format!("Sound {id} is empty in bank {bank}"));
                }
            }
            CueAction::Music { game, track } => {
                let cache = &mut self
                    .studio_previous
                    .as_mut()
                    .ok_or("Open Level Studio first")?
                    .sounds;
                if cache.music.contains_key(&(game, track)) {
                    return Ok(true);
                }
                if cache.music.len() >= 8 {
                    cache.music.clear();
                }
                let clip = match game {
                    AssetGame::Tenchu => {
                        let catalog = tenchu_assets::audio::catalog::Catalog::load(
                            &self.assets.executable,
                            tenchu_assets::script::Language::Japanese,
                        )?;
                        let cue = catalog
                            .music(track as i32)
                            .ok_or("Music track is unavailable")?;
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            Arc::new(Clip::load(&self.assets.disc, cue)?)
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let std::collections::btree_map::Entry::Vacant(entry) =
                                cache.pending.entry(track)
                            {
                                let mut loader =
                                    crate::audio::loader::Loader::new(std::path::Path::new(""))?;
                                loader.request(Some(cue.clone()));
                                entry.insert(loader);
                            }
                            let Some(loaded) = cache.pending[&track].poll() else {
                                return Ok(false);
                            };
                            cache.pending.remove(&track);
                            loaded?.0
                        }
                    }
                    AssetGame::Tenchu2 => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let disc = crate::source::open_disc(
                                &crate::tenchu2::menu::disc_path()
                                    .ok_or("Connect a Tenchu 2 disc from the game launcher")?,
                            )?;
                            let tracks = tenchu_assets::tenchu2::music::tracks(&disc)?;
                            Arc::new(Clip::load(
                                &disc,
                                tracks.get(track).ok_or("Music track is unavailable")?,
                            )?)
                        }
                        #[cfg(target_arch = "wasm32")]
                        return Err(
                            "Connect Tenchu 2 assets in the desktop game to audition this cue"
                                .into(),
                        );
                    }
                };
                cache.music.insert((game, track), clip);
            }
            _ => (),
        }
        Ok(true)
    }

    pub(super) fn studio_audio(&mut self, action: CueAction, preview: bool) -> Result<()> {
        if !self.load_studio_audio(&action)? {
            #[cfg(target_arch = "wasm32")]
            {
                self.studio_previous.as_mut().unwrap().pending_audio =
                    Some(Command::Preview(action));
            }
            self.studio.notice = "Preparing music preview…".into();
            return Ok(());
        }
        if preview {
            self.studio.notice = "Audio preview ready".into();
        }
        let sounds = &self.studio_previous.as_ref().unwrap().sounds;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(sound) = self.video.as_mut().and_then(|v| v.sound.as_mut()) {
            match action {
                CueAction::CustomSound {
                    ref asset,
                    volume,
                    pan,
                } => sound.studio_clip(self.studio.project.assets[asset].clip()?, volume, pan),
                CueAction::CustomMusic { ref asset } => {
                    sound.studio_music(Some(self.studio.project.assets[asset].clip()?))
                }
                CueAction::Sound {
                    game,
                    bank,
                    id,
                    volume,
                    pan,
                } => sound.studio_sound(&sounds.banks[&(game, bank)], id as i16, volume, pan),
                CueAction::Music { game, track } => {
                    sound.studio_music(Some(sounds.music[&(game, track)].clone()))
                }
                CueAction::StopMusic => sound.studio_music(None),
                _ => (),
            }
        }
        if let Some(audio) = &mut self.audio {
            match action {
                CueAction::CustomSound {
                    ref asset,
                    volume,
                    pan,
                } => audio.studio_clip(
                    self.studio.project.assets[asset].clip()?,
                    volume,
                    pan,
                    preview,
                ),
                CueAction::CustomMusic { ref asset } => {
                    audio.studio_music(Some(self.studio.project.assets[asset].clip()?), preview)
                }
                CueAction::Sound {
                    game,
                    bank,
                    id,
                    volume,
                    pan,
                } => audio.studio_sound(
                    sounds.banks[&(game, bank)].clone(),
                    tenchu_core::audio::Sound {
                        id: id as i16,
                        volume,
                        pan,
                    },
                    preview,
                ),
                CueAction::Music { game, track } => {
                    audio.studio_music(Some(sounds.music[&(game, track)].clone()), preview)
                }
                CueAction::StopMusic => audio.studio_music(None, preview),
                _ => (),
            }
        }
        Ok(())
    }
}
