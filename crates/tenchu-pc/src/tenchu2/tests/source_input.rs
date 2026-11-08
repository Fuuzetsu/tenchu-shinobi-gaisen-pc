use tenchu_assets::tenchu2::commands::Commands;
use tenchu_core::tenchu2::input::Pad;

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_INPUT_ORACLE from the edition's input routines"]
fn input_history_and_ai_commands_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let commands = Commands::load(&disc).unwrap();
    let tables = [&commands.standing, &commands.crouched, &commands.jumping];
    let input = std::fs::read_to_string(std::env::var_os("TENCHU2_INPUT_ORACLE").unwrap()).unwrap();
    let mut pad = Pad::default();
    let mut checked = 0;
    for line in input.lines() {
        let values: Vec<i32> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let [
            op,
            table,
            value,
            expected,
            held,
            previous,
            time,
            ref history @ ..,
        ] = values[..]
        else {
            panic!("Incomplete source input trace");
        };
        assert_eq!(history.len(), 7);
        let result = match op {
            0 => {
                pad.update(value as u16);
                pad.held as i16
            }
            1 => pad.consume(tables[table as usize]).unwrap_or(-1),
            2 => pad.issue(tables[table as usize], value as i16) as i16,
            _ => panic!("Unknown source input command"),
        };
        assert_eq!(i32::from(result), expected, "operation {checked}");
        assert_eq!(
            (
                i32::from(pad.held),
                i32::from(pad.previous),
                i32::from(pad.time)
            ),
            (held, previous, time),
            "operation {checked}"
        );
        assert_eq!(pad.history.map(i32::from), history, "operation {checked}");
        checked += 1;
    }
    assert!(checked > 75000);
}
