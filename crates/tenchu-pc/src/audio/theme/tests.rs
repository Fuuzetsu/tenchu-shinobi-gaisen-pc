use super::*;
use crate::level_editor::custom::tests::wav;

fn path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "tenchu-sandbox-theme-{}-{name}.json",
        std::process::id()
    ))
}

#[test]
fn sandbox_theme_preferences_retain_validated_bytes_and_disabled_selection() {
    let path = path("roundtrip");
    let _ = std::fs::remove_file(&path);
    assert_eq!(Theme::load(&path), (Theme::default(), None, None));
    let mut theme = Theme::imported("Owned tone.wav".into(), wav()).unwrap();
    theme.save(&path, Some(4)).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        json["asset"]["data"].is_string(),
        "Retain a bounded base64 payload"
    );
    let (loaded, fallback, notice) = Theme::load(&path);
    assert_eq!(loaded, theme);
    assert_eq!(fallback, Some(4));
    assert!(notice.is_none());
    let clip = loaded.selected().unwrap().clip().unwrap();
    assert_eq!((clip.rate, clip.frames.len()), (8000, 800));
    assert!(clip.frames.iter().any(|f| f[0].abs() > 7000));
    theme.enabled = false;
    theme.save(&path, Some(1)).unwrap();
    let (loaded, fallback, notice) = Theme::load(&path);
    assert_eq!(loaded, theme);
    assert_eq!(fallback, Some(1));
    assert!(notice.is_none());
    assert!(loaded.selected().is_none());
    assert!(
        loaded.asset.is_some(),
        "Disable retains bytes for re-enabling"
    );
    Theme::default().save(&path, None).unwrap();
    assert_eq!(Theme::load(&path), (Theme::default(), None, None));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn sandbox_theme_bad_saved_assets_keep_the_valid_disc_fallback() {
    let path = path("invalid");
    let valid = Theme::imported("Owned tone.wav".into(), wav()).unwrap();
    valid.save(&path, Some(3)).unwrap();
    let base: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (field, bad) in [
        ("version", serde_json::json!(2)),
        ("asset", serde_json::Value::Null),
    ] {
        let mut json = base.clone();
        json[field] = bad;
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let (theme, fallback, notice) = Theme::load(&path);
        assert_eq!(theme, Theme::default());
        assert_eq!(fallback, Some(3));
        assert!(notice.is_some());
    }
    for (field, bad) in [
        ("name", serde_json::json!("")),
        ("name", serde_json::json!("x".repeat(513))),
        ("kind", serde_json::json!("Texture")),
        ("data", serde_json::json!("aW52YWxpZA==")),
    ] {
        let mut json = base.clone();
        json["asset"][field] = bad;
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let (theme, fallback, notice) = Theme::load(&path);
        assert_eq!(theme, Theme::default());
        assert_eq!(fallback, Some(3));
        assert!(notice.is_some(), "{field}");
    }
    let mut json = base;
    json["fallback"] = serde_json::json!(8);
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let (theme, fallback, notice) = Theme::load(&path);
    assert_eq!(theme, Theme::default());
    assert_eq!(fallback, None);
    assert!(notice.is_some());
    std::fs::write(&path, b"broken JSON").unwrap();
    let (theme, fallback, notice) = Theme::load(&path);
    assert_eq!(theme, Theme::default());
    assert_eq!(fallback, None);
    assert!(notice.is_some());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn sandbox_theme_failed_import_or_save_preserves_existing_preferences() {
    let path = path("atomic");
    let valid = Theme::imported("Owned tone.wav".into(), wav()).unwrap();
    valid.save(&path, Some(1)).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(Theme::imported("broken.mp3".into(), b"invalid".to_vec()).is_err());
    assert!(valid.save(&path, Some(8)).is_err());
    let invalid = Theme {
        enabled: true,
        asset: None,
    };
    assert!(invalid.save(&path, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
#[ignore = "requires TENCHU_THEME_FIXTURES containing user-owned tone.wav/ogg/mp3/flac"]
fn sandbox_theme_supported_formats_decode_retained_audio() {
    let root = std::path::PathBuf::from(std::env::var_os("TENCHU_THEME_FIXTURES").unwrap());
    for extension in ["wav", "ogg", "mp3", "flac"] {
        let name = format!("tone.{extension}");
        let theme =
            Theme::imported(name.clone(), std::fs::read(root.join(&name)).unwrap()).unwrap();
        let encoded = postcard::to_allocvec(&theme).unwrap();
        let loaded: Theme = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(loaded, theme);
        let clip = loaded.selected().unwrap().clip().unwrap();
        assert_eq!(clip.rate, 44100);
        assert!((1.9..2.2).contains(&(clip.frames.len() as f64 / f64::from(clip.rate))));
        assert!(clip.frames.iter().any(|f| f[0].abs() > 1000));
        assert!(clip.frames.iter().any(|f| f[0] != f[1]), "stereo {name}");
        eprintln!(
            "{name}: {} retained bytes, {} frames at {} Hz",
            encoded.len(),
            clip.frames.len(),
            clip.rate
        );
    }
}
