use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Matrix, Position},
    random::Random,
    tenchu2::{effects::weather::Weather, terrain::Cursor, visibility::View},
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_WEATHER_ORACLE; JP weather instructions, real terrain and RNG"]
fn native_weather_matches_original_state_terrain_and_random_order() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut maps = std::collections::BTreeMap::new();
    let mut initialized = 0;
    let mut updates = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_WEATHER_ORACLE") {
        let map = row["map"].as_u64().unwrap() as usize;
        if row["operation"] == "initialize" {
            let weather = Weather::for_map(
                map,
                row["resource"].as_u64().unwrap() as u16,
                row["event"].as_bool().unwrap(),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(weather).unwrap(),
                row["expected"],
                "initialization {initialized}"
            );
            initialized += 1;
            continue;
        }
        let level = maps
            .entry(map)
            .or_insert_with(|| Level::load(&disc, map).unwrap());
        let before = &row["before"];
        let mut weather: Weather = serde_json::from_value(before["weather"].clone()).unwrap();
        let mut cursor: Cursor = serde_json::from_value(before["cursor"].clone()).unwrap();
        let mut random = Random::new(before["seed"].as_u64().unwrap() as u32);
        let center: [i32; 3] = serde_json::from_value(row["center"].clone()).unwrap();
        let frame = weather
            .update(
                View {
                    eye: Position::ZERO,
                    look: center.into(),
                    focus: Position::ZERO,
                    matrix: Matrix::default(),
                    distance: 16000,
                },
                &level.collision,
                &mut cursor,
                &math,
                &mut random,
            )
            .unwrap();
        assert_eq!(
            json!({"weather":weather,"cursor":cursor,"seed":random.seed}),
            row["expected"],
            "update {updates}, map {map}"
        );
        assert_eq!(
            random.calls,
            row["random_calls"].as_u64().unwrap(),
            "random calls {updates}"
        );
        let flashes: Vec<_> = frame
            .flash
            .into_iter()
            .map(|intensity| json!(["flash", intensity, 0, 1, 4]))
            .collect();
        let sounds: Vec<_> = frame
            .thunder
            .then_some(json!(["sound", 43, 127]))
            .into_iter()
            .collect();
        let expected_flashes: Vec<_> = row["flashes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flash| json!(["flash", flash[1].as_i64().unwrap().clamp(0, 255), 0, 1, 4]))
            .collect();
        assert_eq!(flashes, expected_flashes, "flash {updates}");
        assert_eq!(json!(sounds), row["sounds"], "sound {updates}");
        updates += 1;
    }
    assert_eq!((initialized, updates), (12, 6144));
}
