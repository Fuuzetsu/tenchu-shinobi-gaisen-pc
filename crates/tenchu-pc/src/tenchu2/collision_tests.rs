use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU_T2_COLLISION_ORACLE; floor queries recorded from the retail MIPS routine"]
fn imported_collision_matches_retail_floor_queries() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let input =
        std::fs::read_to_string(std::env::var_os("TENCHU_T2_COLLISION_ORACLE").unwrap()).unwrap();
    let mut maps = BTreeMap::new();
    let mut checked = 0;
    let mut blocked = 0;
    for line in input.lines() {
        let values: Vec<i32> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [map, x, y, z, expected]: [i32; 5] = values.try_into().unwrap();
        let terrain = maps.entry(map).or_insert_with(|| {
            Level::load(&disc, map as usize)
                .unwrap()
                .collision
                .into_terrain()
                .unwrap()
        });
        let actual =
            terrain::level(terrain, &mut Cursor::new(terrain), [x, y, z].into(), 0).unwrap();
        assert_eq!(actual, expected, "map{map} at {x},{y},{z}");
        checked += 1;
        blocked += usize::from(expected == terrain::NONE);
    }
    assert_eq!(maps.len(), 23);
    assert!(checked > 10_000 && blocked > 1000);
    eprintln!(
        "Matched {checked} retail queries ({blocked} blocked) across {} maps",
        maps.len()
    );
}
