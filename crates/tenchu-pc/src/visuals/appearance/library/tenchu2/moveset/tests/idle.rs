use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC; source-archive idle inventory preserves authored poses and excludes only the shared boss bind clip"]
fn imported_t2_idle_catalog_matches_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let imported = load(&disc).unwrap();
    let clips = motions::collect(
        &disc,
        imported
            .keys()
            .filter_map(|&kind| registry.clip(kind, 0))
            .collect(),
    )
    .unwrap();
    let mut authored = 0;
    let mut placeholders = 0;
    for (&kind, moves) in &imported {
        let Some(id) = registry.clip(kind, 0) else {
            continue;
        };
        let source = &clips[&id];
        let normal = moves
            .character
            .motions
            .iter()
            .find(|m| m.action == MOT_NORMAL);
        if id == 1 {
            assert_eq!(source.duration, 0);
            assert!(
                source.bones[..15]
                    .iter()
                    .all(|bone| bone.iter().all(|key| key.value == [0; 3]))
            );
            assert!(
                normal.is_none(),
                "kind {kind}: bind pose registered as idle"
            );
            placeholders += 1;
        } else if let Some(normal) = normal {
            let actual = moves
                .clips
                .iter()
                .find(|clip| clip.id == normal.clip)
                .unwrap();
            let mut expected = source.gaisen_pose();
            expected.id = actual.id;
            expected.sweep = (-i32::from(source.blend)).try_into().unwrap();
            expected.forward_speed = source.forward_speed;
            expected.side_speed = source.side_speed;
            assert_eq!(actual, &expected, "kind {kind}: changed authored idle");
            authored += 1;
        } else {
            panic!("kind {kind}: missing authored idle {id}");
        }
    }
    assert!(placeholders >= 10);
    assert!(authored >= 10);
    eprintln!(
        "{}: {authored} authored idles preserved, {placeholders} bind poses omitted",
        registry.edition.model_prefix()
    );
}
