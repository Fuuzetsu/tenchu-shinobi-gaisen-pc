use crate::source::DiscSource;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
};
use tenchu_assets::{
    Result,
    audio::{
        catalog::Catalog,
        stream::{Clip, Track},
    },
    game::GameAssets,
};
use tenchu_audio::Mixer;
use tenchu_core::game::Game;

pub struct Sound {
    file: BufWriter<File>,
    mixer: Mixer,
    catalog: Option<Catalog>,
    track: Option<Track>,
    pub frames: u64,
}

impl Sound {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(Self {
            file: BufWriter::new(File::create(path).map_err(|e| e.to_string())?),
            mixer: Mixer::new(48_000),
            catalog: None,
            track: None,
            frames: 0,
        })
    }

    pub fn reload(
        &mut self,
        assets: &GameAssets<DiscSource>,
        game: &Game,
        music: Option<u8>,
    ) -> Result<()> {
        self.mixer
            .set_bank(Arc::new(crate::audio::game_bank(assets, game)?));
        self.catalog = Some(Catalog::load(
            &assets.executable,
            game.mission.scripts.language,
        )?);
        let track = music
            .and_then(|cue| self.catalog.as_ref().unwrap().music(i32::from(cue)))
            .cloned();
        if track != self.track {
            self.music(assets, track)?;
        }
        Ok(())
    }

    fn music(&mut self, assets: &GameAssets<DiscSource>, track: Option<Track>) -> Result<()> {
        let clip = track
            .as_ref()
            .map(|track| Clip::load(&assets.disc, track).map(Arc::new))
            .transpose()?;
        self.mixer
            .music(clip, track.as_ref().is_some_and(|t| t.repeat));
        self.track = track;
        Ok(())
    }

    pub fn tick(&mut self, assets: &GameAssets<DiscSource>, game: &Game) -> Result<()> {
        for sound in game.sounds()? {
            self.mixer.play(sound.id, sound.volume, sound.pan);
        }
        for cue in &game.music {
            let track = cue.and_then(|id| self.catalog.as_ref()?.event(id)).cloned();
            self.music(assets, track)?;
        }
        Ok(())
    }

    pub fn frame(&mut self) -> Result<()> {
        let mut bytes = [0; 800 * 2 * 4];
        for chunk in bytes.chunks_exact_mut(8) {
            let [left, right] = self.mixer.frame();
            chunk[..4].copy_from_slice(&left.to_le_bytes());
            chunk[4..].copy_from_slice(&right.to_le_bytes());
        }
        self.file.write_all(&bytes).map_err(|e| e.to_string())?;
        self.frames += 1;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.file.flush().map_err(|e| e.to_string())
    }
}
