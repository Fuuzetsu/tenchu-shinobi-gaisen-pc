use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tenchu_assets::{audio::stream::Track, frontend::Movie};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub length: u64,
    pub format: String,
    pub start_sector: u64,
    pub boot: Vec<String>,
    pub game: Vec<String>,
    pub audio: Vec<String>,
    pub movies: Vec<String>,
}

pub fn track_file(track: &Track) -> String {
    let key = format!(
        "{}:{}:{}:{}",
        track.path, track.channel, track.start, track.end
    );
    format!("audio/{:x}.ogg", Sha256::digest(key.as_bytes()))
}

pub fn movie_file(movie: Movie) -> String {
    let name = movie
        .path()
        .rsplit('/')
        .next()
        .unwrap()
        .trim_end_matches(".STR");
    format!("movies/{name}.webm")
}
