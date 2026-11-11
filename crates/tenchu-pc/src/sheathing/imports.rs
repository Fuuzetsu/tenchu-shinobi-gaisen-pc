use tenchu_assets::{Result, tenchu2::Moves};

pub const LIMIT: usize = 2 * 1024 * 1024;

pub fn decode(bytes: &[u8]) -> Result<Moves> {
    if bytes.len() > LIMIT {
        return Err("Unarmed moveset exceeds 2 MiB".into());
    }
    let moves: Moves =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid .t2moves file: {e}"))?;
    moves.validate()?;
    Ok(moves)
}

pub fn encode(moves: &Moves) -> Result<Vec<u8>> {
    moves.validate()?;
    serde_json::to_vec(moves).map_err(|e| e.to_string())
}

pub fn load() -> Result<Moves> {
    let path = crate::storage::UserFile::Tatsumaru.path()?;
    let bytes = crate::storage::read(&path)
        .map_err(|e| format!("Import Tatsumaru's moves in F1 > Movement first: {e}"))?;
    decode(&bytes)
}

pub fn export(moves: &Moves) -> Result<bool> {
    let bytes = encode(moves)?;
    let name = "tatsumaru.t2moves";
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Tatsumaru moves (game data; for personal use)")
            .add_filter("Tatsumaru moves", &["t2moves"])
            .set_file_name(name)
            .save_file()
        else {
            return Ok(false);
        };
        crate::storage::atomic_write(&path, &bytes)?;
    }
    #[cfg(target_os = "android")]
    crate::android::export(name, &bytes)?;
    #[cfg(target_arch = "wasm32")]
    crate::web::recording::download(name, &bytes)?;
    Ok(true)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn from_path(path: &std::path::Path) -> Result<Moves> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("t2moves"))
    {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        decode(&bytes)
    } else {
        tenchu_assets::tenchu2::import(&crate::source::open_disc(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_T2_DISC; actual PAL disc animation, hit-table and portable export validation"]
    fn tatsumaru_disc_import_and_roundtrip() {
        let path = std::env::var_os("TENCHU_T2_DISC").unwrap();
        let moves = from_path(path.as_ref()).unwrap();
        let bytes = encode(&moves).unwrap();
        let roundtrip = decode(&bytes).unwrap();
        assert_eq!(moves.attacks.len(), 12);
        for (source, copy) in moves.attacks.iter().zip(roundtrip.attacks) {
            assert_eq!(source.clip, copy.clip);
            println!(
                "{:x}: duration {} / hit {}..{} / contact {}",
                source.clip.id,
                source.clip.duration,
                source.attack.start,
                source.attack.end,
                source.part
            );
        }
        if let Some(path) = std::env::var_os("TENCHU_T2_EXPORT") {
            std::fs::write(path, bytes).unwrap();
        }
        let mut corrupt = moves;
        corrupt.attacks[0].part = 0;
        assert!(encode(&corrupt).is_err());
    }
}
