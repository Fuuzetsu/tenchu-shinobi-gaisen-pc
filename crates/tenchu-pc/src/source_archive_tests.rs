use super::*;
use std::io::Write;

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "tenchu-disc-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const CUE: &[u8] = b"FILE \"track 1.bin\" BINARY\n TRACK 01 MODE2/2352\n INDEX 01 00:00:00\nFILE \"track 2.bin\" BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n";

fn files() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("disc/example.cue", CUE),
        ("disc/track 1.bin", b"data"),
        ("disc/track 2.bin", b"audio"),
    ]
}

fn write_archive(path: &Path, format: Format, files: &[(&str, &[u8])]) {
    let file = File::create(path).unwrap();
    if matches!(format, Format::Zip) {
        let mut zip = zip::ZipWriter::new(file);
        for &(name, data) in files {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    } else {
        let mut tar = tar::Builder::new(Vec::new());
        for &(name, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            tar.append_data(&mut header, name, data).unwrap();
        }
        let tar = tar.into_inner().unwrap();
        match format {
            Format::Tar => (&file).write_all(&tar).unwrap(),
            Format::Gzip => {
                let mut encoder = flate2::write::GzEncoder::new(file, Default::default());
                encoder.write_all(&tar).unwrap();
                encoder.finish().unwrap();
            }
            Format::Zstd => {
                let compressed = ruzstd::encoding::compress_to_vec(
                    tar.as_slice(),
                    ruzstd::encoding::CompressionLevel::Fastest,
                );
                (&file).write_all(&compressed).unwrap();
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn disc_archives_keep_all_tracks_and_reuse_atomic_cache() {
    for format in [Format::Zip, Format::Tar, Format::Gzip, Format::Zstd] {
        let root = Temporary::new();
        let archive = root.0.join("input");
        write_archive(&archive, format, &files());
        let cache = root.0.join("cache");
        let disc = unpack(File::open(&archive).unwrap(), format, &cache, "example").unwrap();
        assert_eq!(fs::read(&disc).unwrap(), CUE);
        assert_eq!(
            fs::read(disc.with_file_name("track 2.bin")).unwrap(),
            b"audio"
        );
        assert_eq!(find_disc(&cache.join("example")).unwrap(), disc);
        let stamp = fs::metadata(&disc).unwrap().modified().unwrap();
        assert_eq!(
            unpack(File::open(&archive).unwrap(), format, &cache, "example").unwrap(),
            disc
        );
        assert_eq!(fs::metadata(&disc).unwrap().modified().unwrap(), stamp);
        assert_eq!(fs::read_dir(cache).unwrap().count(), 1);
    }
}

#[test]
fn rejected_archives_never_install_partial_discs_or_escape() {
    for contents in [
        vec![("../escape.cue", CUE)],
        vec![("C:/escape.cue", CUE)],
        vec![("disc/example.cue", CUE)],
        vec![("one.bin", b"a".as_slice()), ("two.bin", b"b".as_slice())],
        vec![("one.cue", CUE), ("two.cue", CUE)],
        vec![("one.bin", b"a".as_slice()), ("ONE.BIN", b"b".as_slice())],
        vec![(READY, b"../outside".as_slice())],
    ] {
        let root = Temporary::new();
        let archive = root.0.join("input.zip");
        write_archive(&archive, Format::Zip, &contents);
        let cache = root.0.join("cache");
        assert!(unpack(File::open(&archive).unwrap(), Format::Zip, &cache, "bad").is_err());
        assert_eq!(fs::read_dir(cache).unwrap().count(), 0);
        assert!(!root.0.join("escape.cue").exists());
    }
}

#[test]
fn archive_symlinks_corrupt_streams_and_excessive_sizes_are_rejected() {
    let root = Temporary::new();
    let path = root.0.join("input");
    let mut tar = tar::Builder::new(File::create(&path).unwrap());
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    tar.append_link(&mut header, "link", "../outside").unwrap();
    tar.finish().unwrap();
    drop(tar);
    let cache = root.0.join("cache");
    assert!(unpack(File::open(&path).unwrap(), Format::Tar, &cache, "link").is_err());
    write_archive(&path, Format::Gzip, &files());
    let mut corrupt = fs::read(&path).unwrap();
    let crc = corrupt.len() - 8;
    corrupt[crc] ^= 1;
    fs::write(&path, corrupt).unwrap();
    assert!(unpack(File::open(&path).unwrap(), Format::Gzip, &cache, "corrupt").is_err());
    let mut output = Extractor {
        root: &root.0,
        total: 0,
        names: BTreeSet::new(),
    };
    assert!(
        output
            .entry("huge.bin", false, MAX_BYTES + 1, io::empty())
            .is_err()
    );
    assert!(!root.0.join("huge.bin").exists());
    assert_eq!(fs::read_dir(cache).unwrap().count(), 0);
}
