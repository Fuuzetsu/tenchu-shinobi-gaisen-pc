use super::{catalog, pixels::Image};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path, sync::Arc};
use tenchu_assets::Result;

const MAX_PIXELS: usize = 16 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub textures: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct Pack {
    pub textures: BTreeMap<String, Arc<Image>>,
}

impl Pack {
    pub fn from_env() -> Result<Self> {
        match std::env::var_os("TENCHU_ASSET_PACK") {
            Some(path) => Self::load(Path::new(&path)),
            None => Ok(Self::default()),
        }
    }

    pub fn load(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let bytes = read(&root, "pack.json", 1024 * 1024)?;
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if manifest.version != 1 {
            return Err(format!(
                "Unsupported asset pack version {}",
                manifest.version
            ));
        }
        let mut pack = Self::default();
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
            let image = decode_png(&read(&root, &path, 32 * 1024 * 1024)?)
                .map_err(|e| format!("{path}: {e}"))?;
            pixels += image.rgba.len() / 4;
            if pixels > MAX_PIXELS {
                return Err("Texture pack exceeds 16 million decoded pixels".into());
            }
            if catalog::image_id(image.size.map(|n| n as u16), &image.rgba) != id {
                pack.textures.insert(id, Arc::new(image));
            }
        }
        Ok(pack)
    }
}

fn read(root: &Path, name: &str, limit: u64) -> Result<Vec<u8>> {
    let relative = Path::new(name);
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!("Asset path must stay inside the pack: {name}"));
    }
    let path = root
        .join(relative)
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
        manifest.textures.insert(id, "../test.png".into());
        save(&manifest);
        assert!(Pack::load(&root).is_err());
        fs::write(root.join("pack.json"), b"{}").unwrap();
        assert!(Pack::load(&root).is_err());
        assert!(decode_png(&png[..20]).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
