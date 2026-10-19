use std::{
    cell::RefCell,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use tenchu_assets::{ReadAt, Result, check_span, cue, disc::Disc};

pub struct FileSource {
    file: RefCell<File>,
    length: u64,
}

impl FileSource {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        Ok(Self {
            file: RefCell::new(file),
            length,
        })
    }
}

impl ReadAt for FileSource {
    fn byte_len(&self) -> u64 {
        self.length
    }
    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        check_span(offset, output.len() as u64, self.length)?;
        let mut file = self.file.borrow_mut();
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        file.read_exact(output).map_err(|e| e.to_string())
    }
}

pub fn open_disc(path: &Path) -> Result<Disc<FileSource>> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue"))
    {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let track = cue::data_track(&text)?;
        let bin = path.parent().unwrap_or(Path::new(".")).join(track.file);
        Disc::open(FileSource::open(&bin)?, track.format, track.start_sector)
    } else {
        Disc::detect(FileSource::open(path)?)
    }
}
