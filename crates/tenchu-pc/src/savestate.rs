use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use tenchu_assets::Result;

const MAGIC: &[u8; 8] = b"TENCHUST";
const FORMAT: u32 = 12;
const LIMIT: usize = 512 * 1024 * 1024;
const FILE_LIMIT: usize = 128 * 1024 * 1024;

// Preferences have legacy JSON migrations whose input fields differ from their output.
pub mod json {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

    pub struct Borrowed<'a, T>(pub &'a T);

    impl<T: Serialize> Serialize for Borrowed<'_, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serialize(self.0, serializer)
        }
    }

    pub fn serialize<T: Serialize, S: Serializer>(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde_json::to_string(value)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, T: DeserializeOwned, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<T, D::Error> {
        let json = String::deserialize(deserializer)?;
        serde_json::from_str(&json).map_err(serde::de::Error::custom)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read(path: &std::path::Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|e| format!("Load {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > FILE_LIMIT {
        return Err("Savestate exceeds the 128 MiB file limit".into());
    }
    Ok(bytes)
}

pub fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    let payload = postcard::to_allocvec(value).map_err(|e| e.to_string())?;
    if payload.len() > LIMIT {
        return Err("Savestate exceeds the 512 MiB limit".into());
    }
    let version = env!("CARGO_PKG_VERSION").as_bytes();
    let mut bytes = MAGIC.to_vec();
    bytes.extend(FORMAT.to_le_bytes());
    bytes.push(version.len() as u8);
    bytes.extend(version);
    bytes.extend((payload.len() as u64).to_le_bytes());
    bytes.extend(Sha256::digest(&payload));
    let mut encoder = GzEncoder::new(bytes, Compression::fast());
    encoder.write_all(&payload).map_err(|e| e.to_string())?;
    let bytes = encoder.finish().map_err(|e| e.to_string())?;
    if bytes.len() > FILE_LIMIT {
        return Err("Compressed savestate exceeds the 128 MiB limit".into());
    }
    Ok(bytes)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > FILE_LIMIT || bytes.get(..8) != Some(MAGIC) {
        return Err("Not a Tenchu savestate, or the file is too large".into());
    }
    let mut input = &bytes[8..];
    let read_error = |e: std::io::Error| format!("Incomplete savestate: {e}");
    let mut format = [0; 4];
    input.read_exact(&mut format).map_err(read_error)?;
    if u32::from_le_bytes(format) != FORMAT {
        return Err("This savestate format is not supported".into());
    }
    let mut length = [0; 1];
    input.read_exact(&mut length).map_err(read_error)?;
    let mut version = vec![0; usize::from(length[0])];
    input.read_exact(&mut version).map_err(read_error)?;
    if version != env!("CARGO_PKG_VERSION").as_bytes() {
        return Err(format!(
            "Savestate is from v{}; use that game version to load it",
            String::from_utf8_lossy(&version)
        ));
    }
    let mut length = [0; 8];
    input.read_exact(&mut length).map_err(read_error)?;
    let length = u64::from_le_bytes(length);
    if length > LIMIT as u64 {
        return Err("Savestate exceeds the 512 MiB limit".into());
    }
    let mut checksum = [0; 32];
    input.read_exact(&mut checksum).map_err(read_error)?;
    let mut payload = Vec::new();
    GzDecoder::new(input)
        .take(length + 1)
        .read_to_end(&mut payload)
        .map_err(|e| format!("Damaged savestate: {e}"))?;
    if payload.len() as u64 != length || Sha256::digest(&payload).as_slice() != checksum {
        return Err("Savestate checksum failed; the file is incomplete or damaged".into());
    }
    let (state, remaining) = postcard::take_from_bytes(&payload)
        .map_err(|e| format!("Cannot decode this savestate: {e}"))?;
    if !remaining.is_empty() {
        return Err("Savestate contains unexpected trailing data".into());
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_damaged_truncated_and_incompatible_states() {
        let state = vec![1u32, 2, 300, u32::MAX];
        let bytes = encode(&state).unwrap();
        assert_eq!(decode::<Vec<u32>>(&bytes).unwrap(), state);
        for length in [0, 7, 10, 15, bytes.len() - 1] {
            assert!(decode::<Vec<u32>>(&bytes[..length]).is_err());
        }
        for index in [0, 8, 13, 13 + env!("CARGO_PKG_VERSION").len() + 8] {
            let mut damaged = bytes.clone();
            damaged[index] ^= 1;
            assert!(decode::<Vec<u32>>(&damaged).is_err());
        }
    }
}
