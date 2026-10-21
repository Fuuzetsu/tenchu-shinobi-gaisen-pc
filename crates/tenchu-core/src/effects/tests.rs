use super::*;
use crate::math::Position;

#[test]
fn full_pool_keeps_rng_cost_but_does_not_tick_dummy_smoke() {
    let mut effects = Effects::default();
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut env = Environment {
        terrain: &map,
        cursor: &mut cursor,
        clock: 0,
        owner_position: Position::ZERO,
        reference: Position::ZERO,
    };
    let mut rng = Random::new(1);
    effects.smoke_burst(Position::ZERO, 0, 12, 201, &mut rng);
    assert_eq!(rng.calls, 402);
    assert!(effects.slots.iter().all(Option::is_some));
    // Avoid event rolls: this test isolates the inclusive zero lifetime and dummy slot.
    for effect in effects.slots.iter_mut().flatten() {
        let Effect::Smoke(smoke) = effect else {
            unreachable!()
        };
        smoke.change_at = 255;
    }
    let view = View {
        position: Position::ZERO,
        matrix: Matrix::default(),
    };
    let mut scene = Scene::default();
    for _ in 0..15 {
        effects.step(&mut rng, view, &mut scene, &mut env).unwrap();
    }
    assert!(
        effects.slots[..200]
            .iter()
            .all(|s| matches!(s, Some(Effect::Smoke(Smoke { time: 0, .. }))))
    );
    effects.step(&mut rng, view, &mut scene, &mut env).unwrap();
    assert!(effects.slots[..200].iter().all(Option::is_none));
    assert!(matches!(
        effects.slots[200],
        Some(Effect::Smoke(Smoke { time: 15, .. }))
    ));
    assert_eq!(rng.calls, 402);
    effects.smoke_burst(Position::ZERO, 150, 12, 1, &mut rng);
    assert_eq!(rng.calls, 406);
}

#[test]
fn splash_waits_for_visibility_and_new_droplets_run_in_slot_order() {
    let mut effects = Effects::default();
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut env = Environment {
        terrain: &map,
        cursor: &mut cursor,
        clock: 0,
        owner_position: Position::ZERO,
        reference: Position::ZERO,
    };
    let mut rng = Random::new(1);
    let mut scene = Scene::default();
    let forward = View {
        position: Position::ZERO,
        matrix: Matrix::default(),
    };
    let behind = View {
        matrix: Matrix {
            rotation: [[-4096, 0, 0], [0, 4096, 0], [0, 0, -4096]],
            ..Matrix::default()
        },
        ..forward
    };
    effects.splash(Position::new(0, 0, 2000), [8192; 2], 4);
    for _ in 0..30 {
        effects.begin_tick();
        effects
            .step(&mut rng, behind, &mut scene, &mut env)
            .unwrap();
    }
    assert_eq!(rng.calls, 0);
    assert!(effects.drawings.is_empty());
    effects
        .step(&mut rng, forward, &mut scene, &mut env)
        .unwrap();
    assert_eq!(rng.calls, 24);
    assert_eq!(effects.drawings.len(), 7);
    assert!(matches!(
        &effects.slots[1],
        Some(Effect::Splash(Splash {
            mode: 1,
            count: 1,
            ..
        }))
    ));
    let mut expected_rng = Random::new(1);
    for slot in [3, 5, 7, 9, 11, 13] {
        let position = [
            expected_rng.next15() % 200 - 100,
            expected_rng.next15() % 200 - 120,
            2000 + expected_rng.next15() % 200 - 100,
        ];
        let time = (expected_rng.next15() % 27 + 2) as u8;
        let Some(Effect::Bleed(bleed)) = &effects.slots[slot] else {
            panic!("droplet slot {slot}");
        };
        assert_eq!(bleed.position, position);
        assert_eq!(bleed.time, time);
        assert_eq!(bleed.velocity, [0, -19, 0]);
    }
    for _ in 0..40 {
        effects.begin_tick();
        effects
            .step(&mut rng, behind, &mut scene, &mut env)
            .unwrap();
    }
    assert_eq!(effects.slots.iter().flatten().count(), 1);
    assert!(matches!(
        &effects.slots[1],
        Some(Effect::Splash(Splash { count: 1, .. }))
    ));
    for _ in 0..7 {
        effects.begin_tick();
        effects
            .step(&mut rng, forward, &mut scene, &mut env)
            .unwrap();
    }
    assert!(effects.slots.iter().all(Option::is_none));
}

