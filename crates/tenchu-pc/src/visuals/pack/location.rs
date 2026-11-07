use std::path::{Path, PathBuf};

pub fn configured_root() -> Option<PathBuf> {
    configured(false)
}

pub fn configured_tenchu2_root() -> Option<PathBuf> {
    configured(true)
}

fn configured(tenchu2: bool) -> Option<PathBuf> {
    let variable = if tenchu2 {
        "TENCHU2_ASSET_PACK"
    } else {
        "TENCHU_ASSET_PACK"
    };
    if let Some(path) = std::env::var_os(variable) {
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    let executable = std::env::current_exe().ok();
    #[cfg(not(target_os = "android"))]
    let directory = std::env::current_dir().ok();
    #[cfg(target_os = "android")]
    let directory = crate::android::data_dir().ok().map(Path::to_owned);
    discover(executable.as_deref(), directory.as_deref(), tenchu2)
}

fn discover(executable: Option<&Path>, directory: Option<&Path>, tenchu2: bool) -> Option<PathBuf> {
    let (folder, archive) = if tenchu2 {
        ("hd-tenchu2", "tenchu2-hd.tar.zst")
    } else {
        ("hd", "tenchu-hd.tar.zst")
    };
    executable
        .and_then(Path::parent)
        .into_iter()
        .flat_map(|path| path.ancestors().take(4))
        .chain(directory)
        .flat_map(|root| {
            [
                root.join("assets").join(folder),
                root.join(archive),
                root.join("share/tenchu-shinobi-gaisen-pc").join(folder),
            ]
        })
        .find(|path| path.join("pack.json").is_file() || path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn relocated_bundles_and_source_builds_find_their_own_pack_first() {
        let root =
            std::env::temp_dir().join(format!("tenchu-pack-location-{}", std::process::id()));
        let cwd = root.join("working");
        fs::create_dir_all(cwd.join("assets/hd")).unwrap();
        fs::write(cwd.join("assets/hd/pack.json"), "{}").unwrap();
        for (executable, pack) in [
            ("windows/tenchu-pc.exe", "windows/assets/hd"),
            (
                "linux/libexec/tenchu-pc",
                "linux/share/tenchu-shinobi-gaisen-pc/hd",
            ),
            (
                "nix/bin/.tenchu-pc-wrapped",
                "nix/share/tenchu-shinobi-gaisen-pc/hd",
            ),
            ("source/target/release/tenchu-pc", "source/assets/hd"),
            ("source/target/release/deps/test-binary", "source/assets/hd"),
        ] {
            let path = root.join(pack);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("pack.json"), "{}").unwrap();
            assert_eq!(
                discover(Some(&root.join(executable)), Some(&cwd), false),
                Some(path)
            );
        }
        assert_eq!(
            discover(None, Some(&cwd), false),
            Some(cwd.join("assets/hd"))
        );
        assert_eq!(discover(None, Some(&cwd), true), None);
        assert_eq!(discover(None, Some(&root), false), None);
        let archive = root.join("download/tenchu-hd.tar.zst");
        fs::create_dir(archive.parent().unwrap()).unwrap();
        fs::write(&archive, []).unwrap();
        assert_eq!(
            discover(
                Some(&root.join("download/libexec/tenchu-pc")),
                Some(&cwd),
                false
            ),
            Some(archive.clone())
        );
        assert_eq!(
            discover(None, archive.parent(), false),
            Some(archive.clone())
        );
        let t2 = root.join("download/tenchu2-hd.tar.zst");
        fs::write(&t2, []).unwrap();
        assert_eq!(discover(None, archive.parent(), true), Some(t2));
        fs::remove_dir_all(root).unwrap();
    }
}
