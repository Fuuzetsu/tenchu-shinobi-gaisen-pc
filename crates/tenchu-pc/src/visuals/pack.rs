use super::{catalog, pixels::Image};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::Path,
    sync::Arc,
};
use tenchu_assets::{Result, stage::MissionAssets};

const MAX_PIXELS: usize = 16 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[derive(Default)]
pub struct Material {
    pub texture: Option<String>,
    pub blend: u8,
    pub double_sided: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub textures: BTreeMap<String, String>,
    #[serde(default)]
    pub surfaces: BTreeMap<String, super::relief::Rule>,
    #[serde(default)]
    pub water: BTreeMap<String, super::water::Kind>,
    #[serde(default)]
    pub models: BTreeMap<String, String>,
    #[serde(default)]
    pub materials: BTreeMap<String, Material>,
    #[serde(default)]
    pub lights: Option<Vec<super::lights::Rule>>,
    #[serde(default)]
    pub player_models: BTreeMap<String, super::appearance::Body>,
}

#[derive(Default)]
pub struct Pack {
    pub textures: BTreeMap<String, Arc<Image>>,
    pub surfaces: BTreeMap<String, super::relief::Surface>,
    pub water: BTreeMap<String, super::water::Kind>,
    pub images: BTreeMap<String, Arc<Image>>,
    pub models: BTreeMap<String, Vec<super::obj::Triangle>>,
    pub materials: BTreeMap<String, Material>,
    pub lights: Option<Vec<super::lights::Rule>>,
    pub player_models: BTreeMap<String, super::appearance::Body>,
}

struct Selection {
    textures: BTreeSet<String>,
    models: BTreeSet<String>,
    player_models: bool,
}

impl Selection {
    fn mission(mission: &MissionAssets) -> Self {
        let mut textures: BTreeSet<_> = catalog::regions(mission).into_values().collect();
        for &image in &mission.images {
            let region = catalog::Region::sprite(&mission.vram, image);
            textures.insert(catalog::image_id(
                [region.rect[2], region.rect[3]],
                &region.original_pixels(&mission.vram),
            ));
        }
        Self {
            textures,
            player_models: true,
            models: catalog::models(mission)
                .into_iter()
                .map(|model| model.id)
                .collect(),
        }
    }
}

