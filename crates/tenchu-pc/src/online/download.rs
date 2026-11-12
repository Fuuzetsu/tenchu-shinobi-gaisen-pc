use super::artwork::{Artwork, Release};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tenchu_assets::Result;

#[derive(Clone)]
pub enum Status {
    Connecting,
    Receiving(u64),
    Checking,
    Installed(PathBuf),
    Cancelled,
    Failed(String),
}

impl Status {
    pub fn busy(&self) -> bool {
        matches!(self, Self::Connecting | Self::Receiving(_) | Self::Checking)
    }
}

struct Task {
    status: Mutex<Status>,
    cancel: AtomicBool,
}

impl Task {
    fn set(&self, status: Status) {
        *self.status.lock().unwrap() = status;
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err("Download cancelled".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct Downloads {
    active: Mutex<Option<(Artwork, Arc<Task>)>>,
    generation: AtomicU64,
}

pub static DOWNLOADS: LazyLock<Downloads> = LazyLock::new(Downloads::default);

impl Downloads {
    pub fn snapshot(&self) -> Option<(Artwork, Status)> {
        self.active
            .lock()
            .unwrap()
            .as_ref()
            .map(|(artwork, task)| (*artwork, task.status.lock().unwrap().clone()))
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn cancel(&self) {
        if let Some((_, task)) = &*self.active.lock().unwrap() {
            task.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn use_local(&self, artwork: Artwork) {
        let result = artwork.directory().and_then(|directory| {
            match fs::remove_file(directory.join("current.json")) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        });
        let mut active = self.active.lock().unwrap();
        match result {
            Ok(()) => {
                *active = None;
                self.generation.fetch_add(1, Ordering::Release);
            }
            Err(error) => {
                *active = Some((
                    artwork,
                    Arc::new(Task {
                        status: Mutex::new(Status::Failed(error)),
                        cancel: AtomicBool::new(false),
                    }),
                ));
            }
        }
    }

    pub fn start(&'static self, artwork: Artwork) {
        let mut active = self.active.lock().unwrap();
        if active
            .as_ref()
            .is_some_and(|(_, task)| task.status.lock().unwrap().busy())
        {
            return;
        }
        let task = Arc::new(Task {
            status: Mutex::new(Status::Connecting),
            cancel: AtomicBool::new(false),
        });
        *active = Some((artwork, Arc::clone(&task)));
        std::thread::spawn(move || {
            let result = (|| {
                let directory = artwork.directory()?;
                let release = artwork.release();
                let body = super::http::get(&release.url(), std::time::Duration::from_secs(3600))?;
                if body
                    .content_length()
                    .is_some_and(|bytes| bytes != release.bytes)
                {
                    return Err("Download size does not match the published pack".into());
                }
                install(
                    body.into_reader(),
                    release,
                    &directory,
                    &task,
                    crate::visuals::pack::unpack,
                )
            })();
            match result {
                Ok(path) => {
                    task.set(Status::Installed(path));
                    self.generation.fetch_add(1, Ordering::Release);
                }
                Err(_) if task.cancel.load(Ordering::Relaxed) => task.set(Status::Cancelled),
                Err(error) => task.set(Status::Failed(error)),
            }
        });
    }
}

pub fn destination(directory: &Path, release: &Release) -> PathBuf {
    directory.join(format!("{}.tar.zst", release.sha256))
}

struct PartialFile(PathBuf);

impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn install(
    mut source: impl Read,
    release: &Release,
    directory: &Path,
    task: &Task,
    mut validate: impl FnMut(&Path) -> Result<PathBuf>,
) -> Result<PathBuf> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (temporary, mut file) = loop {
        let path = directory.join(format!(
            ".download-{}-{}.tar.zst",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => break (PartialFile(path), file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    };
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        task.check_cancelled()?;
        let count = source.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > release.bytes {
            return Err("Download exceeds the published pack size".into());
        }
        file.write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
        hash.update(&buffer[..count]);
        task.set(Status::Receiving(bytes));
    }
    if bytes != release.bytes || format!("{:x}", hash.finalize()) != release.sha256 {
        return Err("Incomplete or corrupt download: size / SHA-256 does not match".into());
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    task.set(Status::Checking);
    task.check_cancelled()?;
    validate(&temporary.0)?;
    task.check_cancelled()?;
    let destination = destination(directory, release);
    crate::storage::replace_file(&temporary.0, &destination).map_err(|e| e.to_string())?;
    // Prime the archive cache under its final path before the UI reloads it.
    validate(&destination)?;
    task.check_cancelled()?;
    let name = destination.file_name().unwrap().to_string_lossy();
    let selection = serde_json::to_vec(name.as_ref()).map_err(|e| e.to_string())?;
    crate::storage::atomic_write(&directory.join("current.json"), &selection)?;
    Ok(destination)
}

#[cfg(test)]
mod tests;
