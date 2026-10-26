use std::{
    cell::RefCell,
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex},
};
use tenchu_assets::{ReadAt, Result, cache::PAGE_SIZE, check_span, cue, disc::Disc};

pub struct DiscSource {
    file: RefCell<File>,
    length: u64,
    trace: Option<Arc<Mutex<BTreeSet<u32>>>>,
}

impl DiscSource {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        Ok(Self {
            file: RefCell::new(file),
            length,
            trace: None,
        })
    }
}

impl ReadAt for DiscSource {
    fn byte_len(&self) -> u64 {
        self.length
    }
    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        check_span(offset, output.len() as u64, self.length)?;
        if let Some(trace) = &self.trace {
            let mut pages = trace.lock().map_err(|e| e.to_string())?;
            for page in
                offset / PAGE_SIZE as u64..(offset + output.len() as u64).div_ceil(PAGE_SIZE as u64)
            {
                pages.insert(page as u32);
            }
        }
        let mut file = self.file.borrow_mut();
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        file.read_exact(output).map_err(|e| e.to_string())
    }
}

pub fn open_disc(path: &Path) -> Result<Disc<DiscSource>> {
    open_disc_recording(path, None)
}

pub fn open_disc_recording(
    path: &Path,
    trace: Option<Arc<Mutex<BTreeSet<u32>>>>,
) -> Result<Disc<DiscSource>> {
    let source = |path: &Path| -> Result<DiscSource> {
        let mut source = DiscSource::open(path)?;
        source.trace = trace.clone();
        Ok(source)
    };
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue"))
    {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let track = cue::data_track(&text)?;
        let bin = path.parent().unwrap_or(Path::new(".")).join(track.file);
        Disc::open(source(&bin)?, track.format, track.start_sector)
    } else {
        Disc::detect(source(path)?)
    }
}
