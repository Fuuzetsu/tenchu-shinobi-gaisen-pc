use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Seek},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};
use tenchu_assets::Result;

#[cfg(test)]
mod tests;

const MAX_BYTES: u64 = 512 * 1024 * 1024;
const READY: &str = ".tenchu-pack-ready";

#[derive(PartialEq)]
struct Stamp {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

pub(super) fn directory(path: &Path) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if path.is_dir() {
        return Ok(path);
    }
    if !path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".tar.zst")
    {
        return Err("An artwork pack must be a directory or a .tar.zst archive".into());
    }
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if metadata.len() > MAX_BYTES {
        return Err("Artwork archive exceeds 512 MiB".into());
    }
    let stamp = Stamp {
        path,
        size: metadata.len(),
        modified: metadata.modified().map_err(|e| e.to_string())?,
    };
    static LAST: Mutex<Option<(Stamp, PathBuf)>> = Mutex::new(None);
    let mut last = LAST.lock().map_err(|e| e.to_string())?;
    if let Some((previous, directory)) = &*last
        && *previous == stamp
        && complete(directory)
    {
        return Ok(directory.clone());
    }
    let mut hash = Sha256::new();
    io::copy(&mut file, &mut hash).map_err(|e| e.to_string())?;
    file.rewind().map_err(|e| e.to_string())?;
    let id = format!("{:x}", hash.finalize());
    let directory = unpack_cached(file, &cache_root()?, &id)?;
    *last = Some((stamp, directory.clone()));
    Ok(directory)
}

fn cache_root() -> Result<PathBuf> {
    if let Some(root) = std::env::var_os("TENCHU_ASSET_CACHE") {
        return Ok(root.into());
    }
    let root = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA")).map(PathBuf::from)
    } else if let Some(root) = std::env::var_os("XDG_CACHE_HOME") {
        Some(root.into())
    } else {
        std::env::var_os("HOME").map(|root| PathBuf::from(root).join(
            if cfg!(target_os = "macos") { "Library/Caches" } else { ".cache" }
        ))
    }.ok_or("Cannot find a cache directory; set TENCHU_ASSET_CACHE or use an unpacked artwork directory")?;
    Ok(root.join("tenchu-shinobi-gaisen-pc/packs"))
}

fn complete(root: &Path) -> bool {
    root.join(READY).is_file() && root.join("pack.json").is_file()
}

fn unpack_cached(source: impl Read, cache: &Path, id: &str) -> Result<PathBuf> {
    let destination = cache.join(id);
    if complete(&destination) {
        return Ok(destination);
    }
    fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = cache.join(format!(
        "{id}.{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        extract(source, &temporary)?;
        let manifest = fs::read(temporary.join("pack.json"))
            .map_err(|_| "Artwork archive must contain pack.json at its root")?;
        let manifest: super::Manifest =
            serde_json::from_slice(&manifest).map_err(|e| format!("Invalid pack.json: {e}"))?;
        if manifest.version != 1 {
            return Err("Unsupported artwork pack version".into());
        }
        fs::write(temporary.join(READY), id).map_err(|e| e.to_string())?;
        match fs::rename(&temporary, &destination) {
            Ok(()) => Ok(destination.clone()),
            Err(_) if complete(&destination) => Ok(destination.clone()),
            Err(e) => Err(format!("Cannot install artwork cache: {e}")),
        }
    })();
    if temporary.exists() {
        let _ = fs::remove_dir_all(temporary);
    }
    result
}

fn extract(source: impl Read, destination: &Path) -> Result<()> {
    let decoder = ruzstd::decoding::StreamingDecoder::new_with_max_window_size(
        BufReader::new(source),
        64 * 1024 * 1024,
    )
    .map_err(|e| format!("Invalid zstd artwork archive: {e}"))?;
    let mut decoded = decoder.take(MAX_BYTES + 1);
    let mut archive = tar::Archive::new(&mut decoded);
    let mut total = 0u64;
    for (index, entry) in archive.entries().map_err(|e| e.to_string())?.enumerate() {
        if index >= 8192 {
            return Err("Artwork archive exceeds 8192 entries".into());
        }
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("Artwork archives allow only regular files and directories; links are not supported".into());
        }
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let name = path.to_str().ok_or("Artwork archive paths must be UTF-8")?;
        let name = name
            .strip_prefix("./")
            .unwrap_or(name)
            .trim_end_matches('/');
        if name.is_empty() && kind.is_dir() {
            continue;
        }
        super::validate_path(name)?;
        let target = destination.join(name);
        let size = entry.size();
        total = total
            .checked_add(size)
            .ok_or("Artwork archive is too large")?;
        let limit = if name == "pack.json" {
            4 * 1024 * 1024
        } else {
            64 * 1024 * 1024
        };
        if size > limit || total > MAX_BYTES {
            return Err(format!("Artwork archive size limit exceeded at {name}"));
        }
        if kind.is_dir() {
            if size != 0 {
                return Err("Artwork directory contains file data".into());
            }
            fs::create_dir_all(target).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut file = File::create_new(&target).map_err(|e| format!("{name}: {e}"))?;
            io::copy(&mut entry, &mut file).map_err(|e| format!("{name}: {e}"))?;
        }
    }
    let mut padding = [0; 8192];
    loop {
        let n = decoded.read(&mut padding).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if padding[..n].iter().any(|&byte| byte != 0) {
            return Err("Unexpected data after artwork tar entries".into());
        }
    }
    if decoded.limit() == 0 {
        return Err("Artwork archive exceeds 512 MiB decompressed".into());
    }
    let decoder = decoded.into_inner();
    if let Some(expected) = decoder.decoder.get_checksum_from_data()
        && Some(expected) != decoder.decoder.get_calculated_checksum()
    {
        return Err("Artwork archive zstd checksum mismatch".into());
    }
    if decoder
        .into_inner()
        .read(&mut [0])
        .map_err(|e| e.to_string())?
        != 0
    {
        return Err("Artwork archive must contain a single zstd frame".into());
    }
    Ok(())
}
