use super::*;
use ruzstd::encoding::{CompressionLevel, compress_to_vec};
use tar::{EntryType, Header};

fn archive(entries: &[(&str, EntryType, &[u8])]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for &(name, kind, data) in entries {
        let mut header = Header::new_ustar();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(kind);
        // Raw headers allow traversal cases rejected by the tar writer's path API.
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_cksum();
        tar.append(&header, data).unwrap();
    }
    compress_to_vec(
        tar.into_inner().unwrap().as_slice(),
        CompressionLevel::Fastest,
    )
}

struct Temporary(PathBuf);

impl Temporary {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "tenchu-archive-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn extraction_is_atomic_and_reuses_the_complete_cache() {
    let temporary = Temporary::new();
    let bytes = archive(&[
        ("./", EntryType::Directory, b""),
        ("./pack.json", EntryType::Regular, br#"{"version":1}"#),
        (
            "./textures/test.png",
            EntryType::Regular,
            b"synthetic image data",
        ),
    ]);
    assert!(unpack_cached(&bytes[..bytes.len() - 3], &temporary.0, "pack").is_err());
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
    let root = unpack_cached(bytes.as_slice(), &temporary.0, "pack").unwrap();
    assert!(complete(&root));
    assert_eq!(
        fs::read(root.join("textures/test.png")).unwrap(),
        b"synthetic image data"
    );
    assert_eq!(
        unpack_cached(io::empty(), &temporary.0, "pack").unwrap(),
        root
    );
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 1);
}

#[test]
fn unsafe_paths_links_duplicates_and_oversized_files_are_rejected() {
    let temporary = Temporary::new();
    for (i, entries) in [
        vec![("../outside", EntryType::Regular, &b""[..])],
        vec![("/absolute", EntryType::Regular, b"")],
        vec![("C:/drive", EntryType::Regular, b"")],
        vec![("textures/NUL.png", EntryType::Regular, b"")],
        vec![("textures/test.png:stream", EntryType::Regular, b"")],
        vec![("link", EntryType::Symlink, b"")],
        vec![("link", EntryType::Link, b"")],
        vec![
            ("same", EntryType::Regular, b""),
            ("same", EntryType::Regular, b""),
        ],
    ]
    .iter()
    .enumerate()
    {
        assert!(unpack_cached(archive(entries).as_slice(), &temporary.0, &i.to_string()).is_err());
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
    }
    let mut header = Header::new_ustar();
    header.set_path("huge.png").unwrap();
    header.set_size(65 * 1024 * 1024);
    header.set_cksum();
    let bytes = compress_to_vec(&header.as_bytes()[..], CompressionLevel::Fastest);
    let error = unpack_cached(bytes.as_slice(), &temporary.0, "huge").unwrap_err();
    assert!(error.contains("size limit"), "{error}");
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn corrupt_archives_and_invalid_manifests_are_not_cached() {
    let temporary = Temporary::new();
    for (i, manifest) in [b"{}".as_slice(), br#"{"version":2}"#, b"broken"]
        .into_iter()
        .enumerate()
    {
        let bytes = archive(&[("pack.json", EntryType::Regular, manifest)]);
        assert!(unpack_cached(bytes.as_slice(), &temporary.0, &i.to_string()).is_err());
    }
    let good = archive(&[("pack.json", EntryType::Regular, br#"{"version":1}"#)]);
    let mut bad_checksum = good.clone();
    *bad_checksum.last_mut().unwrap() ^= 1;
    let error = unpack_cached(bad_checksum.as_slice(), &temporary.0, "checksum").unwrap_err();
    assert!(error.contains("checksum"), "{error}");
    let two_frames = [good.clone(), good.clone()].concat();
    assert!(unpack_cached(two_frames.as_slice(), &temporary.0, "frames").is_err());
    assert!(unpack_cached(&good[..good.len() / 2], &temporary.0, "truncated").is_err());
    assert!(unpack_cached(archive(&[]).as_slice(), &temporary.0, "empty").is_err());
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}
