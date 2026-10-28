use super::*;
use crate::visuals::pack;

pub(super) fn configured(
    destination: &Path,
    interface: &BTreeSet<String>,
) -> Result<Option<crate::web_assets::Artwork>> {
    std::env::var_os("TENCHU_ASSET_PACK")
        .map(|root| export(Path::new(&root), destination, interface))
        .transpose()
}

fn export(
    root: &Path,
    destination: &Path,
    interface: &BTreeSet<String>,
) -> Result<crate::web_assets::Artwork> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let bytes = pack::read(&root, "pack.json", 4 * 1024 * 1024)?;
    let manifest: pack::Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if manifest.version != 1 {
        return Err("Unsupported asset pack version".into());
    }
    let mut files = BTreeMap::from([("pack.json", (4 * 1024 * 1024, true))]);
    let mut add = |path, limit, boot| {
        let entry = files.entry(path).or_insert((limit, boot));
        entry.0 = entry.0.min(limit);
        entry.1 |= boot;
    };
    for (id, path) in &manifest.textures {
        add(path.as_str(), 32 * 1024 * 1024, interface.contains(id));
    }
    for (id, recipe) in &manifest.recipes {
        recipe.validate()?;
        for path in recipe.files() {
            add(path, 32 * 1024 * 1024, interface.contains(id));
        }
    }
    for (id, surface) in &manifest.surfaces {
        if let Some(path) = &surface.height {
            add(path.as_str(), 32 * 1024 * 1024, interface.contains(id));
        }
    }
    for path in manifest.models.values().chain(
        manifest
            .player_models
            .values()
            .flat_map(|body| &body.parts)
            .filter_map(|part| part.file.as_ref()),
    ) {
        add(path.as_str(), 64 * 1024 * 1024, false);
    }
    let mut result = crate::web_assets::Artwork {
        boot: vec![],
        game: vec![],
    };
    let mut total = 0;
    for (name, (limit, boot)) in files {
        let bytes = pack::read(&root, name, limit)?;
        total += bytes.len();
        let path = destination.join("pack").join(name);
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(path, bytes).map_err(|e| e.to_string())?;
        if boot {
            &mut result.boot
        } else {
            &mut result.game
        }
        .push(name.to_owned());
    }
    println!(
        "Artwork: {} menu files, {} background files, {:.1} MiB compressed",
        result.boot.len(),
        result.game.len(),
        total as f64 / 1048576.0
    );
    Ok(result)
}

pub(super) fn update(disc: &Path, destination: &Path) -> Result<()> {
    let path = destination.join("manifest.json");
    let mut manifest: Manifest =
        serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if manifest.version != 1 {
        return Err("Unsupported browser asset manifest".into());
    }
    let assets = GameAssets::open(source::open_disc(disc)?)?;
    let campaign = Campaign::new(&assets, Options::default(), None)?;
    let mut frontend = Frontend::new(&assets, disc, &campaign, Preferences::default())?;
    let mut interface = BTreeSet::new();
    for language in LANGUAGES {
        frontend.language(&assets, language)?;
        interface.extend(frontend.artwork_images().into_keys());
    }
    manifest.artwork = Some(
        configured(destination, &interface)?.ok_or("Set TENCHU_ASSET_PACK to export artwork")?,
    );
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_partitions_references_without_copying_authoring_sources() {
        let root = std::env::temp_dir().join(format!("tenchu-web-pack-{}", std::process::id()));
        let source = root.join("source");
        let output = root.join("output");
        fs::create_dir_all(source.join("images")).unwrap();
        let manifest = serde_json::json!({
            "version": 1,
            "textures": {"world": "images/world.png"},
            "recipes": {
                "menu": {"steps": [{"op":"overlay", "image":"images/layer.png"}]},
                "unused": {"steps": [{"op":"overlay", "image":"images/layer.png"}]}
            },
            "surfaces": {"menu": {"height": "images/height.png", "strength": 0.5}},
            "models": {"weapon": "weapon.obj"},
            "player_models": {"custom": {"name": "Custom", "parts": [{"file": "body.obj", "parent": null, "position": [0, 0, 0]}]} }
        });
        fs::write(
            source.join("pack.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        for name in [
            "images/layer.png",
            "images/world.png",
            "images/height.png",
            "weapon.obj",
            "body.obj",
            "reference.png",
        ] {
            fs::write(source.join(name), name.as_bytes()).unwrap();
        }
        let files = export(&source, &output, &["menu".into()].into()).unwrap();
        assert_eq!(
            files.boot,
            ["images/height.png", "images/layer.png", "pack.json"]
        );
        assert_eq!(files.game, ["body.obj", "images/world.png", "weapon.obj"]);
        assert!(!output.join("pack/reference.png").exists());
        assert_eq!(fs::read(output.join("pack/body.obj")).unwrap(), b"body.obj");
        fs::remove_file(source.join("body.obj")).unwrap();
        assert!(
            export(&source, &output, &BTreeSet::new())
                .unwrap_err()
                .contains("body.obj")
        );
        fs::write(
            source.join("pack.json"),
            br#"{"version":1,"textures":{"escape":"../outside.png"}}"#,
        )
        .unwrap();
        assert!(
            export(&source, &output, &BTreeSet::new())
                .unwrap_err()
                .contains("inside the pack")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
