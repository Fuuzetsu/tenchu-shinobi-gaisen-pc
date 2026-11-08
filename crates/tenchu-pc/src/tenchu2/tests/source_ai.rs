use tenchu_core::{
    random::Random,
    tenchu2::ai::{Program, Programs},
};

#[test]
#[ignore = "requires TENCHU2_AI_ORACLE from the selected edition's AI assignment routine"]
fn ai_assignments_and_difficulty_randomness_match_source() {
    let data = std::fs::read_to_string(std::env::var_os("TENCHU2_AI_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    let mut random_calls = 0;
    let programs = |values: &[u32]| Programs {
        idle: Program::from_code(values[0] as u16).unwrap(),
        suspicious: Program::from_code(values[1] as u16).unwrap(),
        combat: Program::from_code(values[2] as u16).unwrap(),
        searching: Program::from_code(values[3] as u16).unwrap(),
    };
    for line in data.lines() {
        let values: Vec<u32> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(values.len(), 15);
        let [code, map, difficulty, seed, ..] = values[..] else {
            unreachable!()
        };
        let mut rng = Random::new(seed);
        let mut actual = programs(&values[4..8]);
        let active = actual
            .assign(code as u16, map as usize, difficulty as u8, &mut rng)
            .unwrap();
        assert_eq!(active, values[8] != 0, "row {checked}");
        assert_eq!(actual, programs(&values[11..]), "row {checked}");
        assert_eq!(
            (rng.seed, rng.calls),
            (values[9], u64::from(values[10])),
            "row {checked}"
        );
        random_calls += rng.calls;
        checked += 1;
    }
    assert!(checked > 30000);
    assert!(random_calls > 100);
}

#[test]
#[ignore = "requires TENCHU2_DISC; checks native cast and all placement AI programs"]
fn native_cast_and_ai_cover_the_source_layouts() {
    use tenchu_assets::tenchu2::{Edition, layout::Layout, registry::Registry};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    let total: usize = (0..24)
        .flat_map(|map| (0..3).map(move |hero| (hero, map)))
        .map(|(hero, map)| registry.cast(hero, map).unwrap().len())
        .sum();
    assert_eq!(
        total,
        if registry.edition == Edition::Europe {
            267
        } else {
            252
        }
    );
    let mut selected = 0;
    for layout in layouts {
        assert!(
            layout
                .actors
                .windows(2)
                .all(|actors| actors[0].slot < actors[1].slot)
        );
        for hero in 0..3 {
            let cast = registry.cast(hero, layout.map).unwrap();
            for actor in &layout.actors {
                if cast.contains(&actor.kind) {
                    assert!(registry.actor(actor.kind).is_some());
                    for difficulty in 1..=3 {
                        Programs::default()
                            .assign(actor.behavior, layout.map, difficulty, &mut Random::new(1))
                            .unwrap();
                    }
                    selected += 1;
                }
            }
        }
    }
    assert!(selected > 500);
}
