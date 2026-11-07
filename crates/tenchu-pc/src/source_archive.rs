use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufReader, Read, Seek},
    path::{Component, Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};
use tenchu_assets::{Result, cue};

const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const READY: &str = ".tenchu-disc-ready";

#[derive(PartialEq)]
struct Stamp {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

#[derive(Clone, Copy)]
enum Format {
    Zip,
    Tar,
    Gzip,
    Zstd,
    Xz,
}

impl Format {
    fn for_path(path: &Path) -> Option<Self> {
        let name = path.to_string_lossy().to_ascii_lowercase();
        [
            (".zip", Self::Zip),
            (".tar", Self::Tar),
            (".tar.gz", Self::Gzip),
            (".tgz", Self::Gzip),
            (".tar.zst", Self::Zstd),
            (".tzst", Self::Zstd),
            (".tar.xz", Self::Xz),
            (".txz", Self::Xz),
        ]
        .into_iter()
        .find_map(|(suffix, kind)| name.ends_with(suffix).then_some(kind))
    }
}

pub fn resolve_disc(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        return find_disc(path);
    }
    let Some(format) = Format::for_path(path) else {
        return Ok(path.into());
    };
    let path = path
        .canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if metadata.len() > MAX_BYTES {
        return Err("Disc archive exceeds 2 GiB".into());
    }
    let stamp = Stamp {
        path,
        size: metadata.len(),
        modified: metadata.modified().map_err(|e| e.to_string())?,
    };
    static RECENT: Mutex<Vec<(Stamp, PathBuf)>> = Mutex::new(Vec::new());
    let mut recent = RECENT.lock().map_err(|e| e.to_string())?;
    if let Some((_, disc)) = recent
        .iter()
        .find(|(old, disc)| *old == stamp && disc.is_file())
    {
        return Ok(disc.clone());
    }
    let mut hash = Sha256::new();
    io::copy(&mut file, &mut hash).map_err(|e| e.to_string())?;
    file.rewind().map_err(|e| e.to_string())?;
    let cache = match std::env::var_os("TENCHU_DISC_CACHE") {
        Some(path) => PathBuf::from(path),
        None => crate::storage::cache_directory()?.join("discs"),
    };
    let disc = unpack(file, format, &cache, &format!("{:x}", hash.finalize()))?;
    if recent.len() == 8 {
        recent.remove(0);
    }
    recent.push((stamp, disc.clone()));
    Ok(disc)
}

fn find_disc(root: &Path) -> Result<PathBuf> {
    let mut pending = vec![(root.to_owned(), 0)];
    let (mut cues, mut images) = (Vec::new(), Vec::new());
    let mut count = 0;
    while let Some((directory, depth)) = pending.pop() {
        for entry in
            fs::read_dir(&directory).map_err(|e| format!("{}: {e}", directory.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            count += 1;
            if count > MAX_FILES {
                return Err("Disc folder exceeds 4096 entries".into());
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() && depth < 8 {
                pending.push((entry.path(), depth + 1));
            } else if kind.is_file() {
                let path = entry.path();
                match path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                {
                    Some("cue") => cues.push(path),
                    Some("bin" | "iso") => images.push(path),
                    _ => {}
                }
            }
        }
    }
    let mut choices = if cues.is_empty() { images } else { cues };
    match choices.len() {
        1 => Ok(choices.remove(0)),
        0 => Err("No CUE/BIN or ISO found in the disc folder/archive".into()),
        _ => Err("More than one disc found. Choose its CUE explicitly, or use a folder/archive containing one disc".into()),
    }
}

fn relative(name: &str) -> Result<PathBuf> {
    if name.len() > 1024 || name.contains(['\\', ':', '\0']) {
        return Err(format!("Invalid disc archive path: {name}"));
    }
    let path = Path::new(name);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!("Disc archive path leaves its folder: {name}"));
    }
    let path: PathBuf = path
        .components()
        .filter(|c| *c != Component::CurDir)
        .collect();
    if path.as_os_str().is_empty() {
        return Err("Empty disc archive path".into());
    }
    Ok(path)
}

fn validate_tracks(root: &Path, disc: &Path) -> Result<()> {
    if disc
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue"))
    {
        if fs::metadata(disc).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
            return Err("CUE exceeds 1 MiB".into());
        }
        let text = fs::read_to_string(disc).map_err(|e| e.to_string())?;
        cue::data_track(&text)?;
        for name in cue::binary_files(&text)? {
            let track = disc.parent().unwrap().join(relative(name)?);
            let track = track
                .canonicalize()
                .map_err(|e| format!("Missing CUE track {name}: {e}"))?;
            if !track.starts_with(root) || !track.is_file() {
                return Err(format!("CUE track outside disc archive: {name}"));
            }
        }
    }
    Ok(())
}

fn completed(root: &Path) -> Option<PathBuf> {
    let path = relative(&fs::read_to_string(root.join(READY)).ok()?).ok()?;
    let disc = root.join(path);
    disc.is_file().then_some(disc)
}

