use super::*;
use crate::campaign_saves::{Catalog, Game as SavedGame};

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU2_DISC and empty isolated TENCHU_SAVE/TENCHU_T2_SAVE; switching campaigns restores records/supplies and preserves both originals"]
fn campaign_selection_restores_independent_progress_and_titles() {
    let path = std::path::PathBuf::from(std::env::var_os("TENCHU_SAVE").expect("Isolate T1 saves"));
    let t2_path =
        std::path::PathBuf::from(std::env::var_os("TENCHU_T2_SAVE").expect("Isolate T2 saves"));
    assert!(!path.exists() && !t2_path.exists());
    let original = tenchu_core::progress::save::Save::default();
    Storage::at(path.clone()).write(&original).unwrap();
    let mut catalog = Catalog::load(path.clone()).unwrap();
    let id = catalog.create("Ayame's campaign", Some(0)).unwrap();
    let mut saved = original.clone();
    saved.selection.character = 1;
    saved.progress.highest = [2, 7];
    saved.progress.stock[1][1] = 27;
    Storage::at(catalog.path(id).unwrap())
        .write(&saved)
        .unwrap();
    let mut app = application(Default::default());
    app.menu.campaigns.open(SavedGame::Tenchu1).unwrap();
    app.controls.key(KeyCode::Enter, true);
    app.select_campaign(id).unwrap();
    assert_eq!(
        app.frontend.as_ref().unwrap().page,
        crate::frontend::Page::Title
    );
    assert_eq!(app.campaign.progress, saved.progress);
    assert_eq!(app.storage.load().unwrap().unwrap(), saved);
    assert!(!app.controls.held(KeyCode::Enter));
    assert_eq!(Catalog::active(path.clone()).unwrap(), id);
    app.campaign.checkpoint(app.campaign.options);
    app.save();
    assert_eq!(Storage::at(path.clone()).load().unwrap().unwrap(), original);
    let state = app.capture_state();
    assert!(
        state.is_err(),
        "title must not retain the previous playable scene"
    );
    app.menu.campaigns.open(SavedGame::Tenchu1).unwrap();
    app.select_campaign(0).unwrap();
    assert_eq!(app.campaign.progress, original.progress);

    use crate::tenchu2::native::{
        campaign::{Content, Playback},
        save::SaveFile,
    };
    let original_t2 = tenchu_core::tenchu2::progress::Progress::default();
    SaveFile::at(t2_path.clone()).write(&original_t2).unwrap();
    let original_bytes = std::fs::read(&t2_path).unwrap();
    let mut t2_catalog = Catalog::load(t2_path.clone()).unwrap();
    let t2_id = t2_catalog.create("Tatsumaru's campaign", Some(0)).unwrap();
    let mut selected = original_t2;
    selected.tatsumaru_unlocked = true;
    selected.records[2][1].rank = 6;
    selected.records[2][1].time_tenths = 1337;
    selected.stock[2].items[4] = Some(36);
    SaveFile::at(t2_catalog.path(t2_id).unwrap())
        .write(&selected)
        .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut playback = Playback::title(disc, 2, 1, Some(SaveFile::new().unwrap())).unwrap();
    playback.select_campaign(&mut t2_catalog, t2_id).unwrap();
    assert!(matches!(playback.content, Content::Title(_)));
    assert_eq!(playback.run.progress.records[2][1], selected.records[2][1]);
    assert_eq!(playback.run.progress.stock[2].items[4], Some(36));
    assert_eq!(std::fs::read(t2_path).unwrap(), original_bytes);
    assert_eq!(Catalog::active(path.clone()).unwrap(), 0);
    assert_eq!(Storage::at(path).load().unwrap().unwrap(), original);
}
