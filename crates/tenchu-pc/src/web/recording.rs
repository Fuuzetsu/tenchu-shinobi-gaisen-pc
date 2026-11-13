use super::*;
use std::cell::Cell;

#[wasm_bindgen(module = "/web/recording.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn import_recording(
        extension: &str,
        limit: usize,
    ) -> std::result::Result<js_sys::Uint8Array, JsValue>;
    #[wasm_bindgen(catch)]
    fn export_recording(name: &str, bytes: &js_sys::Uint8Array)
    -> std::result::Result<(), JsValue>;
}

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static COMPLETED: RefCell<Option<(Kind, Result<Vec<u8>>)>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub enum Kind {
    Recording,
    Unarmed,
    Studio,
    StudioAsset(crate::level_editor::custom::Kind),
}

pub fn download(name: &str, bytes: &[u8]) -> Result<()> {
    export_recording(name, &js_sys::Uint8Array::from(bytes)).map_err(js_error)
}

pub fn start(kind: Kind) -> Result<()> {
    if BUSY.replace(true) {
        return Err("A file import is already in progress".into());
    }
    wasm_bindgen_futures::spawn_local(async move {
        let (extension, limit) = match kind {
            Kind::Recording => (".treplay", 128 * 1024 * 1024),
            Kind::Unarmed => (".t2moves", crate::sheathing::imports::LIMIT),
            Kind::StudioAsset(kind) => (
                match kind {
                    crate::level_editor::custom::Kind::Texture => ".png",
                    crate::level_editor::custom::Kind::Model => ".obj",
                    crate::level_editor::custom::Kind::Audio => ".wav,.ogg,.mp3,.flac",
                },
                crate::level_editor::custom::FILE_LIMIT,
            ),
            Kind::Studio => (
                ".tenchu,.json,.msn,.mcs",
                crate::level_editor::Project::MAX_BYTES,
            ),
        };
        let result = import_recording(extension, limit)
            .await
            .map(|b| b.to_vec())
            .map_err(js_error);
        COMPLETED.with_borrow_mut(|pending| *pending = Some((kind, result)));
    });
    Ok(())
}

pub fn poll() -> Option<(Kind, Result<Vec<u8>>)> {
    let result = COMPLETED.with_borrow_mut(Option::take);
    if result.is_some() {
        BUSY.set(false);
    }
    result
}
