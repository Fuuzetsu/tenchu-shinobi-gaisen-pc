use super::*;

fn task() -> Task {
    Task {
        status: Mutex::new(Status::Connecting),
        cancel: AtomicBool::new(false),
    }
}

fn release(bytes: &[u8]) -> Release {
    Release {
        tag: "fixture".into(),
        archive: "fixture.tar.zst".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: bytes.len() as u64,
    }
}

#[test]
fn failed_or_cancelled_downloads_preserve_the_active_pack_and_remove_partial_files() {
    let directory =
        std::env::temp_dir().join(format!("tenchu-download-errors-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let old = directory.join("old.tar.zst");
    fs::write(&old, b"old pack").unwrap();
    fs::write(directory.join("current.json"), b"\"old.tar.zst\"").unwrap();
    let bytes = b"new archive";
    let release = release(bytes);
    for input in [
        b"new".as_slice(),
        b"bad archive".as_slice(),
        b"new archive plus extra".as_slice(),
    ] {
        let error = install(input, &release, &directory, &task(), |_| {
            panic!("unverified archive reached extraction")
        })
        .unwrap_err();
        assert!(error.contains("size") || error.contains("SHA-256"));
        assert_eq!(
            super::super::artwork::installed_at(&directory),
            Some(old.clone())
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    }
    let cancelled = task();
    cancelled.cancel.store(true, Ordering::Relaxed);
    assert!(
        install(
            bytes.as_slice(),
            &release,
            &directory,
            &cancelled,
            |_| panic!("cancelled download reached extraction")
        )
        .is_err()
    );
    assert!(
        install(bytes.as_slice(), &release, &directory, &task(), |_| Err(
            "invalid manifest".into()
        ))
        .is_err()
    );
    assert_eq!(
        super::super::artwork::installed_at(&directory),
        Some(old.clone())
    );
    assert_eq!(fs::read(&old).unwrap(), b"old pack");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);

    let destination = install(bytes.as_slice(), &release, &directory, &task(), |path| {
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_eq!(
            super::super::artwork::installed_at(&directory),
            Some(old.clone())
        );
        Ok(path.to_owned())
    })
    .unwrap();
    assert_eq!(
        super::super::artwork::installed_at(&directory),
        Some(destination.clone())
    );
    assert_eq!(fs::read(&destination).unwrap(), bytes);
    assert_eq!(fs::read(&old).unwrap(), b"old pack");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 3);
    // A damaged selection file must never escape the managed artwork directory.
    fs::write(directory.join("current.json"), b"\"../old.tar.zst\"").unwrap();
    assert!(super::super::artwork::installed_at(&directory).is_none());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancellation_during_validation_does_not_activate_a_pack() {
    let directory =
        std::env::temp_dir().join(format!("tenchu-download-cancel-{}", std::process::id()));
    let task = task();
    let bytes = b"complete";
    assert!(
        install(
            bytes.as_slice(),
            &release(bytes),
            &directory,
            &task,
            |path| {
                task.cancel.store(true, Ordering::Relaxed);
                Ok(path.to_owned())
            }
        )
        .is_err()
    );
    assert!(super::super::artwork::installed_at(&directory).is_none());
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "downloads both published artwork packs; use isolated TENCHU_ARTWORK_DOWNLOADS and TENCHU_ASSET_CACHE"]
fn live_artwork_downloads() {
    assert!(std::env::var_os("TENCHU_ARTWORK_DOWNLOADS").is_some());
    assert!(std::env::var_os("TENCHU_ASSET_CACHE").is_some());
    for artwork in Artwork::ALL {
        DOWNLOADS.start(artwork);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3600);
        loop {
            match DOWNLOADS.snapshot().unwrap().1 {
                Status::Installed(path) => {
                    assert_eq!(artwork.installed(), Some(path.clone()));
                    assert!(
                        crate::visuals::pack::unpack(&path)
                            .unwrap()
                            .join("pack.json")
                            .is_file()
                    );
                    println!(
                        "Verified {}: {} bytes",
                        artwork.release().tag,
                        fs::metadata(path).unwrap().len()
                    );
                    break;
                }
                Status::Failed(error) => panic!("{error}"),
                Status::Cancelled => panic!("unexpected cancellation"),
                _ => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
    assert_eq!(DOWNLOADS.generation(), 2);
}
