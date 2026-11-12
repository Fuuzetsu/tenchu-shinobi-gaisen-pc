use super::*;

fn temporary() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "tenchu-campaign-slots-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn existing_save_and_bound_autosaves_survive_slot_switches() {
    use tenchu_core::progress::save::Save;
    let root = temporary();
    let path = root.join("progress.sav");
    let mut original = Save::default();
    original.progress.highest = [5, 2];
    storage::atomic_write(&path, &original.encode()).unwrap();
    let original_bytes = storage::read(&path).unwrap();
    let mut catalog = Catalog::load(path.clone()).unwrap();
    assert_eq!(Catalog::active_path(path.clone()).unwrap(), path);
    let bound = storage::Storage::at(catalog.path(0).unwrap());
    catalog.rename(0, "Original campaign").unwrap();
    let copy = catalog.create("Ayame run", Some(0)).unwrap();
    let fresh = catalog.create("Fresh run", None).unwrap();
    assert!(
        storage::Storage::at(catalog.path(fresh).unwrap())
            .load()
            .unwrap()
            .is_none()
    );
    catalog.select(copy).unwrap();
    assert_eq!(storage::read(&path).unwrap(), original_bytes);
    assert_eq!(
        Catalog::active_path(path.clone()).unwrap(),
        catalog.path(copy).unwrap()
    );
    original.progress.highest[0] = 7;
    bound.write(&original).unwrap();
    assert_eq!(
        storage::read(&catalog.path(copy).unwrap()).unwrap(),
        original_bytes
    );
    let mut duplicate = storage::Storage::at(catalog.path(copy).unwrap())
        .load()
        .unwrap()
        .unwrap();
    duplicate.progress.highest[1] = 10;
    storage::Storage::at(catalog.path(copy).unwrap())
        .write(&duplicate)
        .unwrap();
    assert_eq!(bound.load().unwrap().unwrap(), original);
    let reloaded = Catalog::load(path).unwrap();
    assert_eq!(reloaded.index.active, copy);
    assert_eq!(reloaded.index.slots[0].name, "Original campaign");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn broken_catalog_and_failed_metadata_write_preserve_saves() {
    let root = temporary();
    let path = root.join("progress.sav");
    let bytes = tenchu_core::progress::save::Save::default().encode();
    storage::atomic_write(&path, &bytes).unwrap();
    let mut catalog = Catalog::load(path.clone()).unwrap();
    std::fs::write(&catalog.directory, b"not a directory").unwrap();
    assert!(catalog.create("Second", Some(0)).is_err());
    assert_eq!(catalog.index.slots.len(), 1);
    assert_eq!(storage::read(&path).unwrap(), bytes);
    std::fs::remove_file(&catalog.directory).unwrap();
    catalog.create("Second", None).unwrap();
    let index = catalog.directory.join("index.json");
    let original = storage::read(&index).unwrap();
    assert!(catalog.rename(0, "Second").is_err());
    assert_eq!(storage::read(&index).unwrap(), original);
    storage::atomic_write(&index, b"truncated").unwrap();
    assert!(Catalog::active_path(path.clone()).is_err());
    assert_eq!(storage::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_t2_slots_preserve_supplies_and_ignore_unplayed_unlocks_in_summary() {
    use crate::tenchu2::native::save::SaveFile;
    use tenchu_core::tenchu2::progress::Progress;
    let root = temporary();
    let path = root.join("tenchu2-jp-progress.json");
    let mut progress = Progress::default();
    progress.records[1][0].rank = 6;
    progress.records[1][0].time_tenths = 1234;
    progress.stock[1].items[4] = Some(37);
    let file = SaveFile::at(path.clone());
    file.write(&progress).unwrap();
    let original = storage::read(&path).unwrap();
    let mut catalog = Catalog::load(path).unwrap();
    let copy = catalog.create("Ayame", Some(0)).unwrap();
    catalog.select(copy).unwrap();
    let copied = SaveFile::at(catalog.path(copy).unwrap()).load().unwrap();
    assert_eq!(copied.stock[1].items[4], Some(37));
    let summary = Game::Tenchu2.summary(&original).unwrap();
    assert!(summary[0].contains("0 recorded results"));
    assert!(summary[1].contains("1 recorded results · 1 Grand Master"));
    progress.stock[1].items[4] = Some(12);
    file.write(&progress).unwrap();
    assert_eq!(
        storage::read(&catalog.path(copy).unwrap()).unwrap(),
        original
    );
    std::fs::remove_dir_all(root).unwrap();
}
