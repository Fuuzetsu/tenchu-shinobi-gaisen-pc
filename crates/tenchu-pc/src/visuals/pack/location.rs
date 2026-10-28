use std::path::{Path, PathBuf};

pub fn configured_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TENCHU_ASSET_PACK") {
        return (!path.is_empty()).then(|| PathBuf::from(path));
    }
    let executable = std::env::current_exe().ok();
    let directory = std::env::current_dir().ok();
    discover(executable.as_deref(), directory.as_deref())
}

fn discover(executable: Option<&Path>, directory: Option<&Path>) -> Option<PathBuf> {
    executable
        .and_then(Path::parent)
        .into_iter()
        .flat_map(|path| path.ancestors().take(4))
        .chain(directory)
        .flat_map(|root| {
            [
                root.join("assets/hd"),
                root.join("tenchu-hd.tar.zst"),
                root.join("share/tenchu-shinobi-gaisen-pc/hd"),
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
                discover(Some(&root.join(executable)), Some(&cwd)),
                Some(path)
            );
        }
        assert_eq!(discover(None, Some(&cwd)), Some(cwd.join("assets/hd")));
        assert_eq!(discover(None, Some(&root)), None);
        let archive = root.join("download/tenchu-hd.tar.zst");
        fs::create_dir(archive.parent().unwrap()).unwrap();
        fs::write(&archive, []).unwrap();
        assert_eq!(
            discover(Some(&root.join("download/libexec/tenchu-pc")), Some(&cwd)),
            Some(archive.clone())
        );
        assert_eq!(discover(None, archive.parent()), Some(archive));
        fs::remove_dir_all(root).unwrap();
    }
}
