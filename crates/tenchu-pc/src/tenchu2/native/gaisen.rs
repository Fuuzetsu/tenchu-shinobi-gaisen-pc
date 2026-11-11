use super::{Session, *};
use crate::{
    audio::output::{Command, Output},
    source::{DiscSource, open_disc},
};
use std::path::PathBuf;
use tenchu_assets::{
    audio::{
        bank::Bank,
        catalog::Catalog,
        stream::{Clip, Track},
    },
    executable::Executable,
    game::GameAssets,
    script::Language,
    stage::MissionAssets,
};
use tenchu_core::{game::Game, tenchu2::world::gaisen::Director};

pub struct Source {
    assets: GameAssets<DiscSource>,
    path: PathBuf,
    stage: usize,
    character: i16,
    layout: usize,
    language: Language,
}

impl Source {
    pub fn open(
        path: &Path,
        stage: usize,
        character: i16,
        layout: usize,
        language: Language,
    ) -> Result<Self> {
        Ok(Self {
            assets: GameAssets::open(open_disc(path)?)?,
            path: path.to_owned(),
            stage,
            character,
            layout,
            language,
        })
    }

    pub fn session(
        &self,
        disc: &Disc<impl ReadAt>,
        options: Options,
        alternate: bool,
        water_depth: i16,
    ) -> Result<Session> {
        let source = Game::new(
            MissionAssets::load_localized(
                &self.assets,
                self.stage,
                self.character,
                self.layout,
                self.language,
            )?,
            &Executable::new(&self.assets.executable)?,
            self.character,
            options.seed,
        )?;
        let (mut assets, layout) = Assets::gaisen_campaign(disc, &source, options.hero, alternate)?;
        assets.level.collision.set_gaisen_water_depth(water_depth)?;
        let bank = super::gaisen_bank(disc, &assets, options.hero)?;
        let mut world = World::new(&layout, options, &assets)?;
        world.instances[world.player].voice = 4;
        let audio = Audio::new(self, &source)?;
        let director = Director::new(source, &mut world, &assets)?;
        let mut session = Session::from_parts(world, assets, Arc::new(bank));
        session.gaisen = Some(Scene { director, audio });
        Ok(session)
    }
}

pub struct Scene {
    pub director: Director,
    pub audio: Audio,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct State<D> {
    director: D,
    current: Option<Track>,
    started: u64,
    tick: u64,
}

#[cfg(test)]
impl State<&Director> {
    pub(super) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            director,
            current,
            started,
            tick,
        } = self;
        director.capture(out)?;
        out.value(current)?;
        out.value(started)?;
        out.value(tick)
    }
}

impl Scene {
    pub(super) fn snapshot(&self) -> State<&Director> {
        State {
            director: &self.director,
            current: self.audio.current.clone(),
            started: self.audio.started,
            tick: self.audio.tick,
        }
    }
}

impl State<Director> {
    pub(super) fn reusable(&self, scene: &Scene) -> Result<bool> {
        self.validate(Some(scene))?;
        Ok(self.director.source.same_scene(&scene.director.source))
    }

    pub(super) fn swap(&mut self, scene: &mut Scene) {
        self.director.source.reuse_scene(&mut scene.director.source);
        std::mem::swap(&mut self.director, &mut scene.director);
        std::mem::swap(&mut self.current, &mut scene.audio.current);
        std::mem::swap(&mut self.started, &mut scene.audio.started);
        std::mem::swap(&mut self.tick, &mut scene.audio.tick);
    }

    pub(super) fn finish_reuse(&self, scene: &mut Scene) {
        if self.current != scene.audio.current {
            scene.audio.clip = None;
            scene.audio.loader.request(scene.audio.current.clone());
        }
        scene.audio.changed = true;
        scene.audio.sounds.clear();
    }

    pub(super) fn create(self, source: &Source) -> Result<Scene> {
        let game = &self.director.source;
        crate::savestate::validate_game(game)?;
        if game.mission.stage.id != source.stage
            || game.mission.layout_index >= 3
            || source.layout != 0 && Some(game.mission.layout_index) != source.layout.checked_sub(1)
            || game.mission.scripts.language != source.language
            || game
                .roster
                .actors
                .get(game.player_id)
                .is_none_or(Option::is_none)
            || game.player().kind != source.character
            || self.started > self.tick
        {
            return Err(
                "Savestate requires the same T1 stage, character, layout and language".into(),
            );
        }
        let mut audio = Audio::new(source, game)?;
        audio.current = self.current;
        audio.started = self.started;
        audio.tick = self.tick;
        audio.changed = true;
        audio.loader.request(audio.current.clone());
        Ok(Scene {
            director: self.director,
            audio,
        })
    }