impl Pack {
    pub fn interface_from_env(textures: BTreeSet<String>) -> Result<Self> {
        Self::configured(Selection {
            textures,
            models: BTreeSet::new(),
            player_models: false,
        })
    }
    pub fn from_env(mission: &MissionAssets) -> Result<Self> {
        Self::configured(Selection::mission(mission))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn configured(selection: Selection) -> Result<Self> {
        match std::env::var_os("TENCHU_ASSET_PACK") {
            Some(path) => Self::load_selected(Path::new(&path), Some(selection)),
            None => Ok(Self::default()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn configured(selection: Selection) -> Result<Self> {
        let Some(bytes) = crate::web::pack_manifest(selection.player_models)? else {
            return Ok(Self::default());
        };
        let manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Self::decode(manifest, Some(selection), crate::web::pack_file)
    }

    pub fn load(root: &Path) -> Result<Self> {
        Self::load_selected(root, None)
    }

    fn load_selected(root: &Path, selection: Option<Selection>) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let bytes = read(&root, "pack.json", 4 * 1024 * 1024)?;
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Self::decode(manifest, selection, |name, limit| read(&root, name, limit))
    }

    fn decode(
        mut manifest: Manifest,
        mut selection: Option<Selection>,
        mut read: impl FnMut(&str, u64) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        if manifest.version != 1 {
            return Err(format!(
                "Unsupported asset pack version {}",
                manifest.version
            ));
        }
        let mut pack = Self::default();
        if let Some(rules) = &manifest.lights {
            for rule in rules {
                rule.validate()?;
            }
        }
        pack.lights = manifest.lights;
        pack.water = manifest.water;
        if let Some(selection) = &selection {
            if !selection.player_models {
                manifest.player_models.clear();
            }
            manifest
                .models
                .retain(|id, _| selection.models.contains(id));
        }
        let mut models = manifest.models;
        for (id, body) in &manifest.player_models {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            {
                return Err(format!("Invalid player model identity {id:?}"));
            }
            body.validate()?;
            for (index, part) in body.parts.iter().enumerate() {
                let key = format!("player/{id}/{index}");
                if let Some(file) = &part.file {
                    models.insert(key, file.clone());
                } else {
                    pack.models.insert(key, Vec::new());
                }
            }
        }
        pack.player_models = manifest.player_models;
        let mut materials = BTreeSet::new();
        let mut triangles = 0;
        for (id, path) in models {
            let data = read(&path, 64 * 1024 * 1024)?;
            let text = std::str::from_utf8(&data).map_err(|e| format!("{path}: {e}"))?;
            let model = super::obj::decode(text).map_err(|e| format!("{path}: {e}"))?;
            for triangle in &model {
                let material = manifest.materials.get(&triangle.material).ok_or_else(|| {
                    format!(
                        "{path}: unknown material {:?}; declare it in pack.json",
                        triangle.material
                    )
                })?;
                materials.insert(triangle.material.clone());
                if let (Some(selection), Some(texture)) = (&mut selection, &material.texture) {
                    selection.textures.insert(texture.clone());
                }
            }
            triangles += model.len();
            if triangles > 1_000_000 {
                return Err("Asset pack exceeds one million model triangles".into());
            }
            pack.models.insert(id, model);
        }
        if let Some(selection) = selection {
            pack.water.retain(|id, _| selection.textures.contains(id));
            manifest
                .textures
                .retain(|id, _| selection.textures.contains(id));
            manifest
                .surfaces
                .retain(|id, _| selection.textures.contains(id));
            manifest.materials.retain(|id, _| materials.contains(id));
        }
        let mut pixels = 0;
        for (id, path) in manifest.textures {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            {
                return Err(format!("Invalid texture identity {id:?}"));
            }
            let image =
                decode_png(&read(&path, 32 * 1024 * 1024)?).map_err(|e| format!("{path}: {e}"))?;
            pixels += image.rgba.len() / 4;
            if pixels > MAX_PIXELS {
                return Err("Texture pack exceeds 16 million decoded pixels".into());
            }
            let changed = catalog::image_id(image.size.map(|n| n as u16), &image.rgba) != id;
            let image = Arc::new(image);
            if changed {
                pack.textures.insert(id.clone(), image.clone());
            }
            pack.images.insert(id, image);
        }
        pack.materials = manifest.materials;
        for (id, rule) in manifest.surfaces {
            if !rule.strength.is_finite() || !(0.0..=2.0).contains(&rule.strength) {
                return Err(format!("Surface {id}: strength must be 0–2"));
            }
            if rule.world_tile.is_some_and(|tile| {
                tile.into_iter()
                    .any(|n| !n.is_finite() || !(1.0..=1_000_000.0).contains(&n))
            }) {
                return Err(format!(
                    "Surface {id}: world tile periods must be 1–1,000,000 units"
                ));
            }
            let height = rule
                .height
                .map(|path| -> Result<_> {
                    let image = decode_png(&read(&path, 32 * 1024 * 1024)?)?;
                    pixels += image.rgba.len() / 4;
                    if pixels > MAX_PIXELS {
                        return Err("Texture pack exceeds 16 million decoded pixels".into());
                    }
                    Ok(Arc::new(image))
                })
                .transpose()?;
            pack.surfaces.insert(
                id,
                super::relief::Surface {
                    height,
                    strength: rule.strength,
                    world_tile: rule.world_tile,
                    uv_tile: rule.uv_tile,
                },
            );
        }
        for (name, material) in &pack.materials {
            if material.blend > 4
                || material
                    .texture
                    .as_ref()
                    .is_some_and(|id| !pack.images.contains_key(id))
            {
                return Err(format!(
                    "Invalid blend mode or missing texture in material {name}"
                ));
            }
        }
        Ok(pack)
    }
}

pub(crate) fn validate_path(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('\\')
        || name
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!("Asset path must stay inside the pack: {name}"));
    }
    Ok(())
}

pub(crate) fn read(root: &Path, name: &str, limit: u64) -> Result<Vec<u8>> {
    validate_path(name)?;
    let path = root
        .join(name)
        .canonicalize()
        .map_err(|e| format!("{name}: {e}"))?;
    if !path.starts_with(root) {
        return Err(format!("Asset path leaves the pack: {name}"));
    }
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > limit {
        return Err(format!("{name} exceeds its file size limit"));
    }
    fs::read(path).map_err(|e| format!("{name}: {e}"))
}

pub fn decode_png(bytes: &[u8]) -> Result<Image> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    decoder.set_limits(png::Limits {
        bytes: 64 * 1024 * 1024,
    });
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = [reader.info().width, reader.info().height];
    if size.into_iter().any(|n| n == 0 || n > 1024) {
        return Err("Textures must be 1–1024 pixels on each side".into());
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let bytes = &buffer[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|&p| [p, p, p, 255]).collect(),
        png::ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return Err("PNG palette was not expanded".into()),
    };
    Ok(Image { size, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; checks campaign selection against exported model and sprite artwork"]
    fn mission_selection_matches_exported_artwork() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let game = crate::session::Options {
            mission: 5,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        let directory =
            std::env::temp_dir().join(format!("tenchu-pack-selection-{}", std::process::id()));
        super::super::export(&game.mission, &directory).unwrap();
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(directory.join("pack.json")).unwrap()).unwrap();
        fs::remove_dir_all(directory).unwrap();
        assert_eq!(
            Selection::mission(&game.mission).textures,
            manifest.textures.into_keys().collect()
        );
    }

    #[test]
    fn mission_selection_reads_only_used_art_and_model_dependencies() {
        let manifest = serde_json::json!({
            "version": 1,
            "textures": {"roof": "roof.png", "blade": "blade.png", "snow": "missing.png"},
            "surfaces": {"snow": {"height": "missing-height.png", "strength": 1.0}},
            "water": {"roof": "off", "blade": "surface", "snow": "waterfall"},
            "models": {"weapons/1": "blade.obj", "stages/snow/map/0": "missing.obj"},
            "materials": {
                "steel": {"texture": "blade"},
                "unused": {"texture": "unavailable"}
            }
        });
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[255; 4])
                .unwrap();
        }
        let mut reads = Vec::new();
        let selection = Selection {
            textures: BTreeSet::from(["roof".into()]),
            models: BTreeSet::from(["weapons/1".into()]),
            player_models: true,
        };
        let pack = Pack::decode(
            serde_json::from_value(manifest.clone()).unwrap(),
            Some(selection),
            |path, _| {
                reads.push(path.to_owned());
                match path {
                    "roof.png" | "blade.png" => Ok(png.clone()),
                    "blade.obj" => {
                        Ok(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl steel\nf 1 2 3\n".to_vec())
                    }
                    _ => Err(format!("Unexpected read: {path}")),
                }
            },
        )
        .unwrap();
        assert_eq!(reads, ["blade.obj", "blade.png", "roof.png"]);
        assert_eq!(pack.images.len(), 2);
        assert_eq!(pack.materials.len(), 1);
        assert_eq!(pack.models.len(), 1);
        assert!(pack.surfaces.is_empty());
        assert_eq!(
            pack.water,
            BTreeMap::from([
                ("roof".into(), super::super::water::Kind::Off),
                ("blade".into(), super::super::water::Kind::Surface),
            ])
        );

        let failed = Pack::decode(
            serde_json::from_value(manifest).unwrap(),
            Some(Selection {
                textures: BTreeSet::from(["snow".into()]),
                models: BTreeSet::new(),
                player_models: false,
            }),
            |path, _| Err(format!("Missing {path}")),
        );
        assert_eq!(failed.err().unwrap(), "Missing missing.png");
    }

