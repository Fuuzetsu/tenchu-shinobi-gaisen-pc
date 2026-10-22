use crate::{source::DiscSource, web_assets::Manifest};
use std::{cell::RefCell, path::Path, rc::Rc};
use tenchu_assets::{
    Result,
    audio::catalog::Catalog,
    cache::Cache,
    disc::{Disc, SectorFormat},
    game::GameAssets,
    script::Language,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/web/host.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn fetch_asset(path: &str, compressed: bool) -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn prefetch_media(paths: JsValue) -> std::result::Result<(), JsValue>;
    pub fn status(message: &str);
    pub fn graphics_backend(name: &str, adapter: &str);
    pub fn graphics_preference() -> String;
    pub fn frame(tick: u64, playing: bool, fault: &str);
    pub fn ready();
    pub fn closed();
    pub fn download_frame();
    pub fn loading_state(ready: bool);
    pub fn error(message: &str);
    pub fn media_pause(paused: bool);
    pub fn configure_game_audio(context: &web_sys::AudioContext);
    pub fn movie_volume(master: u16, music: u8);
    pub fn movie_stereo(stereo: bool);
    pub fn movie_error(video: &web_sys::HtmlVideoElement) -> String;
    #[wasm_bindgen(catch)]
    pub async fn audio_clip(path: &str) -> std::result::Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    pub fn movie_create(path: &str) -> std::result::Result<web_sys::HtmlVideoElement, JsValue>;
    #[wasm_bindgen(catch)]
    pub fn movie_pixels(video: &web_sys::HtmlVideoElement)
    -> std::result::Result<JsValue, JsValue>;
    pub fn movie_close(video: &web_sys::HtmlVideoElement);
}

struct State {
    source: DiscSource,
    format: SectorFormat,
    start: u64,
    ready: bool,
    movies: bool,
    audio: [bool; 4],
    pending: [bool; 4],
    failure: Option<String>,
    snapshot: String,
}

thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }

pub fn update_snapshot(campaign: &crate::campaign::Campaign, paused: bool, fault: &str) {
    use crate::campaign::Screen;
    let game = &campaign.game;
    let player = game.player();
    let screen = match campaign.screen {
        Screen::Frontend => "frontend",
        Screen::Ending(_) => "ending",
        Screen::Launcher => "launcher",
        Screen::Inventory(_) => "equipment",
        Screen::Playing => "playing",
        Screen::Results(_) => "results",
        Screen::GameOver => "game-over",
        Screen::Complete => "complete",
    };
    let data = serde_json::json!({ "tick": game.tick, "screen": screen, "paused": paused, "fault": fault,
        "mission": campaign.options.mission, "character": campaign.options.character, "health": player.health,
        "position": [player.position[0], player.position[1], player.position[2]], "motion": player.motion.action,
        "rng": game.random.seed });
    STATE.with_borrow_mut(|s| s.as_mut().unwrap().snapshot = data.to_string());
    frame(game.tick, campaign.playing(), fault);
}

#[wasm_bindgen]
pub fn snapshot() -> String {
    STATE.with_borrow(|s| {
        s.as_ref()
            .map_or_else(|| "{}".into(), |s| s.snapshot.clone())
    })
}

pub fn disc() -> Result<Disc<DiscSource>> {
    STATE.with_borrow(|state| {
        let s = state.as_ref().ok_or("Browser assets are not initialized")?;
        Disc::open(s.source.clone(), s.format, s.start)
    })
}

pub fn mission_ready(language: Language) -> Result<bool> {
    STATE.with_borrow(|state| {
        let s = state.as_ref().ok_or("Browser assets are not initialized")?;
        s.failure.as_ref().map_or(
            Ok(s.ready && s.movies && s.audio[language_index(language)]),
            |e| Err(e.clone()),
        )
    })
}

fn current(source: &DiscSource) -> bool {
    STATE.with_borrow(|s| {
        s.as_ref()
            .is_some_and(|s| Rc::ptr_eq(&s.source.0, &source.0))
    })
}

fn language_index(language: Language) -> usize {
    match language {
        Language::English => 0,
        Language::Japanese => 1,
        Language::French => 2,
        Language::Italian => 3,
    }
}

pub fn prepare_language(assets: &GameAssets<DiscSource>, language: Language) {
    loading_state(mission_ready(language).unwrap_or(false));
    let index = language_index(language);
    if !STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        if s.pending[index] {
            false
        } else {
            s.pending[index] = true;
            true
        }
    }) {
        return;
    }
    let catalog = match Catalog::load(&assets.executable, language) {
        Ok(c) => c,
        Err(e) => {
            error(&e);
            return;
        }
    };
    let paths = js_sys::Array::new();
    let files: std::collections::BTreeSet<_> = catalog
        .tracks()
        .map(crate::web_assets::track_file)
        .collect();
    for file in files {
        paths.push(&JsValue::from_str(&format!("data/{file}")));
    }
    let source = STATE.with_borrow(|s| s.as_ref().unwrap().source.clone());
    wasm_bindgen_futures::spawn_local(async move {
        let result = prefetch_media(paths.into()).await;
        if !current(&source) {
            return;
        }
        match result {
            Ok(()) => {
                STATE.with_borrow_mut(|s| s.as_mut().unwrap().audio[index] = true);
                status("Campaign audio ready");
            }
            Err(e) => {
                let e = js_error(e);
                STATE.with_borrow_mut(|s| s.as_mut().unwrap().failure = Some(e.clone()));
                status(&format!("Audio download failed: {e}. Reload to retry."));
            }
        }
    });
}

