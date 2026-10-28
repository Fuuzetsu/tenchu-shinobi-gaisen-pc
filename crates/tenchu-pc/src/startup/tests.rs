use super::*;
use std::{cell::RefCell, fs};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tenchu-disc-startup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn remembers_absolute_paths_and_explicit_failure_does_not_fall_back_or_replace_them() {
    let fixture = Fixture::new();
    let saved = fixture.0.join("last-disc.json");
    let relative = Path::new("Cargo.toml");
    let absolute = std::path::absolute(relative).unwrap();
    assert!(relative.is_file());
    select_with(
        Some(relative),
        Some(&saved),
        |_| Ok(()),
        || panic!("Explicit path must skip picker"),
    )
    .unwrap();
    assert_eq!(remembered_path(&saved), Some(absolute.clone()));

    let opened = RefCell::new(Vec::new());
    let result = select_with(
        None,
        Some(&saved),
        |path| {
            opened.borrow_mut().push(path.to_owned());
            Ok(42)
        },
        || panic!("Valid previous disc must skip picker"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, (absolute.clone(), 42));
    assert_eq!(opened.borrow().as_slice(), std::slice::from_ref(&absolute));

    let bad = fixture.0.join("unsupported.cue");
    let result = select_with(
        Some(&bad),
        Some(&saved),
        |path| {
            assert_eq!(path, bad);
            Err::<(), _>("Wrong disc".into())
        },
        || panic!("An explicit failure must not open a different disc"),
    );
    assert_eq!(result.unwrap_err(), "Wrong disc");
    assert_eq!(remembered_path(&saved), Some(absolute));

    let replacement = fixture.0.join("忍者 — another disc.cue");
    fs::write(&replacement, b"fixture").unwrap();
    select_with(
        Some(&replacement),
        Some(&saved),
        |_| Ok(()),
        || panic!("Unexpected picker"),
    )
    .unwrap();
    assert_eq!(remembered_path(&saved), Some(replacement));
}

#[test]
fn missing_damaged_or_invalid_cache_recovers_through_picker_without_blocking_play() {
    let fixture = Fixture::new();
    let saved = fixture.0.join("last-disc.json");
    let disc = fixture.0.join("game.cue");
    fs::write(&disc, b"fixture").unwrap();
    let choose = || Ok(Some((disc.clone(), 7)));

    let selected = select_with(None, Some(&saved), |_| panic!("No saved disc"), choose).unwrap();
    assert_eq!(selected, Some((disc.clone(), 7)));
    assert_eq!(remembered_path(&saved), Some(disc.clone()));

    let selected = select_with(
        None,
        Some(&saved),
        |_| Err("Referenced BIN missing or hashes wrong".into()),
        || Ok(None::<(PathBuf, ())>),
    )
    .unwrap();
    assert!(selected.is_none());
    assert_eq!(
        remembered_path(&saved),
        Some(disc.clone()),
        "Cancelling must not overwrite the last selection"
    );

    fs::remove_file(&disc).unwrap();
    assert!(
        select_with(
            None,
            Some(&saved),
            |_| panic!("Missing file must not be opened"),
            || Ok(None::<(PathBuf, ())>)
        )
        .unwrap()
        .is_none()
    );
    fs::write(&saved, b"{broken").unwrap();
    fs::write(&disc, b"fixture").unwrap();
    assert_eq!(
        select_with(
            None,
            Some(&saved),
            |_| panic!("Corrupt cache must be ignored"),
            choose
        )
        .unwrap(),
        Some((disc.clone(), 7))
    );

    // A read-only/unwritable preference must not prevent a verified disc from opening.
    let selected = select_with(
        Some(&disc),
        Some(&fixture.0),
        |_| Ok(9),
        || panic!("Unexpected picker"),
    )
    .unwrap();
    assert_eq!(selected, Some((disc, 9)));
}
