use super::Session;
use std::sync::Arc;
use tenchu_assets::{ReadAt, Result, audio::stream::Clip, disc::Disc};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Training {
    playing: Option<(u16, u32)>,
    pending: bool,
    clip: Option<Arc<Clip>>,
}

impl Training {
    pub fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        out.value(&self.playing)?;
        out.value(&self.pending)?;
        out.value(&self.clip.is_some())?;
        if let Some(clip) = &self.clip {
            out.shared_arc(clip, true)?;
        }
        Ok(())
    }
    pub fn audio(
        &mut self,
        disc: &Disc<impl ReadAt>,
        training: &tenchu_core::tenchu2::tutorial::Training,
    ) -> Result<Option<Arc<Clip>>> {
        if !self.pending {
            return Ok(None);
        }
        let (cue, _) = self.playing.ok_or("Missing training narration request")?;
        let clip = Arc::new(Clip::load(disc, &training.data.tracks[&cue])?);
        self.clip = Some(clip.clone());
        self.pending = false;
        Ok(Some(clip))
    }
    pub fn validate(&self, training: &tenchu_core::tenchu2::tutorial::Training) -> Result<()> {
        if self.playing != training.playing
            || self.pending && self.playing.is_none()
            || self.clip.as_ref().is_some_and(|clip| {
                self.playing.is_none() || clip.rate == 0 || clip.frames.is_empty()
            })
        {
            return Err("Invalid training narration playback".into());
        }
        training.validate()
    }
    pub fn resume(&self, tick: u32, output: &crate::audio::output::Output) {
        use crate::audio::output::Command;
        if let (Some(clip), Some((_, start))) = (&self.clip, self.playing) {
            output.send(Command::Music(Some(clip.clone()), false));
            output.send(Command::SeekMusic(
                f64::from(tick.wrapping_sub(start)) / 30.0,
            ));
        }
    }
}
impl Session {
    pub(super) fn training_step(&mut self) -> Option<tenchu_core::tenchu2::audio::Sound> {
        let Some(training) = &mut self.training else {
            return None;
        };
        if self.frame.tutorial.cue.is_some() {
            training.playing = self.world.tutorial.as_ref().and_then(|t| t.playing);
            training.pending = true;
            training.clip = None;
        }
        if self.frame.tutorial.confirmation {
            let tick = training.playing.map_or(self.world.tick, |(_, tick)| tick);
            self.world
                .audio
                .play(2, 0, 127, tick, &self.assets.resources.math)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::tenchu2::tutorial::{Input, State};
    fn tick(session: &mut Session) -> Option<tenchu_core::tenchu2::audio::Sound> {
        session.frame.tutorial = session.world.update_training(&session.assets).unwrap();
        session.training_step()
    }
    #[derive(serde::Deserialize)]
    struct Case {
        before: State,
        position: [i32; 3],
        action: i16,
        frame: i16,
        loops: i16,
        duration: i16,
        camera: i16,
        audio_finished: bool,
        height: i16,
        terrain: i32,
        selected: u8,
        active: u8,
        dead: usize,
        expected: serde_json::Value,
        cue: Option<u16>,
        confirmation: bool,
        entered: Option<usize>,
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC; all four instruction signs, decoded narration and saved lesson continuity"]
    fn training_signs_play_and_restore_their_narration() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut session = Session::load(&disc, 1, 1).unwrap();
        let data = session.world.tutorial.as_ref().unwrap().data.clone();
        let mut recorder = crate::rewind::Recorder::default();
        for (&cue, track) in &data.tracks {
            let clip = Clip::load(&disc, track).unwrap();
            assert!(
                clip.frames
                    .iter()
                    .flatten()
                    .any(|sample| sample.unsigned_abs() > 100),
                "silent cue {cue}"
            );
            let seconds = clip.frames.len() as f64 / f64::from(clip.rate);
            let extent = f64::from(track.end - track.start) / 150.0;
            eprintln!("training cue {cue}: {seconds:.2}s audio, {extent:.2}s disc extent");
            assert!(
                (seconds - extent).abs() < 1.0,
                "cue {cue} playback duration differs from lesson clock"
            );
        }
        for (course, &index) in data.starts.iter().enumerate() {
            session.world.actors[session.world.player].actor.position = [900000, 0, 900000].into();
            tick(&mut session);
            let point = data.lessons[index].position.map(|v| i32::from(v) * 1000);
            session.world.actors[session.world.player].actor.position = point.into();
            tick(&mut session);
            assert_eq!(
                session.world.tutorial.as_ref().unwrap().state.lesson,
                Some(index)
            );
            session.world.tick += 1;
            assert!(tick(&mut session).is_some(), "sign {course} confirmation");
            let training = session.training.as_mut().unwrap();
            assert_eq!(
                training.playing,
                Some((data.lessons[index].cue, session.world.tick))
            );
            let clip = training
                .audio(&disc, session.world.tutorial.as_ref().unwrap())
                .unwrap()
                .unwrap();
            assert!(
                training
                    .audio(&disc, session.world.tutorial.as_ref().unwrap())
                    .unwrap()
                    .is_none(),
                "narration restarted without a new cue"
            );
            let before =
                postcard::to_allocvec(&session.world.tutorial.as_ref().unwrap().state).unwrap();
            let saved = session.capture_state().unwrap();
            recorder.begin();
            session.snapshot().capture(&mut recorder).unwrap();
            assert_eq!(
                recorder.finish().unwrap().bytes().unwrap(),
                postcard::to_allocvec(&session.snapshot()).unwrap(),
                "training rewind capture must match saved state"
            );
            session.world.tick += 12;
            session.training = None;
            session.restore_state(&saved).unwrap();
            let restored = session.training.as_ref().unwrap();
            assert_eq!(
                postcard::to_allocvec(&session.world.tutorial.as_ref().unwrap().state).unwrap(),
                before
            );
            assert_eq!(restored.clip.as_ref().unwrap().frames, clip.frames);
            assert_eq!(session.capture_state().unwrap(), saved);
            session.world.tick += 1;
            tick(&mut session);
            assert!(
                !session.training.as_ref().unwrap().pending,
                "stationary player retriggered the sign"
            );
        }
        let training = session.world.tutorial.as_mut().unwrap();
        training.state.lesson = Some(data.lessons.len());
        assert!(training.validate().is_err());
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC and TENCHU2_TUTORIAL_ORACLE from the JP DATAMNG handler"]
    fn training_lessons_match_original_handler() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let session = Session::load(&disc, 1, 1).unwrap();
        let data = &session.world.tutorial.as_ref().unwrap().data;
        let oracle =
            std::fs::read_to_string(std::env::var_os("TENCHU2_TUTORIAL_ORACLE").unwrap()).unwrap();
        let mut count = 0;
        for (i, line) in oracle.lines().enumerate() {
            let case: Case = serde_json::from_str(line).unwrap();
            let mut state = case.before;
            let mut player = session.world.actors[session.world.player].actor.clone();
            player.position = case.position.into();
            player.motion.action = case.action;
            player.motion.frame = case.frame;
            player.motion.loops = case.loops;
            player.height = case.height;
            player.terrain.height = case.terrain;
            player.inventory.selected = Some(case.selected.try_into().unwrap());
            player.inventory.active = case.active.try_into().unwrap();
            let update = state.step(
                data,
                Input {
                    player: &player,
                    duration: case.duration,
                    camera: case.camera,
                    defeated_practice_guards: case.dead,
                    audio_finished: case.audio_finished,
                },
            );
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                case.expected,
                "case {i}"
            );
            assert_eq!(
                (update.cue, update.confirmation, update.entered),
                (case.cue, case.confirmation, case.entered),
                "case {i}"
            );
            count += 1;
        }
        assert!(count >= 4000);
    }
}