pub fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

async fn fetch(path: &str, compressed: bool) -> Result<Vec<u8>> {
    let bytes = fetch_asset(&format!("data/{path}"), compressed)
        .await
        .map_err(js_error)?;
    Ok(js_sys::Uint8Array::new(&bytes).to_vec())
}

#[wasm_bindgen(start)]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn play(quick: bool) -> std::result::Result<(), JsValue> {
    start(quick).await.map_err(|e| JsValue::from_str(&e))
}

async fn start(quick: bool) -> Result<()> {
    status("Preparing the original menu…");
    let manifest: Manifest =
        serde_json::from_slice(&fetch("manifest.json", false).await?).map_err(|e| e.to_string())?;
    if manifest.version != 1 {
        return Err("Unsupported browser asset manifest".into());
    }
    let format = match manifest.format.as_str() {
        "Iso" => SectorFormat::Iso,
        "Mode1" => SectorFormat::Mode1,
        "Mode2" => SectorFormat::Mode2,
        "Mode2_2336" => SectorFormat::Mode2_2336,
        _ => return Err("Invalid browser disc format".into()),
    };
    let source = DiscSource(Rc::new(RefCell::new(Cache::new(manifest.length)?)));
    for path in &manifest.boot {
        let bytes = fetch(path, true).await?;
        source.0.borrow_mut().insert(&bytes)?;
    }
    STATE.with_borrow_mut(|state| {
        *state = Some(State {
            source: source.clone(),
            format,
            start: manifest.start_sector,
            ready: false,
            movies: false,
            audio: [false; 4],
            pending: [false; 4],
            failure: None,
            snapshot: "{}".into(),
        })
    });
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            for (index, path) in manifest.game.iter().enumerate() {
                status(&format!(
                    "Preparing missions in the background: {}/{}",
                    index + 1,
                    manifest.game.len()
                ));
                let bytes = fetch(path, true).await?;
                if !current(&source) {
                    return Ok(());
                }
                source.0.borrow_mut().insert(&bytes)?;
            }
            STATE.with_borrow_mut(|s| s.as_mut().unwrap().ready = true);
            status("Mission data ready");
            let paths = js_sys::Array::new();
            for path in &manifest.movies {
                paths.push(&JsValue::from_str(&format!("data/{path}")));
            }
            prefetch_media(paths.into()).await.map_err(js_error)?;
            if !current(&source) {
                return Ok(());
            }
            STATE.with_borrow_mut(|s| s.as_mut().unwrap().movies = true);
            status("Movies ready");
            Ok::<_, String>(())
        }
        .await;
        if let Err(message) = result
            && current(&source)
        {
            STATE.with_borrow_mut(|s| s.as_mut().unwrap().failure = Some(message.clone()));
            status(&format!(
                "Asset download failed: {message}. Reload to retry."
            ));
        }
    });
    let assets = GameAssets::open(disc()?)?;
    crate::window::run(
        assets,
        crate::session::Options::default(),
        Path::new("browser"),
        None,
        None,
        crate::window::Startup {
            quick_launcher: quick,
            ..Default::default()
        },
    )
}

pub fn read_saved(path: &Path) -> std::io::Result<Vec<u8>> {
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .map_err(|e| std::io::Error::other(js_error(e)))?
        .ok_or_else(|| std::io::Error::other("Browser storage unavailable"))?;
    let key = format!("tenchu-pc/{}", path.display());
    let text = storage
        .get_item(&key)
        .map_err(|e| std::io::Error::other(js_error(e)))?
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
    serde_json::from_str(&text).map_err(std::io::Error::other)
}

pub fn write_saved(path: &Path, bytes: &[u8]) -> Result<()> {
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .map_err(js_error)?
        .ok_or("Browser storage unavailable")?;
    storage
        .set_item(
            &format!("tenchu-pc/{}", path.display()),
            &serde_json::to_string(bytes).map_err(|e| e.to_string())?,
        )
        .map_err(js_error)
}

#[wasm_bindgen]
pub fn validate_save(progress: &[u8], settings: &[u8]) -> std::result::Result<(), JsValue> {
    if !progress.is_empty() {
        tenchu_core::progress::save::Save::decode(progress).map_err(|e| JsValue::from_str(&e))?;
    }
    if !settings.is_empty() {
        serde_json::from_slice::<crate::settings::Settings>(settings)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }
    Ok(())
}
