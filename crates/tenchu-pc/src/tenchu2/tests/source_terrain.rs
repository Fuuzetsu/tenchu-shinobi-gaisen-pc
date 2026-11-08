use serde::Deserialize;
use tenchu_core::{
    math::Math,
    tenchu2::terrain::{self, Cursor, Options, Probe, Query, Surface},
};

#[derive(Deserialize)]
struct Row {
    map: usize,
    reset: bool,
    position: [i32; 3],
    #[serde(flatten)]
    query: Sample,
    current: Option<[usize; 2]>,
    last: Option<[usize; 2]>,
    material: u16,
    index: usize,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Sample {
    Floor {
        mode: u8,
        result: Option<i32>,
    },
    Ray {
        #[serde(default)]
        traversal: bool,
        step: [i16; 3],
        count: i16,
        hit: Option<[i32; 3]>,
    },
    Probe {
        width: i16,
        avoid: bool,
        ignore: bool,
        previous: [i32; 6],
        probe: [i32; 6],
    },
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TERRAIN_ORACLE from the original MCD queries"]
fn native_terrain_and_sight_rays_match_source_in_every_map() {
    use std::io::BufRead;
    use tenchu_assets::tenchu2::{Edition, collision::Rules, level::Level, math_tables};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let math = Math {
        tables: math_tables(&disc).unwrap(),
    };
    let rules = Rules::load(&disc).unwrap();
    let mut map = None;
    let mut cursor = Cursor::default();
    let mut map_id = usize::MAX;
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = [0; 3];
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_TERRAIN_ORACLE").unwrap()).unwrap(),
    );
    for (index, line) in input.lines().enumerate() {
        let row: Row = serde_json::from_str(&line.unwrap()).unwrap();
        if row.map != map_id {
            map = Some(Level::load(&disc, row.map).unwrap().collision);
            cursor = Cursor::default();
            map_id = row.map;
            seen.insert(map_id);
        }
        if row.reset {
            cursor.current = None;
        }
        match row.query {
            Sample::Floor { mode, result } => {
                let query = match mode {
                    0 => Query::Base,
                    1 => Query::Surface,
                    2 => Query::Uncached,
                    3 => Query::Relative,
                    _ => panic!("Unknown terrain query"),
                };
                assert_eq!(
                    terrain::level(
                        map.as_ref().unwrap(),
                        &mut cursor,
                        row.position.into(),
                        query
                    )
                    .unwrap(),
                    result,
                    "row {index} map {map_id}"
                );
                counts[0] += 1;
            }
            Sample::Ray {
                traversal,
                step,
                count,
                hit,
            } => {
                let ray = if traversal {
                    terrain::traverse
                } else {
                    terrain::passage
                };
                assert_eq!(
                    ray(
                        map.as_ref().unwrap(),
                        &mut cursor,
                        row.position.into(),
                        step.into(),
                        count,
                        edition,
                        &math
                    )
                    .unwrap()
                    .map(|p| p.to_array()),
                    hit,
                    "row {index} map {map_id}"
                );
                counts[1] += 1;
            }
            Sample::Probe {
                width,
                avoid,
                ignore,
                previous,
                probe,
            } => {
                let mut contact = Probe {
                    floor: (previous[0] != i32::MIN).then_some(previous[0]),
                    height: previous[1],
                    material: previous[2] as u16,
                    blocked: previous[3] as u16,
                    lower: previous[4] as u16,
                    higher: previous[5] as u16,
                };
                contact
                    .sample(
                        map.as_ref().unwrap(),
                        &mut cursor,
                        &rules,
                        row.position.into(),
                        width,
                        Options {
                            avoid_water: avoid,
                            ignore_steps: ignore,
                        },
                    )
                    .unwrap();
                assert_eq!(
                    [
                        contact.floor.unwrap_or(i32::MIN),
                        contact.height,
                        i32::from(contact.material),
                        i32::from(contact.blocked),
                        i32::from(contact.lower),
                        i32::from(contact.higher),
                    ],
                    probe,
                    "row {index} map {map_id}"
                );
                counts[2] += 1;
            }
        }
        let surface = |id: Option<Surface>| id.map(|id| [id.cell, id.node]);
        assert_eq!(
            surface(cursor.current),
            row.current,
            "row {index} map {map_id}"
        );
        assert_eq!(surface(cursor.last), row.last, "row {index} map {map_id}");
        assert_eq!(cursor.material, row.material, "row {index} map {map_id}");
        assert_eq!(cursor.index, row.index, "row {index} map {map_id}");
    }
    assert_eq!(seen.len(), Level::available(&disc).unwrap().len());
    assert!(counts[0] > 45000 && counts[1] > 11000 && counts[2] > 17000);
}
