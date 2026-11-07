use crate::source::{DiscSource, open_disc};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};
use tenchu_assets::{
    Result,
    audio::stream::{Clip, Stream, Track},
    disc::Disc,
};

type Loaded = Result<(Arc<Clip>, bool)>;

#[derive(Clone, PartialEq, Eq)]
struct Request {
    disc: Option<PathBuf>,
    track: Track,
}

pub struct Loader {
    generation: Arc<AtomicU64>,
    commands: Sender<(u64, Request)>,
    results: Receiver<(u64, Loaded)>,
}

impl Loader {
    pub fn new(path: &Path) -> Result<Self> {
        let disc = open_disc(path)?;
        let generation = Arc::new(AtomicU64::new(0));
        let active = generation.clone();
        let (commands, receiver) = mpsc::channel::<(u64, Request)>();
        let (results, loaded) = mpsc::channel();
        thread::Builder::new()
            .name("tenchu-audio".into())
            .spawn(move || {
                let mut cache: VecDeque<(Request, Arc<Clip>)> = VecDeque::new();
                let mut imported = None;
                while let Ok((serial, request)) = receiver.recv() {
                    if active.load(Ordering::Relaxed) != serial {
                        continue;
                    }
                    let result =
                        if let Some(index) = cache.iter().position(|(key, _)| key == &request) {
                            let entry = cache.remove(index).unwrap();
                            let clip = entry.1.clone();
                            cache.push_back(entry);
                            Ok(Some(clip))
                        } else {
                            request_disc(&disc, &mut imported, request.disc.as_deref())
                                .and_then(|source| decode(source, &request.track, &active, serial))
                                .map(|clip| clip.map(Arc::new))
                        };
                    let result = match result {
                        Ok(None) => continue,
                        Ok(Some(clip)) => {
                            if !cache.iter().any(|(key, _)| key == &request) {
                                let size = clip.frames.len() * 4;
                                while !cache.is_empty()
                                    && cache
                                        .iter()
                                        .map(|(_, clip)| clip.frames.len() * 4)
                                        .sum::<usize>()
                                        + size
                                        > 64 * 1024 * 1024
                                {
                                    cache.pop_front();
                                }
                                if size <= 64 * 1024 * 1024 {
                                    cache.push_back((request.clone(), clip.clone()));
                                }
                            }
                            Ok((clip, request.track.repeat))
                        }
                        Err(error) => Err(error),
                    };
                    if results.send((serial, result)).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            generation,
            commands,
            results: loaded,
        })
    }

    pub fn request(&mut self, track: Option<Track>) {
        self.request_from(track, None);
    }

    pub fn request_from(&mut self, track: Option<Track>, disc: Option<&Path>) {
        let serial = self
            .generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        if let Some(track) = track {
            let _ = self.commands.send((
                serial,
                Request {
                    disc: disc.map(Path::to_path_buf),
                    track,
                },
            ));
        }
    }

    pub fn poll(&self) -> Option<Loaded> {
        let mut latest = None;
        while let Ok((serial, result)) = self.results.try_recv() {
            if self.generation.load(Ordering::Relaxed) == serial {
                latest = Some(result);
            }
        }
        latest
    }
}

fn request_disc<'a>(
    main: &'a Disc<DiscSource>,
    imported: &'a mut Option<(PathBuf, Disc<DiscSource>)>,
    path: Option<&Path>,
) -> Result<&'a Disc<DiscSource>> {
    let Some(path) = path else {
        return Ok(main);
    };
    if imported.as_ref().is_none_or(|(current, _)| current != path) {
        *imported = Some((path.to_owned(), open_disc(path)?));
    }
    Ok(&imported.as_ref().unwrap().1)
}

impl Drop for Loader {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }
}

fn decode(
    disc: &Disc<DiscSource>,
    track: &Track,
    generation: &AtomicU64,
    serial: u64,
) -> Result<Option<Clip>> {
    let mut stream = Stream::open(disc, track)?;
    let mut clip = Clip {
        rate: 37800,
        frames: Vec::new(),
    };
    loop {
        if generation.load(Ordering::Relaxed) != serial {
            return Ok(None);
        }
        let Some(sector) = stream.next(disc)? else {
            break;
        };
        clip.rate = sector.rate;
        clip.frames.extend(sector.frames);
    }
    if clip.frames.is_empty() && track.start != track.end {
        return Err(format!(
            "No XA audio in {} channel {}",
            track.path, track.channel
        ));
    }
    Ok(Some(clip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use tenchu_assets::{audio::catalog::Catalog, game::GameAssets, script::Language};

    #[test]
    #[ignore = "requires both discs; cancellation and cache identity across disc sources"]
    fn imported_music_does_not_leak_into_the_main_disc_cache() {
        let path = std::env::var_os("TENCHU_DISC").unwrap();
        let t2 = std::env::var_os("TENCHU2_DISC").unwrap();
        let disc = open_disc(t2.as_ref()).unwrap();
        let mut track = tenchu_assets::tenchu2::music::tracks(&disc)
            .unwrap()
            .remove(0);
        let mut loader = Loader::new(path.as_ref()).unwrap();
        loader.request_from(Some(track.clone()), Some(t2.as_ref()));
        loader.request(None);
        track.end = 32;
        let expected = Clip::load(&disc, &track).unwrap();
        let wait = |loader: &Loader| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if let Some(result) = loader.poll() {
                    return result;
                }
                assert!(Instant::now() < deadline, "Audio worker stalled");
                thread::sleep(Duration::from_millis(2));
            }
        };
        loader.request_from(Some(track.clone()), Some(t2.as_ref()));
        assert_eq!(wait(&loader).unwrap().0.frames, expected.frames);
        loader.request(Some(track.clone()));
        assert!(
            wait(&loader).is_err(),
            "Used imported cache for the wrong disc"
        );
        loader.request_from(Some(track), Some(t2.as_ref()));
        assert_eq!(wait(&loader).unwrap().0.frames, expected.frames);
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; checks asynchronous XA cancellation and replacement"]
    fn cancelled_decode_cannot_replace_the_latest_cue() {
        let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(open_disc(path.as_ref()).unwrap()).unwrap();
        let catalog = Catalog::load(&assets.executable, Language::English).unwrap();
        let mut loader = Loader::new(path.as_ref()).unwrap();
        let long = catalog.music(0).unwrap().clone();
        for index in 0..8 {
            loader.request(Some(long.clone()));
            loader.request(None);
            let mut short = catalog.music(11).unwrap().clone();
            short.end = short.start + 24 + index * 8;
            short.repeat = false;
            let expected = Clip::load(&assets.disc, &short).unwrap();
            loader.request(Some(short));
            let deadline = Instant::now() + Duration::from_secs(10);
            let (clip, repeat) = loop {
                if let Some(result) = loader.poll() {
                    break result.unwrap();
                }
                assert!(Instant::now() < deadline, "Audio worker stalled");
                thread::sleep(Duration::from_millis(2));
            };
            assert!(!repeat);
            assert_eq!(clip.rate, expected.rate);
            assert_eq!(clip.frames, expected.frames);
        }
    }
}
