use super::*;
use crate::visuals::pack;
use flate2::read::GzDecoder;
use std::io::Read;

pub(super) fn include_briefings(
    assets: &GameAssets<source::DiscSource>,
    language: Language,
    interface: &mut BTreeSet<String>,
) -> Result<()> {
    use crate::frontend::draw;
    use tenchu_assets::frontend::{Image, mission::Briefing};
    for stage in assets.stages()?.into_iter().filter(|stage| stage.order > 0) {
        let briefing = Briefing::load(&assets.executable, language, stage.id, 0)?;
        for path in [briefing.background, briefing.text] {
            let image = Image::parse(&assets.asset(&path)?)?;
            interface.insert(draw::identity(&draw::pixels(&image)));
        }
    }
    Ok(())
}

pub(super) fn configured(
    destination: &Path,
    interface: &BTreeSet<String>,
) -> Result<Option<crate::web_assets::Artwork>> {
    let browser_pack = destination.parent().map(|path| path.join("assets/hd"));
    let root = if std::env::var_os("TENCHU_ASSET_PACK").is_none()
        && let Some(path) = browser_pack.filter(|p| p.join("pack.json").is_file())
    {
        Some(path)
    } else {
        pack::configured_root()
    };
    root.map(|root| export(&root, destination, interface))
        .transpose()
}

fn export(
    root: &Path,
    destination: &Path,
    interface: &BTreeSet<String>,
) -> Result<crate::web_assets::Artwork> {
    let root = pack::unpack(root)?;
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
    let trace = Arc::new(Mutex::new(BTreeSet::new()));
    let assets = GameAssets::open(source::open_disc_recording(disc, Some(trace.clone()))?)?;
    let campaign = Campaign::new(&assets, Options::default(), None)?;
    let mut frontend = Frontend::new(&assets, disc, &campaign, Preferences::default())?;
    let mut interface = BTreeSet::new();
    for language in LANGUAGES {
        frontend.language(&assets, language)?;
        interface.extend(frontend.artwork_images().into_keys());
        include_briefings(&assets, language, &mut interface)?;
    }
    let trial = assets.disc.find("TENCHU/TRIAL.EXE")?;
    assets.disc.file(&trial).read(0, trial.size as usize)?;
    let pages = trace.lock().map_err(|e| e.to_string())?.clone();
    refresh_boot(destination, &mut manifest, assets.disc.source(), pages)?;
    manifest.artwork = configured(destination, &interface)?;
    fs::write(
        path,
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn refresh_boot(
    destination: &Path,
    manifest: &mut Manifest,
    source: &impl ReadAt,
    pages: BTreeSet<u32>,
) -> Result<()> {
    if source.byte_len() != manifest.length {
        return Err("Browser asset export belongs to a different disc".into());
    }
    let mut cache = Cache::new(manifest.length)?;
    for path in &manifest.boot {
        let mut bytes = Vec::new();
        GzDecoder::new(fs::File::open(destination.join(path)).map_err(|e| e.to_string())?)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        cache.insert(&bytes)?;
    }
    let missing: Vec<_> = pages
        .into_iter()
        .filter(|&page| cache.read(u64::from(page) * PAGE_SIZE as u64, 1).is_err())
        .collect();
    for pages in missing.chunks(64) {
        let path = format!("pages/menu-{:03}.gz", manifest.boot.len());
        let bytes = Cache::bundle(source, pages.iter().copied())?;
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        gzip.write_all(&bytes).map_err(|e| e.to_string())?;
        fs::write(
            destination.join(&path),
            gzip.finish().map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        manifest.boot.push(path);
    }
    if !missing.is_empty() {
        println!("Added {} menu pages to the browser boot set", missing.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_boot_adds_missing_menu_pages_without_reexporting_media() {
        let directory =
            std::env::temp_dir().join(format!("tenchu-web-boot-{}", std::process::id()));
        fs::create_dir_all(directory.join("pages")).unwrap();
        let source = vec![123; PAGE_SIZE * 3];
        let mut manifest = Manifest {
            version: 1,
            length: source.len() as u64,
            format: "Iso2048".into(),
            start_sector: 0,
            boot: vec![],
            game: vec!["pages/game-000.gz".into()],
            audio: vec!["audio/track.ogg".into()],
            movies: vec!["movies/opening.webm".into()],
            artwork: None,
        };
        refresh_boot(&directory, &mut manifest, &source, BTreeSet::from([0])).unwrap();
        let original = fs::read(directory.join(&manifest.boot[0])).unwrap();
        refresh_boot(&directory, &mut manifest, &source, BTreeSet::from([0, 2])).unwrap();
        refresh_boot(&directory, &mut manifest, &source, BTreeSet::from([0, 2])).unwrap();
        assert_eq!(manifest.boot.len(), 2);
        assert_eq!(
            fs::read(directory.join(&manifest.boot[0])).unwrap(),
            original
        );
        assert_eq!(manifest.game, ["pages/game-000.gz"]);
        assert_eq!(manifest.audio, ["audio/track.ogg"]);
        assert_eq!(manifest.movies, ["movies/opening.webm"]);
        let mut bytes = Vec::new();
        GzDecoder::new(fs::File::open(directory.join(&manifest.boot[1])).unwrap())
            .read_to_end(&mut bytes)
            .unwrap();
        let mut cache = Cache::new(manifest.length).unwrap();
        cache.insert(&bytes).unwrap();
        assert_eq!(
            cache.read(2 * PAGE_SIZE as u64, PAGE_SIZE).unwrap(),
            source[..PAGE_SIZE]
        );
        assert!(cache.read(0, 1).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

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
