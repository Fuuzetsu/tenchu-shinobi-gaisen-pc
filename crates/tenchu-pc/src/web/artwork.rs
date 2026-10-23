use super::*;
use std::collections::BTreeMap;

pub(super) struct Cache {
    pub ready: bool,
    files: BTreeMap<String, Vec<u8>>,
}

#[wasm_bindgen(module = "/web/host.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn fetch_artwork(paths: JsValue) -> std::result::Result<JsValue, JsValue>;
}

async fn fetch_files(paths: &[String]) -> Result<BTreeMap<String, Vec<u8>>> {
    let names = js_sys::Array::new();
    for name in paths {
        crate::visuals::pack::validate_path(name)?;
        names.push(&JsValue::from_str(name));
    }
    let buffers = js_sys::Array::from(&fetch_artwork(names.into()).await.map_err(js_error)?);
    if buffers.length() as usize != paths.len() {
        return Err("Incomplete artwork download".into());
    }
    Ok(paths
        .iter()
        .cloned()
        .zip(buffers.iter().map(|b| js_sys::Uint8Array::new(&b).to_vec()))
        .collect())
}

pub(super) async fn boot(manifest: Option<&crate::web_assets::Artwork>) -> Result<Cache> {
    let mut cache = Cache {
        ready: manifest.is_none(),
        files: BTreeMap::new(),
    };
    if let Some(manifest) = manifest {
        for (index, paths) in manifest.boot.chunks(6).enumerate() {
            status(&format!(
                "Preparing menu artwork: {}/{}",
                (index * 6 + paths.len()),
                manifest.boot.len()
            ));
            cache.files.extend(fetch_files(paths).await?);
        }
        if !cache.files.contains_key("pack.json") {
            return Err("Browser artwork is missing pack.json".into());
        }
    }
    Ok(cache)
}

pub(super) async fn finish(
    manifest: Option<crate::web_assets::Artwork>,
    source: &DiscSource,
) -> Result<()> {
    if let Some(manifest) = manifest {
        for (index, paths) in manifest.game.chunks(6).enumerate() {
            status(&format!(
                "Preparing mission artwork in the background: {}/{}",
                index * 6 + paths.len(),
                manifest.game.len()
            ));
            let files = fetch_files(paths).await?;
            if !current(source) {
                return Ok(());
            }
            STATE.with_borrow_mut(|s| s.as_mut().unwrap().artwork.files.extend(files));
        }
    }
    if current(source) {
        STATE.with_borrow_mut(|s| s.as_mut().unwrap().artwork.ready = true);
    }
    Ok(())
}

pub(crate) fn manifest(mission: bool) -> Result<Option<Vec<u8>>> {
    STATE.with_borrow(|s| {
        let s = s.as_ref().ok_or("Browser assets are not initialized")?;
        Ok(if mission && !s.artwork.ready {
            None
        } else {
            s.artwork.files.get("pack.json").cloned()
        })
    })
}

pub(crate) fn file(name: &str, limit: u64) -> Result<Vec<u8>> {
    crate::visuals::pack::validate_path(name)?;
    STATE.with_borrow(|s| {
        let s = s.as_ref().ok_or("Browser assets are not initialized")?;
        let bytes = s
            .artwork
            .files
            .get(name)
            .ok_or_else(|| format!("Artwork not loaded: {name}"))?;
        if bytes.len() as u64 > limit {
            return Err(format!("{name} exceeds its file size limit"));
        }
        Ok(bytes.clone())
    })
}
