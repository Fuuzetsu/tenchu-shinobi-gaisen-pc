use super::*;
use std::cell::Cell;

#[wasm_bindgen(module = "/web/recording.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn import_recording() -> std::result::Result<js_sys::Uint8Array, JsValue>;
    #[wasm_bindgen(catch)]
    fn export_recording(name: &str, bytes: &js_sys::Uint8Array)
    -> std::result::Result<(), JsValue>;
}

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static COMPLETED: RefCell<Option<Result<Vec<u8>>>> = const { RefCell::new(None) };
}

pub fn download(name: &str, bytes: &[u8]) -> Result<()> {
    export_recording(name, &js_sys::Uint8Array::from(bytes)).map_err(js_error)
}

pub fn start() -> Result<()> {
    if BUSY.replace(true) {
        return Err("A recording import is already in progress".into());
    }
    wasm_bindgen_futures::spawn_local(async {
        let result = import_recording()
            .await
            .map(|b| b.to_vec())
            .map_err(js_error);
        COMPLETED.with_borrow_mut(|pending| *pending = Some(result));
    });
    Ok(())
}

pub fn poll() -> Option<Result<Vec<u8>>> {
    let result = COMPLETED.with_borrow_mut(Option::take);
    if result.is_some() {
        BUSY.set(false);
    }
    result
}
