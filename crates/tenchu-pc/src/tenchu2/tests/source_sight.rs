use serde::Deserialize;
use tenchu_core::{
    math::Math,
    tenchu2::{
        actor::State,
        ai::sight::{Body, Context, Observer, Ranges, Sight, Visibility},
        terrain::Cursor,
    },
};

#[derive(Deserialize)]
struct Row {
    map: usize,
    position: [i32; 3],
    yaw: i16,
    height: i16,
    kind: i16,
    state: u8,
    weapon: i16,
    range: i16,
    slot: i16,
    target: [i32; 3],
    target_height: i16,
    target_state: u8,
    target_action: i16,
    water: bool,
    present: bool,
    player: [i32; 3],
    player_yaw: i16,
    tick: u32,
    script: u16,
    near: bool,
    initial_ranges: [i32; 6],
    result: i16,
    distance: i32,
    angle: i16,
    player_angle: i16,
    destination: [i32; 3],
    ranges: [i32; 6],
    cover_calls: usize,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SIGHT_ORACLE from the original perception routine"]
fn native_perception_and_edition_differences_match_source() {
    use std::io::BufRead;
    use tenchu_assets::tenchu2::{Edition, level::Level, math_tables};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let math = Math {
        tables: math_tables(&disc).unwrap(),
    };
    let mut map = None;
    let mut map_id = usize::MAX;
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = [0; 5];
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_SIGHT_ORACLE").unwrap()).unwrap(),
    );
    for (index, line) in input.lines().enumerate() {
        let row: Row = serde_json::from_str(&line.unwrap()).unwrap();
        if row.map != map_id {
            map = Some(Level::load(&disc, row.map).unwrap().collision);
            map_id = row.map;
            seen.insert(map_id);
        }
        let observer = Observer {
            body: Body {
                position: row.position.into(),
                yaw: row.yaw,
                height: row.height,
                state: State::from_action(i16::from(row.state) << 8).unwrap(),
                action: 0,
                in_water: false,
            },
            kind: row.kind,
            slot: row.slot,
            weapon: row.weapon,
            range: row.range,
        };
        let target = Body {
            position: row.target.into(),
            yaw: 0,
            height: row.target_height,
            state: State::from_action(i16::from(row.target_state) << 8).unwrap(),
            action: row.target_action,
            in_water: row.water,
        };
        let mut cover_calls = 0;
        let mut near = || {
            cover_calls += 1;
            row.near
        };
        let mut context = Context {
            edition,
            tick: row.tick,
            player_position: row.player.into(),
            player_yaw: row.player_yaw,
            script_mode: row.script,
            terrain: map.as_ref().unwrap(),
            math: &math,
            near_object: &mut near,
        };
        let mut ranges = Ranges::new(row.initial_ranges[2] - 2000);
        let mut sight = Sight {
            distance: 123,
            angle: 321,
            player_angle: -123,
            destination: [100, 200, 300].into(),
        };
        let visibility = sight
            .check(
                &observer,
                row.present.then_some(&target),
                &mut context,
                &mut ranges,
                &mut Cursor::default(),
            )
            .unwrap();
        let actual = match visibility {
            Visibility::Unchanged => 0,
            Visibility::Hidden => -2,
            Visibility::Unnoticed => -1,
            Visibility::Recognized => 1,
            Visibility::Suspicious => 2,
        };
        assert_eq!(actual, row.result, "row {index} map {map_id}");
        assert_eq!(
            (sight.distance, sight.angle, sight.player_angle),
            (row.distance, row.angle, row.player_angle),
            "row {index} map {map_id}"
        );
        assert_eq!(sight.destination.to_array(), row.destination, "row {index}");
        assert_eq!(cover_calls, row.cover_calls, "row {index}");
        assert_eq!(
            [
                ranges.standing.notice,
                ranges.standing.recognize,
                ranges.standing.forget,
                ranges.crouched.notice,
                ranges.crouched.recognize,
                ranges.crouched.forget,
            ],
            row.ranges,
            "row {index}"
        );
        counts[(actual + 2) as usize] += 1;
    }
    assert_eq!(seen.len(), Level::available(&disc).unwrap().len());
    assert!(counts.iter().all(|&n| n > 100));
}
