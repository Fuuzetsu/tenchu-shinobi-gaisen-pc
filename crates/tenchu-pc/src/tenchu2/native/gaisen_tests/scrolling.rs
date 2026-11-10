use super::{doors::Sounds, *};

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; all authored scrolls advance on logic ticks, preserve gameplay and resume from snapshots"]
fn gaisen_scrolling_preserves_gameplay_and_snapshot_phase() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut count = 0;
    for stage in 0..11 {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let (mut assets, layout) = Assets::gaisen(&disc, &source, 0, false).unwrap();
        let mut world = World::new(
            &layout,
            Options {
                hero: 0,
                difficulty: 2,
                seed: 1,
                inventory: Default::default(),
            },
            &assets,
        )
        .unwrap();
        assert_eq!(
            world.environment.texture_scrolls.len(),
            source.vram.scrolling.len()
        );
        let mut expected = assets.gaisen_scrolls.clone();
        count += expected.len();
        let mut reference = world.clone();
        reference.environment.texture_scrolls.clear();
        for _ in 0..257 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            reference
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
            for (actual, expected) in world.environment.texture_scrolls.iter().zip(&mut expected) {
                expected.step();
                assert_eq!(
                    postcard::to_allocvec(actual).unwrap(),
                    postcard::to_allocvec(expected).unwrap()
                );
            }
        }
        let mut gameplay = world.clone();
        gameplay.environment.texture_scrolls.clear();
        assert_eq!(
            postcard::to_allocvec(&gameplay).unwrap(),
            postcard::to_allocvec(&reference).unwrap(),
            "stage {stage}: scrolling changed gameplay"
        );
        let snapshot = postcard::to_allocvec(&world).unwrap();
        for _ in 0..40 {
            world
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        let mut restored: World = postcard::from_bytes(&snapshot).unwrap();
        for _ in 0..40 {
            restored
                .step([0; 2], &mut assets, &mut Sounds::default())
                .unwrap();
        }
        assert_eq!(
            postcard::to_allocvec(&world).unwrap(),
            postcard::to_allocvec(&restored).unwrap(),
            "stage {stage}: scrolling snapshot diverged"
        );
        eprintln!(
            "stage {stage}: {} scrolls, exact native gameplay and phase continuation",
            expected.len()
        );
    }
    assert!(count > 0);
    eprintln!("{count} authored texture scrolls");
}
