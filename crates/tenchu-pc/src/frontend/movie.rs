use super::raster::Canvas;
use crate::source::{DiscSource, open_disc};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tenchu_assets::{
    Result,
    audio::stream::{Clip, Stream, Track},
    font::Font,
    frontend::{Movie, movie_commands},
    game::GameAssets,
    script::Language,
    texture::Vram,
};
use tenchu_core::frontend::Captions;

struct Decoder {
    child: Child,
    frames: Option<Receiver<Result<Vec<u8>>>>,
    audio: Receiver<Result<Arc<Clip>>>,
    cancelled: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Decoder {
    fn new(path: &Path, movie: Movie) -> Result<Self> {
        let disc = open_disc(path)?;
        let entry = disc.find(movie.path())?;
        disc.xa_sector(entry.sector)?;
        let audio_disc = open_disc(path)?;
        let ffmpeg = std::env::var_os("TENCHU_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let mut child = Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "psxstr",
                "-i",
                "pipe:0",
                "-map",
                "0:v:0",
                "-an",
                "-vf",
                "scale=320:240",
                "-pix_fmt",
                "rgba",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                format!("Start movie decoder: {e}. Install FFmpeg or set TENCHU_FFMPEG.")
            })?;
        let mut input = child.stdin.take().unwrap();
        let mut output = child.stdout.take().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let (send, frames) = mpsc::sync_channel(4);
        let (sound, audio) = mpsc::channel();
        let active = cancelled.clone();
        let feeder = thread::spawn(move || {
            let mut raw = [0; 2352];
            raw[1..11].fill(255);
            raw[15] = 2;
            for i in 0..entry.size.div_ceil(2048) {
                if active.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(sector) = disc.xa_sector(entry.sector + i) else {
                    break;
                };
                raw[16..].copy_from_slice(&sector);
                if input.write_all(&raw).is_err() {
                    break;
                }
            }
        });
        let reader = thread::spawn(move || {
            for frame in 1..movie.last_frame() {
                let mut bytes = vec![0; 320 * 240 * 4];
                if let Err(error) = output.read_exact(&mut bytes) {
                    let _ = send.send(Err(format!(
                        "{} ended at frame {frame}: {error}",
                        movie.path()
                    )));
                    break;
                }
                if send.send(Ok(bytes)).is_err() {
                    break;
                }
            }
        });
        let active = cancelled.clone();
        let audio_worker = thread::spawn(move || {
            let result = (|| {
                let file = audio_disc.find(movie.path())?;
                let track = Track {
                    path: movie.path().into(),
                    channel: 1,
                    start: 0,
                    end: file.size.div_ceil(2048),
                    repeat: false,
                };
                let mut stream = Stream::open(&audio_disc, &track)?;
                let mut clip = Clip {
                    rate: 37800,
                    frames: vec![],
                };
                while !active.load(Ordering::Relaxed) {
                    let Some(sector) = stream.next(&audio_disc)? else {
                        break;
                    };
                    clip.rate = sector.rate;
                    clip.frames.extend(sector.frames);
                }
                Ok(Arc::new(clip))
            })();
            let _ = sound.send(result);
        });
        Ok(Self {
            child,
            frames: Some(frames),
            audio,
            cancelled,
            threads: vec![feeder, reader, audio_worker],
        })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.frames.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

pub struct Player {
    decoder: Decoder,
    movie: Movie,
    captions: Captions,
    pixels: Vec<u8>,
    next: Option<Vec<u8>>,
    audio: Option<Arc<Clip>>,
    started: bool,
    frame: u32,
    elapsed: f64,
    font: Font,
    vram: Vram,
}

#[derive(Default)]
pub struct Update {
    pub changed: bool,
    pub finished: bool,
    pub audio: Option<Arc<Clip>>,
    pub voices: Vec<i16>,
}

impl Player {
    pub fn new(
        path: &Path,
        assets: &GameAssets<DiscSource>,
        movie: Movie,
        language: Language,
    ) -> Result<Self> {
        let captions = movie
            .subtitles(language)
            .map(|p| assets.asset(&p).and_then(|b| movie_commands(&b)))
            .transpose()?
            .unwrap_or_default();
        let mut vram = Vram::default();
        let font = Font::load(assets, &mut vram)?;
        Ok(Self {
            decoder: Decoder::new(path, movie)?,
            movie,
            captions: Captions::new(captions),
            pixels: vec![0; 320 * 240 * 4],
            next: None,
            audio: None,
            started: false,
            frame: 0,
            elapsed: 0.0,
            font,
            vram,
        })
    }
    pub fn advance(&mut self, elapsed: Duration) -> Result<Update> {
        let mut update = Update::default();
        if self.next.is_none() && self.frame < self.movie.last_frame() - 1 {
            match self.decoder.frames.as_ref().unwrap().try_recv() {
                Ok(frame) => self.next = Some(frame?),
                Err(mpsc::TryRecvError::Empty) => (),
                Err(_) => return Err("Movie decoder disconnected".into()),
            }
        }
        if !self.started {
            if self.audio.is_none() {
                match self.decoder.audio.try_recv() {
                    Ok(audio) => self.audio = Some(audio?),
                    Err(mpsc::TryRecvError::Empty) => (),
                    Err(_) => return Err("Movie audio decoder disconnected".into()),
                }
            }
            if self.next.is_none() || self.audio.is_none() {
                return Ok(update);
            }
            self.started = true;
            update.audio = self.audio.take();
            self.elapsed = 1.0 / 15.0;
        } else {
            self.elapsed += elapsed.as_secs_f64();
        }
        while self.elapsed >= 1.0 / 15.0 {
            if self.next.is_none() && self.frame < self.movie.last_frame() - 1 {
                match self.decoder.frames.as_ref().unwrap().try_recv() {
                    Ok(frame) => self.next = Some(frame?),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(_) => return Err("Movie decoder disconnected".into()),
                }
            }
            if let Some(pixels) = self.next.take() {
                self.pixels = pixels;
                self.frame += 1;
                self.elapsed -= 1.0 / 15.0;
                update.changed = true;
                // DecodeMovieFrame returns the following STR frame's counter.
                update.voices.extend(self.captions.frame(self.frame + 1));
            } else if self.frame >= self.movie.last_frame() - 1 {
                update.finished = true;
                break;
            }
        }
        Ok(update)
    }
    pub fn paint(&mut self, canvas: &mut Canvas) {
        canvas.pixels.copy_from_slice(&self.pixels);
        super::captions::paint(canvas, &self.captions.text, &mut self.font, &mut self.vram);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_time::Instant;

    #[test]
    #[ignore = "requires TENCHU_DISC and FFmpeg; decodes every opening/training/ending frame and XA track"]
    fn all_movies_decode_to_the_source_endpoint_with_captions_and_audio() {
        let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(open_disc(path.as_ref()).unwrap()).unwrap();
        for movie in [
            Movie::Opening,
            Movie::Training,
            Movie::Rikimaru,
            Movie::Ayame,
            Movie::Credits,
        ] {
            let mut player = Player::new(path.as_ref(), &assets, movie, Language::English).unwrap();
            let start = Instant::now();
            let mut frames = 0;
            let mut voices = Vec::new();
            let mut audio_frames = 0;
            loop {
                let update = player.advance(Duration::from_secs_f64(1.0 / 15.0)).unwrap();
                if let Some(audio) = update.audio {
                    assert_eq!(audio.rate, 37800);
                    audio_frames = audio.frames.len();
                }
                voices.extend(update.voices);
                if update.changed {
                    frames = player.frame;
                    if [60, 300, 520].contains(&frames)
                        && let Some(path) = std::env::var_os("TENCHU_FRONTEND_CAPTURE")
                    {
                        let mut canvas = Canvas::new(320);
                        player.paint(&mut canvas);
                        crate::frontend::tests::capture(
                            &canvas,
                            Path::new(&path),
                            &format!("{movie:?}-{frames}"),
                        );
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
                if update.finished {
                    break;
                }
                assert!(
                    start.elapsed() < Duration::from_secs(90),
                    "{movie:?} decoder stalled at {}",
                    player.frame
                );
            }
            assert_eq!(frames, movie.last_frame() - 1);
            assert!(audio_frames as f64 / 37800.0 >= frames as f64 / 15.0);
            if movie != Movie::Opening {
                assert!(!voices.is_empty());
            }
            println!("{movie:?}: {frames} frames, {audio_frames} audio samples, voices {voices:?}");
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC and FFmpeg; compares movie cues at varied display cadences and cancels decoder workers"]
    fn display_cadence_and_early_exit_do_not_lose_movie_cues() {
        let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(open_disc(path.as_ref()).unwrap()).unwrap();
        let mut reference = None;
        for hz in [60.0, 15.0, 5.0] {
            let mut player =
                Player::new(path.as_ref(), &assets, Movie::Training, Language::French).unwrap();
            let start = Instant::now();
            let mut voices = Vec::new();
            let mut tracks = 0;
            loop {
                let update = player.advance(Duration::from_secs_f64(1.0 / hz)).unwrap();
                voices.extend(update.voices);
                tracks += usize::from(update.audio.is_some());
                if update.finished {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(30));
                if !update.changed {
                    thread::sleep(Duration::from_millis(1));
                }
            }
            let result = (player.frame, voices, tracks, player.captions.text.clone());
            if let Some(reference) = &reference {
                assert_eq!(&result, reference);
            } else {
                reference = Some(result);
            }
        }
        for _ in 0..8 {
            let player =
                Player::new(path.as_ref(), &assets, Movie::Opening, Language::English).unwrap();
            drop(player);
        }
    }
}
