use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    ReadAt, Result,
    audio::stream::{Clip, Track},
    disc::Disc,
};
use tenchu_core::tenchu2::world::World;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Music {
    tracks: BTreeMap<u16, Track>,
    playing: Option<(u16, u32)>,
    pending: bool,
    clip: Option<Arc<Clip>>,
}

impl Music {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        Ok(Self {
            tracks: tenchu_assets::tenchu2::music::cues(disc)?,
            ..Default::default()
        })
    }

    pub fn step(&mut self, world: &World, event: Option<u16>) {
        let idle = self.playing.is_none_or(|(cue, start)| {
            let track = &self.tracks[&cue];
            world.tick.wrapping_sub(start) >= (track.end - track.start) / 5
        });
        let cue = event.or_else(|| {
            idle.then(|| {
                let player = &world.actors[world.player].actor;
                let target = world.mission.targets[0]?;
                world.mission.boss_music(
                    world.controls.death_music,
                    player.health,
                    world.actors[target].actor.mission_kind(),
                )
            })?
        });
        if let Some(cue) = cue.filter(|cue| self.tracks.contains_key(cue)) {
            if self.playing.is_none_or(|(previous, _)| previous != cue) {
                self.clip = None;
            }
            self.playing = Some((cue, world.tick));
            self.pending = true;
        }
    }

    pub fn audio(&mut self, disc: &Disc<impl ReadAt>) -> Result<Option<Arc<Clip>>> {
        if !self.pending {
            return Ok(None);
        }
        let (cue, _) = self.playing.ok_or("Missing native T2 music request")?;
        if self.clip.is_none() {
            self.clip = Some(Arc::new(Clip::load(disc, &self.tracks[&cue])?));
        }
        self.pending = false;
        Ok(self.clip.clone())
    }

    pub fn interrupt(&mut self) {
        self.playing = None;
        self.pending = false;
        self.clip = None;
    }

    pub fn resume(&self, tick: u32, output: &crate::audio::output::Output) {
        use crate::audio::output::Command;
        if let (Some((_, start)), Some(clip)) = (self.playing, &self.clip) {
            output.send(Command::Music(Some(clip.clone()), false));
            output.send(Command::SeekMusic(
                f64::from(tick.wrapping_sub(start)) / 30.0,
            ));
        }
    }

    pub fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        out.value(&self.tracks)?;
        out.value(&self.playing)?;
        out.value(&self.pending)?;
        out.value(&self.clip.is_some())?;
        if let Some(clip) = &self.clip {
            out.shared_arc(clip, true)?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.tracks.values().any(|track| track.start >= track.end)
            || self
                .playing
                .is_some_and(|(cue, _)| !self.tracks.contains_key(&cue))
            || self.pending && self.playing.is_none()
            || self.clip.as_ref().is_some_and(|clip| {
                self.playing.is_none() || clip.rate == 0 || clip.frames.is_empty()
            })
        {
            return Err("Invalid native T2 music playback".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
