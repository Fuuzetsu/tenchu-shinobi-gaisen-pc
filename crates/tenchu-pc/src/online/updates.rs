use semver::Version;
use serde::Deserialize;
use std::sync::{Arc, LazyLock, Mutex};
use tenchu_assets::Result;

const URL: &str =
    "https://api.github.com/repos/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases?per_page=30";
const RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
}

impl Release {
    pub fn url(&self) -> String {
        format!("{}/releases/tag/{}", super::REPOSITORY, self.tag)
    }
}

#[derive(Clone, Default)]
pub enum Status {
    #[default]
    NotChecked,
    Checking,
    Current,
    Available(Release),
    Failed(String),
}

#[derive(Default)]
pub struct Updates {
    status: Arc<Mutex<Status>>,
}

pub static UPDATES: LazyLock<Updates> = LazyLock::new(Updates::default);

impl Updates {
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    fn begin(&self) -> bool {
        let mut status = self.status.lock().unwrap();
        if !matches!(*status, Status::NotChecked) {
            return false;
        }
        *status = Status::Checking;
        true
    }

    pub fn start(&self, enabled: bool) {
        if !enabled || !self.begin() {
            return;
        }
        let status = Arc::clone(&self.status);
        let finish = move |result: Result<Vec<u8>>| {
            *status.lock().unwrap() =
                match result.and_then(|bytes| newer_release(&bytes, env!("CARGO_PKG_VERSION"))) {
                    Ok(Some(release)) => Status::Available(release),
                    Ok(None) => Status::Current,
                    Err(error) => Status::Failed(error),
                };
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || finish(fetch()));
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move { finish(fetch().await) });
    }
}

fn newer_release(bytes: &[u8], current: &str) -> Result<Option<Release>> {
    if bytes.len() > RESPONSE_LIMIT {
        return Err("Release information exceeds 2 MiB".into());
    }
    #[derive(Deserialize)]
    struct Entry {
        tag_name: String,
        draft: bool,
        prerelease: bool,
    }
    let entries: Vec<Entry> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let current = Version::parse(current).map_err(|e| e.to_string())?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            if entry.draft || entry.prerelease {
                return None;
            }
            let version = Version::parse(entry.tag_name.strip_prefix('v')?).ok()?;
            (version.pre.is_empty() && version > current).then_some(Release {
                tag: entry.tag_name,
                version,
            })
        })
        .max_by(|a, b| a.version.cmp(&b.version)))
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch() -> Result<Vec<u8>> {
    use std::io::Read;
    let body = super::http::get(URL, std::time::Duration::from_secs(20))?;
    let mut bytes = Vec::new();
    body.into_reader()
        .take(RESPONSE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
async fn fetch() -> Result<Vec<u8>> {
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window().ok_or("Browser window unavailable")?;
    let abort = web_sys::AbortController::new().map_err(|e| format!("{e:?}"))?;
    let options = web_sys::RequestInit::new();
    options.set_signal(Some(&abort.signal()));
    let timeout = Closure::<dyn FnMut()>::new(move || abort.abort());
    let timer = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout.as_ref().unchecked_ref(),
            20_000,
        )
        .map_err(|e| format!("{e:?}"))?;
    let result = async {
        let response = JsFuture::from(window.fetch_with_str_and_init(URL, &options))
            .await
            .map_err(|e| format!("{e:?}"))?
            .dyn_into::<web_sys::Response>()
            .map_err(|_| "Invalid release response")?;
        if !response.ok() {
            return Err(format!("GitHub returned HTTP {}", response.status()));
        }
        let bytes = JsFuture::from(response.array_buffer().map_err(|e| format!("{e:?}"))?)
            .await
            .map_err(|e| format!("{e:?}"))?;
        Ok(js_sys::Uint8Array::new(&bytes).to_vec())
    }
    .await;
    window.clear_timeout_with_handle(timer);
    result
}

#[cfg(test)]
mod tests;