fn unpack(file: File, format: Format, cache: &Path, id: &str) -> Result<PathBuf> {
    fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let cache = cache.canonicalize().map_err(|e| e.to_string())?;
    let destination = cache.join(id);
    if let Some(disc) = completed(&destination) {
        return Ok(disc);
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temporary = cache.join(format!(
        "{id}.{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        extract(file, format, &temporary)?;
        let disc = find_disc(&temporary)?;
        validate_tracks(&temporary, &disc)?;
        let local = disc.strip_prefix(&temporary).map_err(|e| e.to_string())?;
        fs::write(
            temporary.join(READY),
            local.to_str().ok_or("Disc path must be UTF-8")?,
        )
        .map_err(|e| e.to_string())?;
        match fs::rename(&temporary, &destination) {
            Ok(()) => Ok(destination.join(local)),
            Err(_) if completed(&destination).is_some() => Ok(completed(&destination).unwrap()),
            Err(e) => Err(format!("Cannot install disc cache: {e}")),
        }
    })();
    if temporary.exists() {
        let _ = fs::remove_dir_all(temporary);
    }
    result
}

struct Extractor<'a> {
    root: &'a Path,
    total: u64,
    names: BTreeSet<String>,
}

impl Extractor<'_> {
    fn entry(
        &mut self,
        name: &str,
        directory: bool,
        size: u64,
        mut source: impl Read,
    ) -> Result<()> {
        if directory && matches!(name, "." | "./") {
            return Ok(());
        }
        let path = relative(name)?;
        if path == Path::new(READY) {
            return Err("Reserved disc archive filename".into());
        }
        if self.names.len() >= MAX_FILES
            || !self.names.insert(path.to_string_lossy().to_lowercase())
        {
            return Err("Duplicate or excessive disc archive entries".into());
        }
        self.total = self
            .total
            .checked_add(size)
            .ok_or("Disc archive size overflow")?;
        if self.total > MAX_BYTES || (directory && size != 0) {
            return Err("Disc archive exceeds 2 GiB or has invalid directory data".into());
        }
        let target = self.root.join(path);
        if directory {
            fs::create_dir_all(target).map_err(|e| e.to_string())?;
        } else {
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            let mut file = File::create_new(target).map_err(|e| format!("{name}: {e}"))?;
            if io::copy(&mut source.by_ref().take(size + 1), &mut file)
                .map_err(|e| format!("{name}: {e}"))?
                != size
            {
                return Err(format!("Disc archive size mismatch: {name}"));
            }
        }
        Ok(())
    }
}

fn extract(file: File, format: Format, root: &Path) -> Result<()> {
    let mut output = Extractor {
        root,
        total: 0,
        names: BTreeSet::new(),
    };
    if matches!(format, Format::Zip) {
        let mut archive = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| e.to_string())?;
        if archive.len() > MAX_FILES {
            return Err("Disc archive exceeds 4096 entries".into());
        }
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
            let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
            if !matches!(mode, 0 | 0o100000 | 0o040000) {
                return Err("Disc archives cannot contain links or special files".into());
            }
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            output.entry(&name, entry.is_dir(), entry.size(), &mut entry)?;
        }
    } else {
        let file = BufReader::new(file);
        let mut zstd = None;
        let reader: Box<dyn Read + '_> = match format {
            Format::Tar => Box::new(file),
            Format::Gzip => Box::new(flate2::read::MultiGzDecoder::new(file)),
            Format::Zstd => {
                zstd = Some(
                    ruzstd::decoding::StreamingDecoder::new_with_max_window_size(
                        file,
                        64 * 1024 * 1024,
                    )
                    .map_err(|e| e.to_string())?,
                );
                Box::new(zstd.as_mut().unwrap())
            }
            Format::Xz => Box::new(lzma_rust2::XzReader::new_mem_limit(file, true, 128 * 1024)),
            Format::Zip => unreachable!(),
        };
        let mut reader = reader.take(MAX_BYTES + 8 * 1024 * 1024);
        let mut archive = tar::Archive::new(&mut reader);
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err("Disc archives cannot contain links or special files".into());
            }
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            output.entry(
                path.to_str().ok_or("Disc archive paths must be UTF-8")?,
                kind.is_dir(),
                entry.size(),
                &mut entry,
            )?;
        }
        // Read to EOF so compression checksums and truncated streams are checked too.
        let mut padding = [0; 8192];
        loop {
            let n = reader.read(&mut padding).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if padding[..n].iter().any(|&b| b != 0) {
                return Err("Unexpected data after disc tar entries".into());
            }
        }
        if reader.limit() == 0 {
            return Err("Disc archive is too large".into());
        }
        drop(reader);
        if let Some(decoder) = zstd {
            if let Some(expected) = decoder.decoder.get_checksum_from_data()
                && Some(expected) != decoder.decoder.get_calculated_checksum()
            {
                return Err("Disc archive zstd checksum mismatch".into());
            }
            if decoder
                .into_inner()
                .read(&mut [0])
                .map_err(|e| e.to_string())?
                != 0
            {
                return Err("Disc archive must contain one zstd frame".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "source_archive_tests.rs"]
mod tests;
