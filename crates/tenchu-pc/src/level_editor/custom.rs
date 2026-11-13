mod audio;
mod models;
#[cfg(test)]
pub(crate) mod tests;
pub(super) mod ui;

use crate::visuals::{obj, pack, pixels::Image};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
use tenchu_assets::{Result, audio::stream::Clip};

pub const FILE_LIMIT: usize = 32 * 1024 * 1024;
const BUNDLE_LIMIT: usize = 64 * 1024 * 1024;
const DECODED_LIMIT: usize = 192 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Texture,
    Model,
    Audio,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Texture => "PNG texture",
            Self::Model => "OBJ model",
            Self::Audio => "Audio",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub kind: Kind,
    data: Payload,
}

#[derive(Clone)]
struct Payload(Arc<Contents>);

struct Contents {
    bytes: Arc<[u8]>,
    hash: String,
    decoded: OnceLock<Result<Decoded>>,
}

enum Decoded {
    Texture(Arc<Image>),
    Model(Vec<obj::Triangle>),
    Audio(Arc<Clip>),
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Payload")
            .field("hash", &self.0.hash)
            .field("bytes", &self.0.bytes.len())
            .finish()
    }
}

impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.hash == other.0.hash
    }
}
impl Eq for Payload {}

impl Serialize for Payload {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(&self.0.bytes))
    }
}

impl<'de> Deserialize<'de> for Payload {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        if encoded.len() > FILE_LIMIT.div_ceil(3) * 4 {
            return Err(serde::de::Error::custom("Custom asset exceeds 32 MiB"));
        }
        let bytes = STANDARD.decode(encoded).map_err(serde::de::Error::custom)?;
        Self::new(bytes).map_err(serde::de::Error::custom)
    }
}

impl Payload {
    fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > FILE_LIMIT {
            return Err("Custom assets must contain 1 byte to 32 MiB".into());
        }
        Ok(Self(Arc::new(Contents {
            hash: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.into(),
            decoded: OnceLock::new(),
        })))
    }
}

impl Asset {
    pub fn import(name: String, kind: Kind, bytes: Vec<u8>) -> Result<Self> {
        let asset = Self {
            name,
            kind,
            data: Payload::new(bytes)?,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn id(&self) -> &str {
        &self.data.0.hash
    }
    pub fn bytes(&self) -> usize {
        self.data.0.bytes.len()
    }

    fn decoded(&self) -> Result<&Decoded> {
        self.data
            .0
            .decoded
            .get_or_init(|| {
                let bytes = &self.data.0.bytes;
                Ok(match self.kind {
                    Kind::Texture => Decoded::Texture(Arc::new(pack::decode_png(bytes)?)),
                    Kind::Model => {
                        let triangles = obj::decode(
                            std::str::from_utf8(bytes).map_err(|_| "OBJ must be UTF-8 text")?,
                        )?;
                        if triangles.is_empty() || triangles.len() > 100_000 {
                            return Err("Models need 1–100,000 triangulated faces".into());
                        }
                        if triangles.iter().any(|t| t.material.len() > 256) {
                            return Err("Model material name is too long".into());
                        }
                        if triangles
                            .iter()
                            .map(|t| t.material.as_str())
                            .collect::<std::collections::BTreeSet<_>>()
                            .len()
                            > 256
                        {
                            return Err("Models support up to 256 materials".into());
                        }
                        Decoded::Model(triangles)
                    }
                    Kind::Audio => Decoded::Audio(Arc::new(audio::decode(bytes.clone())?)),
                })
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn validate(&self) -> Result<usize> {
        if self.name.is_empty() || self.name.len() > 512 {
            return Err("Asset names need 1–512 bytes".into());
        }
        Ok(match (self.kind, self.decoded()?) {
            (Kind::Texture, Decoded::Texture(image)) => image.rgba.len(),
            (Kind::Model, Decoded::Model(model)) => {
                model.len() * std::mem::size_of::<obj::Triangle>()
            }
            (Kind::Audio, Decoded::Audio(clip)) => clip.frames.len() * 4,
            _ => return Err("Asset type does not match its contents".into()),
        })
    }

    pub fn clip(&self) -> Result<Arc<Clip>> {
        match self.decoded()? {
            Decoded::Audio(clip) => Ok(clip.clone()),
            _ => Err("Choose an audio asset".into()),
        }
    }

    pub fn model(&self) -> Result<&[obj::Triangle]> {
        match self.decoded()? {
            Decoded::Model(model) => Ok(model),
            _ => Err("Choose an OBJ model".into()),
        }
    }

    fn image(&self) -> Result<Arc<Image>> {
        match self.decoded()? {
            Decoded::Texture(image) => Ok(image.clone()),
            _ => Err("Choose a PNG texture".into()),
        }
    }

    pub fn description(&self) -> String {
        match self.decoded() {
            Ok(Decoded::Texture(image)) => format!("{} × {} pixels", image.size[0], image.size[1]),
            Ok(Decoded::Model(model)) => format!("{} triangles", model.len()),
            Ok(Decoded::Audio(clip)) => format!(
                "{:.1} seconds · {} Hz",
                clip.frames.len() as f64 / f64::from(clip.rate),
                clip.rate
            ),
            Err(error) => error,
        }
    }
}

pub type Assets = BTreeMap<String, Asset>;

pub(super) fn share_payloads(assets: &mut Assets, shared: &mut Assets) {
    for (id, asset) in assets {
        if let Some(previous) = shared
            .get(id)
            .filter(|a| a.kind == asset.kind && a.data == asset.data)
        {
            asset.data = previous.data.clone();
        } else {
            shared.insert(id.clone(), asset.clone());
        }
    }
}

pub fn get<'a>(assets: &'a Assets, id: &str, kind: Kind) -> Result<&'a Asset> {
    assets
        .get(id)
        .filter(|a| a.kind == kind)
        .ok_or_else(|| format!("Missing {} asset: {id}", kind.label()))
}

pub fn validate(assets: &Assets) -> Result<()> {
    if assets.len() > 128 {
        return Err("A map supports up to 128 custom assets".into());
    }
    let mut encoded = 0;
    let mut decoded = 0;
    for (id, asset) in assets {
        if id != asset.id() {
            return Err("Custom asset content hash does not match its ID".into());
        }
        encoded += asset.bytes();
        decoded += asset.validate()?;
        if encoded > BUNDLE_LIMIT || decoded > DECODED_LIMIT {
            return Err("Map assets exceed 64 MiB of files or 192 MiB decoded; reduce textures, models or audio duration".into());
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Model {
    pub asset: String,
    /// Millimetres per OBJ unit.
    pub scale: u32,
    pub y_up: bool,
    pub materials: BTreeMap<String, Option<String>>,
}

impl Model {
    pub fn validate(&self, assets: &Assets) -> Result<()> {
        let _ = get(assets, &self.asset, Kind::Model)?;
        if !(1..=100_000).contains(&self.scale) || self.materials.len() > 256 {
            return Err("Invalid model scale or material count".into());
        }
        for (name, id) in &self.materials {
            if name.len() > 256 {
                return Err("Material name is too long".into());
            }
            if let Some(id) = id {
                let _ = get(assets, id, Kind::Texture)?;
            }
        }
        Ok(())
    }
}
