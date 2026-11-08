use super::*;
use serde_json::Value;
use std::io::BufRead;

#[test]
#[ignore = "requires TENCHU2_RIBBON_ORACLE from original JP trail routines"]
fn ribbons_match_original_sampling_decay_geometry_and_colors() {
    let file = std::fs::File::open(std::env::var_os("TENCHU2_RIBBON_ORACLE").unwrap()).unwrap();
    let mut count = 0;
    for (case, line) in std::io::BufReader::new(file).lines().enumerate() {
        let row: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let source: Source = serde_json::from_value(row["source"].clone()).unwrap();
        let mut ribbon = Ribbon::new(source);
        let mut drawings = Vec::new();
        let mut alive = true;
        for command in row["commands"].as_array().unwrap() {
            match command["op"].as_str().unwrap() {
                "sample" => ribbon.push(Some(
                    serde_json::from_value(command["sample"].clone()).unwrap(),
                )),
                "stop" => ribbon.push(None),
                "update" => {
                    alive = ribbon.update(&mut drawings);
                    assert_eq!(
                        alive,
                        command["alive"].as_bool().unwrap(),
                        "lifetime {case}"
                    );
                }
                _ => panic!("Unknown ribbon command"),
            }
        }
        assert_eq!(
            serde_json::to_value(&ribbon).unwrap(),
            row["after"],
            "state {case}"
        );
        assert_eq!(
            serde_json::to_value(drawings).unwrap(),
            row["drawings"],
            "drawing {case}"
        );
        assert_eq!(
            alive,
            row["alive"].as_bool().unwrap(),
            "final lifetime {case}"
        );
        count += 1;
    }
    assert!(count >= 1000);
    eprintln!("{count} original ribbon sequences");
}

#[test]
fn reused_projectile_slots_retire_old_trails_without_overwriting_them() {
    let mut effects = Effects::default();
    let source = Source::Projectile(7);
    effects.start_ribbon(source).unwrap();
    effects.sample_ribbon(
        source,
        Some(Sample {
            edges: [[10, 20], [30, 40]],
            depth: 100,
        }),
    );
    effects.start_ribbon(source).unwrap();
    effects.sample_ribbon(
        source,
        Some(Sample {
            edges: [[50, 60], [70, 80]],
            depth: 200,
        }),
    );
    let Effect::Ribbon(old) = &effects.entries[0] else {
        unreachable!()
    };
    assert!(!old.active());
    assert_eq!(old.samples[0].depth, 100);
    let Effect::Ribbon(new) = &effects.entries[1] else {
        unreachable!()
    };
    assert!(new.active());
    assert_eq!(new.samples[0].depth, 200);
    for i in 0..60 {
        effects.start_ribbon(Source::Projectile(i + 100)).unwrap();
    }
    effects.flash(Position::ZERO, false).unwrap_err();
    for _ in 0..3 {
        effects.entries.retain_mut(|e| {
            let Effect::Ribbon(ribbon) = e else {
                unreachable!()
            };
            ribbon.update(&mut Vec::new())
        });
    }
    effects.flash(Position::ZERO, false).unwrap();
}