#[test]
fn spawning_from_a_full_pool_preserves_the_visiting_slot_and_dummy_lifetime() {
    let mut effects = Effects::default();
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut env = Environment {
        terrain: &map,
        cursor: &mut cursor,
        clock: 0,
        owner_position: Position::ZERO,
        reference: Position::ZERO,
    };
    effects.slots[..200].fill(Some(Effect::Bleed(Bleed {
        position: Position::new(0, 0, 2000),
        velocity: [0; 3],
        color: [0; 3],
        time: 20,
        mode: 1,
    })));
    effects.slots[0] = Some(Effect::Splash(Splash {
        position: Position::new(0, 0, 2000),
        size: [8192; 2],
        speed: 4,
        count: 0,
        mode: 0,
    }));
    let mut rng = Random::new(1);
    let mut scene = Scene::default();
    effects
        .step(
            &mut rng,
            View {
                position: Position::ZERO,
                matrix: Matrix::default(),
            },
            &mut scene,
            &mut env,
        )
        .unwrap();
    assert_eq!(rng.calls, 24);
    assert_eq!(effects.cursor, 0);
    assert!(matches!(
        effects.slots[0],
        Some(Effect::Splash(Splash { count: 1, .. }))
    ));
    assert!(effects.slots[1..200].iter().all(|slot| matches!(
        slot,
        Some(Effect::Bleed(Bleed {
            time: 19,
            mode: 1,
            ..
        }))
    )));
    assert!(matches!(
        effects.slots[200],
        Some(Effect::Bleed(Bleed {
            velocity: [0, -20, 0],
            ..
        }))
    ));
    assert_eq!(effects.drawings.len(), 200);
}

#[test]
fn expiring_attached_flash_still_visits_its_coordinate() {
    let mut scene = Scene::default();
    let parent = scene.add(
        Some(0),
        Matrix {
            translation: Position::new(0, 0, -1000),
            ..Matrix::default()
        },
    );
    let mut effects = Effects::default();
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut env = Environment {
        terrain: &map,
        cursor: &mut cursor,
        clock: 0,
        owner_position: Position::ZERO,
        reference: Position::ZERO,
    };
    effects.flash(Some(parent), Position::ZERO, 12288, 1);
    let Some(Effect::Flash(flash)) = effects.slots[1].as_mut() else {
        unreachable!()
    };
    flash.mode = 1;
    let mut rng = Random::new(1);
    effects
        .step(
            &mut rng,
            View {
                position: Position::ZERO,
                matrix: Matrix::default(),
            },
            &mut scene,
            &mut env,
        )
        .unwrap();
    assert!(effects.slots[1].is_none());
    assert!(effects.drawings.is_empty());
    assert_eq!(scene.models[parent].flags, scene.epoch);
}

fn flat_map() -> tenchu_assets::terrain::AreaMap {
    let mut bytes = vec![0u8; 48];
    bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
    bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
    for base in [8, 36] {
        for (i, v) in [-1000i16, -1000, 1000, 1000].into_iter().enumerate() {
            bytes[base + i * 2..base + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    bytes[44..46].copy_from_slice(&0x2000u16.to_le_bytes());
    bytes[46..48].copy_from_slice(&u16::MAX.to_le_bytes());
    tenchu_assets::terrain::AreaMap::parse(bytes).unwrap()
}

mod explosions;

mod snow;
