#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_MATH_ORACLE from the edition's PsyQ routines"]
fn native_math_uses_edition_tables() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = tenchu_core::math::Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let data = std::fs::read_to_string(std::env::var_os("TENCHU2_MATH_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    for line in data.lines() {
        let values: Vec<i32> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [operation, a, b, x, y] = values[..] else {
            panic!("Incomplete math trace")
        };
        let actual = match operation {
            0 => math.pair(a as i16),
            1 => [i32::from(math.atan2(a, b).unwrap()), 0],
            2 => [math.square_root(a), 0],
            _ => panic!("Unknown math operation"),
        };
        assert_eq!(actual, [x, y], "operation {operation}: {a}, {b}");
        checked += 1;
    }
    assert!(checked > 75000);
}
