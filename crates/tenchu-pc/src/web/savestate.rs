use super::*;
use std::cell::Cell;

#[wasm_bindgen(module = "/web/state-store.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn state_write(slot: u8, bytes: js_sys::Uint8Array) -> std::result::Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn state_read(slot: u8) -> std::result::Result<js_sys::Uint8Array, JsValue>;
}

pub enum Completed {
    Saved(u8),
    Loaded(u8, Vec<u8>),
    Failed(String),
}

thread_local! {
    static BUSY: Cell<bool> = const { Cell::new(false) };
    static COMPLETED: RefCell<Option<Completed>> = const { RefCell::new(None) };
}

pub fn start(slot: u8, bytes: Option<Vec<u8>>) -> Result<()> {
    if BUSY.replace(true) {
        return Err("A savestate operation is already in progress".into());
    }
    wasm_bindgen_futures::spawn_local(async move {
        let result = if let Some(bytes) = bytes {
            state_write(slot, js_sys::Uint8Array::from(bytes.as_slice()))
                .await
                .map(|()| Completed::Saved(slot))
        } else {
            state_read(slot)
                .await
                .map(|bytes| Completed::Loaded(slot, bytes.to_vec()))
        };
        COMPLETED.with_borrow_mut(|pending| {
            *pending = Some(result.unwrap_or_else(|e| Completed::Failed(js_error(e))));
        });
    });
    Ok(())
}

pub fn poll() -> Option<Completed> {
    let result = COMPLETED.with_borrow_mut(Option::take);
    if result.is_some() {
        BUSY.set(false);
    }
    result
}
