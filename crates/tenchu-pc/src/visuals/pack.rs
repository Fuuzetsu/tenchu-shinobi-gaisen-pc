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

#[cfg(not(target_arch = "wasm32"))]
mod archive;
#[cfg(not(target_arch = "wasm32"))]
mod location;
#[cfg(not(target_arch = "wasm32"))]
pub use location::configured_root;

const MAX_PIXELS: usize = 16 * 1024 * 1024;
const MAX_INTERFACE_PIXELS: usize = 32 * 1024 * 1024;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub recipes: BTreeMap<String, super::texture_recipe::Recipe>,
    #[serde(default)]
    pub surfaces: BTreeMap<String, super::relief::Rule>,
    #[serde(default)]
    pub water: BTreeMap<String, super::water::Kind>,
    #[serde(default)]
    pub vegetation: BTreeMap<String, super::vegetation::Kind>,
    #[serde(default)]
    pub motion: BTreeMap<String, Vec<super::motion::Rule>>,
    #[serde(default)]
    pub models: BTreeMap<String, String>,
    #[serde(default)]
    pub model_patches: BTreeMap<String, Vec<super::model_patch::Patch>>,
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
    pub vegetation: BTreeMap<String, super::vegetation::Kind>,
    pub motion: BTreeMap<String, Vec<super::motion::Rule>>,
    pub images: BTreeMap<String, Arc<Image>>,
    pub models: BTreeMap<String, Vec<super::obj::Triangle>>,
    pub model_patches: BTreeMap<String, Vec<super::model_patch::Patch>>,
    pub materials: BTreeMap<String, Material>,
    pub lights: Option<Vec<super::lights::Rule>>,
    pub player_models: BTreeMap<String, super::appearance::Body>,
}

struct Selection {
    max_pixels: usize,
    originals: BTreeMap<String, Image>,
    textures: BTreeSet<String>,
    models: BTreeSet<String>,
    player_models: bool,
}

impl Selection {
    fn mission(mission: &MissionAssets) -> Self {
        let mut originals: BTreeMap<_, _> = catalog::regions(mission)
            .into_iter()
            .map(|(region, id)| {
                (
                    id,
                    Image {
                        size: [u32::from(region.rect[2]), u32::from(region.rect[3])],
                        rgba: region.original_pixels(&mission.vram),
                    },
                )
            })
            .collect();
        for &sprite in &mission.images {
            let region = catalog::Region::sprite(&mission.vram, sprite);
            let image = Image {
                size: [u32::from(region.rect[2]), u32::from(region.rect[3])],
                rgba: region.original_pixels(&mission.vram),
            };
            originals.insert(
                catalog::image_id(image.size.map(|n| n as u16), &image.rgba),
                image,
            );
        }
        Self {
            max_pixels: MAX_PIXELS,
            textures: originals.keys().cloned().collect(),
            originals,
            player_models: true,
            models: catalog::models(mission)
                .into_iter()
                .map(|model| model.id)
                .collect(),
        }
    }
}

impl Pack {
    pub fn interface_from_env(originals: BTreeMap<String, Image>) -> Result<Self> {
        Self::configured(Selection {
            max_pixels: MAX_INTERFACE_PIXELS,
            textures: originals.keys().cloned().collect(),
            originals,
            models: BTreeSet::new(),
            player_models: false,
        })
    }
    pub fn from_env(mission: &MissionAssets) -> Result<Self> {
        Self::configured(Selection::mission(mission))
    }

