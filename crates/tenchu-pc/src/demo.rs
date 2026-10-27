mod catalog;
pub mod framing;
pub mod overlay;
mod presentation;
#[cfg(not(target_arch = "wasm32"))]
mod recorded;
pub mod remake;
#[cfg(test)]
mod tests;

pub use catalog::{Clip, Feature, Shot};
pub use presentation::{Scene, animate, camera, readout};
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;
use std::{path::PathBuf, time::Duration};
use tenchu_assets::Result;

pub struct Player {
    pub catalog: Vec<Clip>,
    pub playlist: Vec<usize>,
    pub selected: Vec<bool>,
    pub index: usize,
    pub frame: u64,
    pub simulation_ticks: u64,
    pub cinematic_active: bool,
    pub finished: bool,
    pub library: bool,
    pub captions: bool,
    pub transport: bool,
    pub repeat: bool,
    pub exit_when_finished: bool,
    pub muted: bool,
    pub video: Option<PathBuf>,
    pub scene: Scene,
    inputs: Option<Vec<u16>>,
    phase: u128,
    simulation_phase: u16,
    last_speed: u16,
}

impl Player {
    pub fn benchmark(seconds: u16) -> Result<Self> {
        let mut catalog = catalog::parse(include_bytes!("../data/benchmarks.json"))?;
        for clip in &mut catalog {
            clip.seconds = seconds;
        }
        let mut player = Self::new(catalog);
        player.captions = false;
        player.transport = false;
        player.muted = true;
        Ok(player)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn parse(args: &[OsString]) -> Result<Self> {
        let mut catalog = catalog::builtin()?;
        let mut clips = None;
        let mut duration = None;
        let mut repeat = false;
        let mut clean = false;
        let mut exit_when_finished = false;
        let mut muted = false;
        let mut video = None;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.to_str().ok_or("Invalid demo option")? {
                "--loop" => repeat = true,
                "--clean" => clean = true,
                "--exit" => exit_when_finished = true,
                "--mute" => muted = true,
                "--video" => {
                    if video.is_some() {
                        return Err("Choose one --video OUTPUT.mp4".into());
                    }
                    video = Some(PathBuf::from(
                        args.next().ok_or("--video needs OUTPUT.mp4")?,
                    ));
                }
                "--clips" => {
                    clips = Some(
                        args.next()
                            .ok_or("--clips needs comma-separated clip IDs")?
                            .to_str()
                            .ok_or("Invalid clip IDs")?
                            .to_owned(),
                    );
                }
                "--duration" => {
                    duration = Some(
                        args.next()
                            .and_then(|a| a.to_str())
                            .and_then(|a| a.parse::<u16>().ok())
                            .filter(|n| (2..=120).contains(n))
                            .ok_or("--duration needs 2–120 seconds per clip")?,
                    );
                }
                "--catalog" => {
                    let path = args.next().ok_or("--catalog needs a JSON filename")?;
                    let bytes = std::fs::read(path).map_err(|e| format!("Demo catalog: {e}"))?;
                    catalog = catalog::parse(&bytes)?;
                    let parent = std::path::Path::new(path).parent().unwrap();
                    for clip in &mut catalog {
                        if let Some(recording) = &mut clip.recording {
                            recording.path = parent.join(&recording.path);
                        }
                    }
                }
                other => return Err(format!("Unknown demo option {other}")),
            }
        }
        if repeat && exit_when_finished {
            return Err("Choose --loop or --exit, not both".into());
        }
        if video.is_some() && repeat {
            return Err("Video exports play the selected clips once; omit --loop".into());
        }
        if let Some(seconds) = duration {
            for clip in &mut catalog {
                clip.seconds = seconds;
            }
        }
        let playlist = if let Some(clips) = clips {
            clips
                .split(',')
                .map(|id| {
                    catalog
                        .iter()
                        .position(|c| c.id == id.trim())
                        .ok_or_else(|| format!("Unknown demo clip '{id}'; use demo --list"))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            (0..catalog.len()).collect()
        };
        let mut player = Self::new(catalog);
        player.selected = (0..player.catalog.len())
            .map(|i| playlist.contains(&i))
            .collect();
        player.playlist = playlist;
        player.captions = !clean;
        player.transport = !clean && video.is_none();
        player.repeat = repeat;
        player.exit_when_finished = exit_when_finished;
        player.muted = muted || video.is_some();
        player.video = video;
        Ok(player)
    }

    fn new(catalog: Vec<Clip>) -> Self {
        Self {
            playlist: (0..catalog.len()).collect(),
            selected: vec![true; catalog.len()],
            catalog,
            index: 0,
            frame: 0,
            simulation_ticks: 0,
            cinematic_active: false,
            phase: 0,
            simulation_phase: 0,
            last_speed: 100,
            finished: false,
            library: false,
            captions: true,
            transport: true,
            repeat: false,
            exit_when_finished: false,
            muted: false,
            video: None,
            scene: Scene::default(),
            inputs: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn list() -> Result<()> {
        for clip in catalog::builtin()? {
            println!(
                "{:<18} {:>2}s  Mission {:>2}  {}\n    {}",
                clip.id, clip.seconds, clip.mission, clip.title, clip.description
            );
        }
        Ok(())
    }

    pub fn clip(&self) -> &Clip {
        &self.catalog[self.playlist[self.index]]
    }

    pub fn load_game(
        &mut self,
        assets: &tenchu_assets::game::GameAssets<crate::source::DiscSource>,
        language: Option<tenchu_assets::script::Language>,
    ) -> Result<(tenchu_core::game::Game, crate::session::Options)> {
        self.inputs = None;
        if self.clip().recording.is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            return self.load_recording(assets);
            #[cfg(target_arch = "wasm32")]
            {
                let recording = self.clip().recording.as_ref().unwrap();
                return Err(format!(
                    "Demo recording {} at tick {} requires the desktop host",
                    recording.path.display(),
                    recording.start_tick
                ));
            }
        }
        let options = crate::session::Options {
            mission: self.clip().mission,
            character: self.clip().character,
            language,
            ..Default::default()
        };
        let mut game = options.load(assets)?;
        if self.clip().feature != Feature::Cinematics {
            game.events = Default::default();
            for _ in 0..30 {
                game.step(0)?;
            }
        }
        if matches!(
            self.clip().feature,
            Feature::GrenadeLight | Feature::KaenLight
        ) {
            game.set_inventory(
                if self.clip().feature == Feature::GrenadeLight {
                    4
                } else {
                    17
                },
                255,
            )?;
        }
        Ok((game, options))
    }

    pub fn reset(&mut self, index: usize) {
        self.index = index.min(self.playlist.len() - 1);
        self.frame = 0;
        self.simulation_ticks = 0;
        self.phase = 0;
        self.simulation_phase = 0;
        self.last_speed = 100;
        self.finished = false;
    }

    pub fn next(&mut self) -> bool {
        if self.index + 1 < self.playlist.len() {
            self.reset(self.index + 1);
            true
        } else if self.repeat {
            self.reset(0);
            true
        } else {
            self.finished = true;
            self.phase = 0;
            false
        }
    }

    pub fn time(&self) -> f32 {
        self.frame as f32 / 60.0
    }
    pub fn progress(&self) -> f32 {
        (self.time() / f32::from(self.clip().seconds)).min(1.0)
    }

    pub fn elapse(&mut self, elapsed: Duration) {
        self.phase += elapsed.as_nanos() * 60;
    }

    pub fn queue_frame(&mut self) {
        self.phase = 1_000_000_000;
    }

    pub fn step(&mut self, cinematic: bool) -> Option<usize> {
        if self.phase < 1_000_000_000 || self.finished {
            return None;
        }
        self.phase -= 1_000_000_000;
        self.frame += 1;
        self.last_speed = if self.clip().feature == Feature::Cinematics && !cinematic {
            100
        } else {
            self.speed()
        };
        self.simulation_phase += self.last_speed;
        let ticks = self.simulation_phase / 200;
        self.simulation_phase %= 200;
        self.simulation_ticks += u64::from(ticks);
        Some(usize::from(ticks))
    }

    pub fn speed(&self) -> u16 {
        let p = self.progress();
        match self.clip().feature {
            Feature::Step => {
                if self.frame.is_multiple_of(30) {
                    200
                } else {
                    0
                }
            }
            Feature::Simulation => [25, 50, 100, 200, 400][section(p, 5)],
            Feature::Cinematics => [100, 400, 1600][section(p, 3)],
            _ => 100,
        }
    }

    pub fn alpha(&self) -> f32 {
        if self.clip().feature == Feature::Step {
            return 1.0;
        }
        ((f32::from(self.simulation_phase) + self.phase as f32 / 1e9 * f32::from(self.last_speed))
            / 200.0)
            .min(1.0)
    }

    pub fn ended(&self) -> bool {
        self.frame >= u64::from(self.clip().seconds) * 60
    }

    pub fn buttons(&self, tick: u64) -> u16 {
        if let Some(inputs) = &self.inputs {
            return inputs.get(tick as usize).copied().unwrap_or(0);
        }
        use tenchu_core::input::{R1, R2, SQUARE, TRIANGLE, UP};
        match self.clip().feature {
            Feature::Remake(feature) => remake::buttons(feature, tick, self.clip().seconds),
            Feature::Blood | Feature::BloodSnow => {
                if tick >= 60 && tick.is_multiple_of(35) {
                    SQUARE
                } else {
                    0
                }
            }
            Feature::SnowFootprints => {
                if tick < u64::from(self.clip().seconds) * 6 {
                    UP
                } else {
                    0
                }
            }
            Feature::GrenadeLight | Feature::KaenLight => {
                let half = u64::from(self.clip().seconds) * 15;
                if tick == 0 {
                    R2
                } else if tick == 2 || tick == half + 2 {
                    TRIANGLE
                } else {
                    0
                }
            }
            Feature::Interpolation
            | Feature::ModelDetail
            | Feature::CharacterRefinement
            | Feature::SecondaryMotion
            | Feature::PlayerModels
            | Feature::Step
            | Feature::Simulation => match tick % 120 {
                5 => SQUARE,
                50..=65 => R1,
                80 => SQUARE,
                _ => 0,
            },
            _ => 0,
        }
    }
}

pub fn section(progress: f32, count: usize) -> usize {
    (progress.clamp(0.0, 1.0) * count as f32)
        .floor()
        .min((count - 1) as f32) as usize
}

pub fn sweep(progress: f32) -> f32 {
    let t = ((progress - 0.12) / 0.76).clamp(0.0, 1.0);
    (1.0 - (t * std::f32::consts::TAU).cos()) * 0.5
}
