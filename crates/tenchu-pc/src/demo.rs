mod catalog;
pub mod framing;
pub mod overlay;
mod presentation;
#[cfg(test)]
mod tests;

pub use catalog::{Clip, Feature, Shot};
pub use presentation::{Scene, animate, camera, readout};
use std::{ffi::OsString, path::PathBuf, time::Duration};
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
    phase: u128,
    simulation_phase: u16,
    last_speed: u16,
}

impl Player {
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
        let selected = (0..catalog.len()).map(|i| playlist.contains(&i)).collect();
        Ok(Self {
            catalog,
            playlist,
            selected,
            index: 0,
            frame: 0,
            simulation_ticks: 0,
            cinematic_active: false,
            phase: 0,
            simulation_phase: 0,
            last_speed: 100,
            finished: false,
            library: false,
            captions: !clean,
            transport: !clean && video.is_none(),
            repeat,
            exit_when_finished,
            muted: muted || video.is_some(),
            video,
            scene: Scene::default(),
        })
    }

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
        use tenchu_core::input::{R1, SQUARE};
        match self.clip().feature {
            Feature::Interpolation
            | Feature::ModelDetail
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
