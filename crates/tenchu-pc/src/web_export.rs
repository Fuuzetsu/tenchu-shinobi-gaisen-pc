use crate::{
    campaign::Campaign,
    frontend::{Frontend, Preferences},
    session::Options,
    source,
    web_assets::{Manifest, movie_file, track_file},
};
use flate2::{Compression, write::GzEncoder};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};
use tenchu_assets::{
    ReadAt, Result,
    audio::{bank::Bank, catalog::Catalog, stream::Clip},
    cache::{Cache, PAGE_SIZE},
    frontend::Movie,
    game::GameAssets,
    script::Language,
};

const MOVIES: [Movie; 5] = [
    Movie::Opening,
    Movie::Training,
    Movie::Rikimaru,
    Movie::Ayame,
    Movie::Credits,
];
const LANGUAGES: [Language; 4] = [
    Language::English,
    Language::Japanese,
    Language::French,
    Language::Italian,
];

pub fn export(disc: &Path, destination: &Path) -> Result<()> {
    if destination.exists()
        && fs::read_dir(destination)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("Web asset destination must be empty".into());
    }
    for directory in ["pages", "audio", "movies"] {
        fs::create_dir_all(destination.join(directory)).map_err(|e| e.to_string())?;
    }
    let trace = Arc::new(Mutex::new(BTreeSet::new()));
    let assets = GameAssets::open(source::open_disc_recording(disc, Some(trace.clone()))?)?;
    let campaign = Campaign::new(&assets, Options::default(), None)?;
    let mut frontend = Frontend::new(&assets, disc, &campaign, Preferences::default())?;
    for language in LANGUAGES {
        frontend.language(&assets, language)?;
        for character in 0..2 {
            let options = Options {
                character,
                language: Some(language),
                ..Default::default()
            };
            let game = options.load(&assets)?;
            Bank::load(&assets, game.mission.stage.id, game.player().kind, language)?;
        }
        for movie in MOVIES {
            if let Some(path) = movie.subtitles(language) {
                assets.asset(&path)?;
            }
            if let Some(path) = movie.voices(language) {
                assets.asset(&path)?;
            }
        }
    }
    let boot = trace.lock().map_err(|e| e.to_string())?.clone();
    for name in [
        "TENCHU/MAIN.EXE",
        "TENCHU/MENU.EXE",
        "TENCHU/ENDING.EXE",
        "TENCHU/DATA.VOL",
    ] {
        let entry = assets.disc.find(name)?;
        assets.disc.file(&entry).read(0, entry.size as usize)?;
    }
    let rest: Vec<_> = trace
        .lock()
        .map_err(|e| e.to_string())?
        .difference(&boot)
        .copied()
        .collect();
    let raw = source::open_disc(disc)?;
    let mut manifest = Manifest {
        version: 1,
        length: raw.source().byte_len(),
        format: format!("{:?}", raw.format()),
        start_sector: raw.start_sector(),
        boot: vec![],
        game: vec![],
        audio: vec![],
        movies: vec![],
    };
    for (label, pages, files) in [
        (
            "boot",
            boot.into_iter().collect::<Vec<_>>(),
            &mut manifest.boot,
        ),
        ("game", rest, &mut manifest.game),
    ] {
        for (index, pages) in pages.chunks(64).enumerate() {
            let path = format!("pages/{label}-{index:03}.gz");
            let bytes = Cache::bundle(raw.source(), pages.iter().copied())?;
            let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
            gzip.write_all(&bytes).map_err(|e| e.to_string())?;
            fs::write(
                destination.join(&path),
                gzip.finish().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            files.push(path);
        }
        println!(
            "{label}: {} pages, {:.1} MiB uncompressed",
            pages.len(),
            pages.len() as f64 * PAGE_SIZE as f64 / 1048576.0
        );
    }
    let mut tracks = BTreeMap::new();
    for language in LANGUAGES {
        for track in Catalog::load(&assets.executable, language)?.tracks() {
            tracks
                .entry(track_file(track))
                .or_insert_with(|| track.clone());
        }
    }
    let ffmpeg = std::env::var_os("TENCHU_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    for (index, (path, track)) in tracks.iter().enumerate() {
        let clip = Clip::load(&raw, track)?;
        let mut encoder = Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "s16le",
                "-ar",
            ])
            .arg(clip.rate.to_string())
            .args([
                "-ac", "2", "-i", "pipe:0", "-c:a", "libopus", "-b:a", "128k", "-y",
            ])
            .arg(destination.join(path))
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut input = encoder.stdin.take().unwrap();
        let bytes: Vec<_> = clip
            .frames
            .iter()
            .flat_map(|f| f.iter().flat_map(|n| n.to_le_bytes()))
            .collect();
        input.write_all(&bytes).map_err(|e| e.to_string())?;
        drop(input);
        if !encoder.wait().map_err(|e| e.to_string())?.success() {
            return Err(format!("Encode {path}"));
        }
        manifest.audio.push(path.clone());
        if index % 25 == 0 {
            println!("Audio {index}/{}", tracks.len());
        }
    }
    for movie in MOVIES {
        let entry = raw.find(movie.path())?;
        let path = movie_file(movie);
        let mut encoder = Command::new(&ffmpeg)
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
                "-map",
                "0:a:0?",
                "-vf",
                "scale=320:240",
                "-c:v",
                "libvpx-vp9",
                "-crf",
                "15",
                "-b:v",
                "0",
                "-deadline",
                "good",
                "-cpu-used",
                "4",
                "-c:a",
                "libopus",
                "-b:a",
                "128k",
                "-y",
            ])
            .arg(destination.join(&path))
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut input = encoder.stdin.take().unwrap();
        let mut sector = [0; 2352];
        sector[1..11].fill(255);
        sector[15] = 2;
        for i in 0..entry.size.div_ceil(2048) {
            sector[16..].copy_from_slice(&raw.xa_sector(entry.sector + i)?);
            input.write_all(&sector).map_err(|e| e.to_string())?;
        }
        drop(input);
        if !encoder.wait().map_err(|e| e.to_string())?.success() {
            return Err(format!("Encode {path}"));
        }
        manifest.movies.push(path);
        println!("Movie {}", movie.path());
    }
    fs::write(
        destination.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("Prepared browser assets in {}", destination.display());
    Ok(())
}