    #[test]
    fn pack_skips_unchanged_exports_and_rejects_broken_or_escaping_edits() {
        let root = std::env::temp_dir().join(format!("tenchu-pack-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let rgba = [31, 127, 255, 128, 0, 0, 0, 0];
        let id = catalog::image_id([2, 1], &rgba);
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
        }
        fs::write(root.join("test.png"), &png).unwrap();
        let mut manifest = Manifest {
            version: 1,
            textures: BTreeMap::from([(id.clone(), "test.png".into())]),
            surfaces: BTreeMap::new(),
            water: BTreeMap::new(),
            models: BTreeMap::new(),
            materials: BTreeMap::new(),
            lights: None,
            player_models: BTreeMap::new(),
        };
        let save = |manifest: &Manifest| {
            fs::write(
                root.join("pack.json"),
                serde_json::to_vec(manifest).unwrap(),
            )
            .unwrap()
        };
        save(&manifest);
        assert!(Pack::load(&root).unwrap().textures.is_empty());
        manifest.textures = BTreeMap::from([("new_art".into(), "test.png".into())]);
        save(&manifest);
        assert_eq!(Pack::load(&root).unwrap().textures["new_art"].rgba, rgba);
        manifest.surfaces.insert(
            "new_art".into(),
            super::super::relief::Rule {
                height: Some("test.png".into()),
                strength: 0.5,
                world_tile: Some([4000.0, 2000.0]),
                uv_tile: true,
            },
        );
        save(&manifest);
        assert_eq!(
            Pack::load(&root).unwrap().surfaces["new_art"]
                .height
                .as_ref()
                .unwrap()
                .rgba,
            rgba
        );
        assert_eq!(
            Pack::load(&root).unwrap().surfaces["new_art"].world_tile,
            Some([4000.0, 2000.0])
        );
        assert!(Pack::load(&root).unwrap().surfaces["new_art"].uv_tile);
        manifest.surfaces.get_mut("new_art").unwrap().world_tile = Some([0.0, 2000.0]);
        save(&manifest);
        assert!(Pack::load(&root).is_err());
        manifest.surfaces.get_mut("new_art").unwrap().world_tile = None;
        manifest.surfaces.get_mut("new_art").unwrap().height = Some("../test.png".into());
        save(&manifest);
        assert!(Pack::load(&root).is_err());
        manifest.surfaces.clear();
        manifest.textures.insert(id, "../test.png".into());
        save(&manifest);
        assert!(Pack::load(&root).is_err());
        fs::write(root.join("pack.json"), b"{}").unwrap();
        assert!(Pack::load(&root).is_err());
        assert!(decode_png(&png[..20]).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