    pub(super) fn validate(&self, current: Option<&Scene>) -> Result<()> {
        let current = current.ok_or("Open the matching T1 stage before restoring this state")?;
        let game = &self.director.source;
        crate::savestate::validate_game(game)?;
        let previous = &current.director.source;
        if game.mission.stage != previous.mission.stage
            || game.mission.layout_index != previous.mission.layout_index
            || game.mission.scripts.language != previous.mission.scripts.language
            || game
                .roster
                .actors
                .get(game.player_id)
                .is_none_or(Option::is_none)
            || game.player().kind != previous.player().kind
            || self.started > self.tick
        {
            return Err(
                "Savestate requires the same T1 stage, character, layout and language".into(),
            );
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn restore(self, scene: &mut Scene) {
        scene.director = self.director;
        scene.audio.current = self.current;
        scene.audio.started = self.started;
        scene.audio.tick = self.tick;
        scene.audio.clip = None;
        scene.audio.sounds.clear();
        scene.audio.changed = true;
        scene.audio.loader.request(scene.audio.current.clone());
    }
}

pub struct Audio {
    bank: Arc<Bank>,
    catalog: Catalog,
    loader: crate::audio::loader::Loader,
    current: Option<Track>,
    clip: Option<Arc<Clip>>,
    started: u64,
    tick: u64,
    changed: bool,
    pub sounds: Vec<Sourced<tenchu_core::audio::Sound>>,
}

impl Audio {
    fn new(source: &Source, game: &Game) -> Result<Self> {
        Ok(Self {
            bank: Arc::new(crate::audio::game_bank(&source.assets, game)?),
            catalog: Catalog::load(&source.assets.executable, source.language)?,
            loader: crate::audio::loader::Loader::new(&source.path)?,
            current: None,
            clip: None,
            started: 0,
            tick: 0,
            changed: false,
            sounds: Vec::new(),
        })
    }

    pub fn step(
        &mut self,
        game: &Game,
        listener: Option<tenchu_core::audio::Listener>,
    ) -> Result<()> {
        self.tick = game.tick;
        self.sounds = if let Some(listener) = listener {
            game.audio_with_listener(listener, game.effects.sounds.iter().copied(), None)?
        } else {
            game.audio_with_burning(game.effects.sounds.iter().copied(), None)?
        };
        for sound in &mut self.sounds {
            if sound.bank.is_none() {
                sound.bank = Some(self.bank.clone());
            }
        }
        for cue in &game.music {
            self.current = cue.and_then(|id| self.catalog.event(id)).cloned();
            self.loader.request(self.current.clone());
            self.clip = None;
            self.started = self.tick;
            self.changed = true;
        }
        Ok(())
    }

    pub fn synchronize(&mut self, output: Option<&Output>, muted: bool, restart: bool) {
        if let Some(result) = self.loader.poll() {
            match result {
                Ok((clip, _)) => {
                    self.clip = Some(clip);
                    self.changed = true;
                }
                Err(error) => eprintln!("Gaisen audio: {error}"),
            }
        }
        if (self.changed || restart)
            && let Some(output) = output
        {
            let repeat = self.current.as_ref().is_some_and(|track| track.repeat);
            output.send(Command::Music(
                if muted { None } else { self.clip.clone() },
                repeat,
            ));
            if !muted && self.clip.is_some() {
                output.send(Command::SeekMusic(
                    self.tick.saturating_sub(self.started) as f64 / 30.0,
                ));
            }
        }
        self.changed = false;
    }
}

impl Session {
    pub fn cinematic(&self) -> bool {
        self.gaisen
            .as_ref()
            .is_some_and(|scene| scene.director.presenting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires both discs; original XA tracks, effect banks, cancellation and seek survive muted cinematic playback"]
    fn imported_host_audio_uses_original_disc_and_native_player_bank() {
        let source = Source::open(
            std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
            0,
            0,
            1,
            Language::English,
        )
        .unwrap();
        let disc = open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut session = source
            .session(
                &disc,
                Options {
                    hero: 0,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
                4000,
            )
            .unwrap();
        assert_eq!(session.world.instances[session.world.player].voice, 4);
        let scene = session.gaisen.as_mut().unwrap();
        let (audio, game) = (&mut scene.audio, &mut scene.director.source);
        let (output, received) = Output::recording();
        for repeat in [false, true] {
            let cue = (0..200)
                .find(|id| audio.catalog.event(*id).is_some_and(|t| t.repeat == repeat))
                .unwrap();
            game.tick = 60;
            game.music = vec![Some(cue)];
            game.effects.sounds = vec![(game.player().position, 0x40)];
            audio.step(game, None).unwrap();
            assert!(!audio.sounds.is_empty());
            assert!(
                audio
                    .sounds
                    .iter()
                    .all(|s| Arc::ptr_eq(s.bank.as_ref().unwrap(), &audio.bank))
            );
            let expected =
                Clip::load(&source.assets.disc, audio.current.as_ref().unwrap()).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while audio.clip.is_none() {
                audio.synchronize(Some(&output), true, false);
                assert!(std::time::Instant::now() < deadline, "XA worker stalled");
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            assert_eq!(audio.clip.as_ref().unwrap().frames, expected.frames);
            assert!(
                received
                    .try_iter()
                    .all(|c| !matches!(c, Command::Music(Some(_), _)))
            );
            game.tick += 90;
            game.music.clear();
            audio.step(game, None).unwrap();
            audio.synchronize(Some(&output), false, true);
            let commands: Vec<_> = received.try_iter().collect();
            assert!(
                commands
                    .iter()
                    .any(|c| matches!(c, Command::Music(Some(_), r) if *r == repeat))
            );
            assert!(
                commands
                    .iter()
                    .any(|c| matches!(c, Command::SeekMusic(seconds) if *seconds == 3.0))
            );
            game.music = vec![None];
            audio.step(game, None).unwrap();
            audio.synchronize(Some(&output), false, false);
            assert!(matches!(
                received.try_recv().unwrap(),
                Command::Music(None, false)
            ));
            assert!(audio.current.is_none() && audio.clip.is_none());
        }
    }
}
