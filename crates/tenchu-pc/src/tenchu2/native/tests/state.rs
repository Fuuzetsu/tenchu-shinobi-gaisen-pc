use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC; complete native world/resources/display snapshots continue all three heroes with live particles"]
fn native_resource_snapshots_preserve_simulation_and_display_continuation() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [9, 119, 191] {
        let mut original = Session::load(&disc, sequence, 17).unwrap();
        original.world.actors[original.world.player]
            .actor
            .invulnerable = true;
        for _ in 0..60 {
            original.step(0).unwrap();
        }
        let position = original.world.actors[original.world.player].actor.position;
        original
            .world
            .effects
            .explosion(
                position,
                0,
                &original.assets.resources.math,
                &mut original.world.random,
            )
            .unwrap();
        for _ in 0..3 {
            original.step(0).unwrap();
        }
        assert!(!original.frame.effects.sprites.is_empty());
        let before =
            postcard::to_allocvec(&(&original.world, &original.assets, &original.frame)).unwrap();
        let bytes = crate::savestate::encode(&(&original.world, &original.assets, &original.frame))
            .unwrap();
        let (world, assets, frame): (World, Assets, Frame) =
            crate::savestate::decode(&bytes).unwrap();
        assert!(postcard::to_allocvec(&(&world, &assets, &frame)).unwrap() == before);
        let mut restored = Session::from_parts(world, assets, original.bank.clone());
        restored.frame = frame;
        for tick in 0..180 {
            let buttons = match tick % 90 {
                0..30 => RIGHT,
                30..60 => UP,
                60 => ATTACK,
                _ => 0,
            };
            let a = original.step(buttons).unwrap();
            let b = restored.step(buttons).unwrap();
            assert_eq!(
                a.iter().map(|s| s.sound).collect::<Vec<_>>(),
                b.iter().map(|s| s.sound).collect::<Vec<_>>(),
                "audio sequence {sequence}, update {tick}",
            );
            assert!(
                postcard::to_allocvec(&original.world).unwrap()
                    == postcard::to_allocvec(&restored.world).unwrap(),
                "world sequence {sequence}, update {tick}",
            );
            assert!(
                postcard::to_allocvec(&original.frame).unwrap()
                    == postcard::to_allocvec(&restored.frame).unwrap(),
                "display sequence {sequence}, update {tick}",
            );
        }
        assert!(
            postcard::to_allocvec(&original.assets).unwrap()
                == postcard::to_allocvec(&restored.assets).unwrap(),
            "resource continuation sequence {sequence}",
        );
        eprintln!(
            "Native sequence {sequence}: {} bytes raw, {} bytes compressed; 180 continued updates match",
            before.len(),
            bytes.len(),
        );
    }
}
