use tenchu_assets::{Result, tenchu2::Moves};
use tenchu_core::game::sheathing::UnarmedStyle;

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

pub fn file(style: UnarmedStyle) -> crate::storage::UserFile {
    if style == UnarmedStyle::WangXiaohai {
        crate::storage::UserFile::WangXiaohai
    } else {
        crate::storage::UserFile::Tatsumaru
    }
}

pub fn load(style: UnarmedStyle) -> Result<Moves> {
    let path = file(style).path()?;
    let bytes = crate::storage::read(&path)
        .map_err(|e| format!("Import {} moves in F1 > Movement first: {e}", style.name()))?;
    let moves = decode(&bytes)?;
    if moves.is_wang() != (style == UnarmedStyle::WangXiaohai) {
        return Err("Imported moves do not match the selected unarmed style".into());
    }
    Ok(moves)
}

pub fn export(moves: &Moves) -> Result<bool> {
    let bytes = encode(moves)?;
    let name = if moves.is_wang() {
        "wang-xiaohai.t2moves"
    } else {
        "tatsumaru.t2moves"
    };
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export unarmed moves (game data; for personal use)")
            .add_filter("Tenchu 2 unarmed moves", &["t2moves"])
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
pub fn from_path(path: &std::path::Path, style: UnarmedStyle) -> Result<Moves> {
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
        let disc = crate::source::open_disc(path)?;
        if style == UnarmedStyle::WangXiaohai {
            tenchu_assets::tenchu2::unarmed::import_wang(&disc)
        } else {
            tenchu_assets::tenchu2::import(&disc)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU2_DISC; verify Wang Xiaohai's source registration, authored tracks, strike geometry and portable roundtrip"]
    fn wang_xiaohai_disc_import_and_roundtrip() {
        use tenchu_assets::tenchu2::{
            BONES, motions, registry::Registry, unarmed, weapons::Weapons,
        };
        let path = std::env::var_os("TENCHU2_DISC").unwrap();
        let disc = crate::source::open_disc(path.as_ref()).unwrap();
        let moves = from_path(path.as_ref(), UnarmedStyle::WangXiaohai).unwrap();
        let registry = Registry::load(&disc).unwrap();
        assert_eq!(registry.actor(unarmed::WANG_KIND).unwrap().model, "syaohai");
        let clips = motions::collect(&disc, unarmed::WANG_CLIPS.into()).unwrap();
        let weapons = Weapons::load(&disc).unwrap();
        let bytes = encode(&moves).unwrap();
        let copy = decode(&bytes).unwrap();
        assert_eq!(encode(&copy).unwrap(), bytes);
        assert_eq!(copy.attacks.len(), 8);
        for (i, entry) in copy.attacks.iter().enumerate() {
            let source = &clips[&entry.clip.id];
            assert_eq!(entry.clip.root, source.root);
            for (part, &bone) in BONES.iter().enumerate() {
                assert_eq!(entry.clip.bones[part], source.bones[bone]);
            }
            assert_eq!(entry.clip.sweep, -source.blend as i8);
            assert_eq!(entry.clip.forward_speed, source.forward_speed);
            assert_eq!(entry.clip.side_speed, source.side_speed);
            let attack = weapons.attack(entry.clip.id).unwrap();
            assert_eq!(
                [
                    entry.attack.power,
                    entry.attack.start,
                    entry.attack.end,
                    entry.attack.combo,
                    entry.attack.steering
                ],
                [
                    attack.power,
                    attack.start,
                    attack.end,
                    attack.combo,
                    attack.steering
                ]
                .map(i16::from)
            );
            assert_eq!(BONES[entry.part], [11, 5, 8, 5, 8, 8, 11, 8][i]);
            let contact = weapons
                .parts(6)
                .find(|p| p.bone == BONES[entry.part])
                .unwrap();
            assert_eq!(entry.hitbox.as_ref().unwrap().center, contact.center);
            assert_eq!(entry.hitbox.as_ref().unwrap().radius, contact.radius);
        }
        if let Some(path) = std::env::var_os("TENCHU_WANG_EXPORT") {
            std::fs::write(path, &bytes).unwrap();
        }
        let mut corrupt = copy.clone();
        corrupt.attacks[0].hitbox = None;
        assert!(encode(&corrupt).is_err());
        corrupt = copy.clone();
        corrupt.attacks[0].hitbox.as_mut().unwrap().radius = -1;
        assert!(encode(&corrupt).is_err());
        corrupt = copy;
        corrupt.attacks.swap(0, 1);
        assert!(encode(&corrupt).is_err());
    }

    #[test]
    #[ignore = "requires TENCHU_T2_DISC; actual PAL disc animation, hit-table and portable export validation"]
    fn tatsumaru_disc_import_and_roundtrip() {
        let path = std::env::var_os("TENCHU_T2_DISC").unwrap();
        let moves = from_path(path.as_ref(), UnarmedStyle::Tatsumaru).unwrap();
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