    pub fn for_models(
        vram: &tenchu_assets::texture::Vram,
        models: &[catalog::ModelSource<'_>],
    ) -> Result<Self> {
        let originals = catalog::model_regions(vram, models)
            .into_iter()
            .map(|(region, id)| {
                (
                    id,
                    Image {
                        size: [region.rect[2].into(), region.rect[3].into()],
                        rgba: region.original_pixels(vram),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self::configured(Selection {
            max_pixels: MAX_PIXELS,
            textures: originals.keys().cloned().collect(),
            originals,
            models: models.iter().map(|m| m.id.clone()).collect(),
            player_models: false,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn configured(selection: Selection) -> Result<Self> {
        match configured_root() {
            Some(path) => Self::load_selected(&path, Some(selection)),
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
        #[cfg(not(target_arch = "wasm32"))]
        let root = archive::directory(root)?;
        #[cfg(target_arch = "wasm32")]
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
        for (id, recipe) in &manifest.recipes {
            if manifest.textures.contains_key(id) {
                return Err(format!("Texture {id} declares both a PNG and a recipe"));
            }
            recipe
                .validate()
                .map_err(|error| format!("Texture {id}: {error}"))?;
        }
        let mut pack = Self::default();
        if let Some(rules) = &manifest.lights {
            for rule in rules {
                rule.validate()?;
            }
        }
        pack.lights = manifest.lights;
        pack.water = manifest.water;
        pack.vegetation = manifest.vegetation;
        for rules in manifest.motion.values() {
            if rules.len() > 4 {
                return Err("A model part supports at most four secondary-motion rules".into());
            }
            for rule in rules {
                rule.validate()?;
            }
        }
        pack.motion = manifest.motion;
        for patches in manifest.model_patches.values() {
            if patches.len() > 4096 {
                return Err("A model supports at most 4096 triangle patches".into());
            }
            for patch in patches {
                patch.validate()?;
            }
        }
        pack.model_patches = manifest.model_patches;
        if let Some(selection) = &selection {
            pack.model_patches
                .retain(|id, _| selection.models.contains(id));
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
        if let Some(selection) = &selection {
            pack.water.retain(|id, _| selection.textures.contains(id));
            pack.vegetation
                .retain(|id, _| selection.textures.contains(id));
            manifest
                .textures
                .retain(|id, _| selection.textures.contains(id));
            manifest
                .recipes
                .retain(|id, _| selection.textures.contains(id));
            manifest
                .surfaces
                .retain(|id, _| selection.textures.contains(id));
            manifest.materials.retain(|id, _| materials.contains(id));
        }
        let max_pixels = selection.as_ref().map_or(MAX_PIXELS, |s| s.max_pixels);
        let mut pixels = 0;
        let mut decoded: BTreeMap<String, Arc<Image>> = BTreeMap::new();
        let mut share_image = |image: Image, label: &str| -> Result<(String, Arc<Image>)> {
            let id = catalog::image_id(image.size.map(|n| n as u16), &image.rgba);
            let shared = if let Some(shared) = decoded.get(&id) {
                shared.clone()
            } else {
                pixels += image.rgba.len() / 4;
                if pixels > max_pixels {
                    return Err(format!(
                        "Texture pack exceeds {} million unique decoded pixels while loading {label}",
                        max_pixels / (1024 * 1024),
                    ));
                }
                let image = Arc::new(image);
                decoded.insert(id.clone(), image.clone());
                image
            };
            Ok((id, shared))
        };
        let mut load_image = |path: &str| -> Result<Image> {
            decode_png(&read(path, 32 * 1024 * 1024)?).map_err(|e| format!("{path}: {e}"))
        };
        let ids: BTreeSet<_> = manifest
            .textures
            .keys()
            .chain(manifest.recipes.keys())
            .collect();
        for id in ids {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
            {
                return Err(format!("Invalid texture identity {id:?}"));
            }
            let image = if let Some(recipe) = manifest.recipes.get(id) {
                let original = selection
                    .as_ref()
                    .and_then(|s| s.originals.get(id))
                    .ok_or_else(|| {
                        format!("Texture recipe {id} needs its original disc texture")
                    })?;
                recipe
                    .apply(original, &mut load_image)
                    .map_err(|e| format!("Texture {id}: {e}"))?
            } else {
                load_image(&manifest.textures[id])?
            };
            let (content, image) = share_image(image, id)?;
            let changed = content != *id;
            if changed {
                pack.textures.insert(id.clone(), image.clone());
            }
            pack.images.insert(id.clone(), image);
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
                .map(|path| share_image(load_image(&path)?, &path).map(|(_, image)| image))
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
        || name.contains(':')
        || name.chars().any(char::is_control)
        || name.split('/').any(|part| {
            let device = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            part.ends_with([' ', '.'])
                || matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ["COM", "LPT"].iter().any(|prefix| {
                    device.strip_prefix(prefix).is_some_and(|n| {
                        matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                    })
                })
        })
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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn unpack(root: &Path) -> Result<std::path::PathBuf> {
    archive::directory(root)
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
    #[ignore = "requires TENCHU_DISC and TENCHU_EXPECTED_PACK; compares native disc-backed recipes across menus and missions"]
    fn configured_artwork_matches_reference_with_real_disc_sources() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let expected = std::env::var_os("TENCHU_EXPECTED_PACK").expect("TENCHU_EXPECTED_PACK");
        let root = configured_root().expect("Configured HD pack");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let compare = |selection: Selection, label: &str| {
            let started = std::time::Instant::now();
            let actual = Pack::load_selected(
                &root,
                Some(Selection {
                    max_pixels: selection.max_pixels,
                    originals: selection.originals.clone(),
                    textures: selection.textures.clone(),
                    models: selection.models.clone(),
                    player_models: selection.player_models,
                }),
            )
            .unwrap();
            let elapsed = started.elapsed();
            let reference = Pack::load_selected(Path::new(&expected), Some(selection)).unwrap();
            assert_eq!(
                actual.images.keys().collect::<Vec<_>>(),
                reference.images.keys().collect::<Vec<_>>(),
                "{label}"
            );
            for (id, image) in &actual.images {
                assert!(**image == *reference.images[id], "{label}: {id}");
            }
            println!("{label}: {} textures, {:?}", actual.textures.len(), elapsed);
        };
        for character in 0..2 {
            for mission in 0..=10 {
                let game = crate::session::Options {
                    character,
                    mission,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                compare(
                    Selection::mission(&game.mission),
                    &format!("character {character}, mission {mission}"),
                );
            }
        }
        let campaign = crate::campaign::Campaign::new(&assets, Default::default(), None).unwrap();
        let tables = tenchu_assets::trial::Tables::load(&assets).unwrap();
        for (index, mission) in tables.presets(&assets).unwrap().into_iter().enumerate() {
            let theme = tenchu_assets::trial::Theme::load(&assets, mission.theme).unwrap();
            let mut random = tenchu_core::random::Random::new(1);
            let mission = tenchu_assets::stage::MissionAssets::load_trial(
                &assets,
                &tables,
                &theme,
                &mission,
                0,
                || random.next15() as u16,
            )
            .unwrap();
            compare(Selection::mission(&mission), &format!("TRIAL {index}"));
        }
        let mut frontend =
            crate::frontend::Frontend::new(&assets, disc.as_ref(), &campaign, Default::default())
                .unwrap();
        for language in [
            tenchu_assets::script::Language::English,
            tenchu_assets::script::Language::Japanese,
            tenchu_assets::script::Language::French,
            tenchu_assets::script::Language::Italian,
        ] {
            frontend.language(&assets, language).unwrap();
            let originals = frontend.artwork_images();
            compare(
                Selection {
                    max_pixels: MAX_INTERFACE_PIXELS,
                    textures: originals.keys().cloned().collect(),
                    originals,
                    models: BTreeSet::new(),
                    player_models: false,
                },
                &format!("menu {language:?}"),
            );
        }
    }

    #[test]
    fn mission_selection_reads_only_used_art_and_model_dependencies() {
        let manifest = serde_json::json!({
            "version": 1,
            "textures": {"blade": "blade.png", "snow": "missing.png"},
            "recipes": {
                "roof": {"steps": [{"op":"overlay", "image":"roof.png"}]},
                "unused": {"steps": [{"op":"overlay", "image":"unavailable.png"}]}
            },
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
            max_pixels: MAX_PIXELS,
            originals: BTreeMap::from([(
                "roof".into(),
                Image {
                    size: [1, 1],
                    rgba: vec![0; 4],
                },
            )]),
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
        assert!(Arc::ptr_eq(&pack.images["roof"], &pack.images["blade"]));
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
                max_pixels: MAX_PIXELS,
                originals: BTreeMap::new(),
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
            recipes: BTreeMap::new(),
            textures: BTreeMap::from([(id.clone(), "test.png".into())]),
            surfaces: BTreeMap::new(),
            water: BTreeMap::new(),
            vegetation: BTreeMap::new(),
            motion: BTreeMap::new(),
            models: BTreeMap::new(),
            model_patches: BTreeMap::new(),
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
