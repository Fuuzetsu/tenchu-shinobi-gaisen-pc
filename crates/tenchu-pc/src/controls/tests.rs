use super::*;

#[test]
fn stick_thresholds_preserve_diagonals_and_do_not_chatter() {
    let mut pad = Pad::default();
    assert_eq!(pad.sample(3, 0, [0.54, -0.54], true), 0);
    assert_eq!(pad.active, None);
    assert_eq!(
        pad.sample(3, input::CROSS, [0.55, -0.55], true),
        input::CROSS | input::RIGHT | input::DOWN
    );
    for axes in [[0.54, -0.54], [0.41, -0.41], [0.5, -0.5]] {
        assert_eq!(pad.sample(3, input::CROSS, axes, true), 0);
        assert_eq!(pad.buttons(), input::CROSS | input::RIGHT | input::DOWN);
    }
    assert_eq!(pad.sample(3, 0, [0.4, -0.4], true), 0);
    assert_eq!(pad.buttons(), 0);
    assert_eq!(pad.sample(3, 0, [-1.0, 1.0], true), input::LEFT | input::UP);
    assert_eq!(pad.sample(3, input::UP, [0.0; 2], true), 0);
    assert_eq!(pad.buttons(), input::UP);
}

#[test]
fn menu_and_focus_changes_require_release_before_gameplay() {
    let mut controls = Controls::default();
    assert_eq!(
        controls.pad.sample(0, input::CROSS, [0.0; 2], true),
        input::CROSS
    );
    controls.key(KeyCode::Enter, true);
    controls.clear();
    assert_eq!(controls.pad.sample(0, input::CROSS, [0.0; 2], true), 0);
    assert_eq!(controls.buttons(), 0);
    controls.pad.sample(0, 0, [0.0; 2], true);
    assert_eq!(
        controls.pad.sample(0, input::CROSS, [0.0; 2], true),
        input::CROSS
    );
    controls.clear();
    assert_eq!(controls.pad.sample(0, input::START, [1.0, 0.0], false), 0);
    assert_eq!(controls.pad.sample(0, input::START, [1.0, 0.0], true), 0);
    assert_eq!(controls.buttons(), 0);
    controls.pad.sample(0, 0, [0.0; 2], true);
    assert_eq!(
        controls.pad.sample(0, input::START, [0.0; 2], true),
        input::START
    );
    assert!(matches!(
        gamepad_command(input::START, true, true),
        Some(Command::Pause)
    ));
    assert!(gamepad_command(input::CIRCLE | input::CROSS | input::SQUARE, true, false).is_none());
    assert!(matches!(
        gamepad_command(input::CIRCLE, true, true),
        Some(Command::Menu(Action::Back))
    ));
    assert!(matches!(
        gamepad_command(input::CROSS, false, false),
        Some(Command::Menu(Action::Confirm))
    ));
}

#[test]
fn disconnect_and_other_devices_do_not_release_keyboard_input() {
    let mut controls = Controls::default();
    controls.pad.sample(1, input::CROSS, [0.0, 1.0], true);
    assert_eq!(controls.pad.sample(2, input::START, [1.0, 0.0], true), 0);
    assert_eq!(controls.buttons(), input::CROSS | input::UP);
    assert!(!controls.pad.disconnect(2));
    controls.key(KeyCode::Space, true);
    controls.pad.sample(1, 0, [0.0; 2], true);
    assert_eq!(controls.buttons(), input::CROSS);
    controls.pad.sample(1, input::CROSS, [0.0, 1.0], true);
    controls.key(KeyCode::Space, false);
    assert_eq!(controls.buttons(), input::CROSS | input::UP);
    controls.key(KeyCode::KeyW, true);
    assert!(controls.pad.disconnect(1));
    assert_eq!(controls.buttons(), input::UP);
    assert_eq!(controls.pad.sample(2, input::CIRCLE, [0.0; 2], false), 0);
    assert_eq!(controls.pad.active, None);
    assert_eq!(
        controls.pad.sample(2, input::START, [0.0; 2], true),
        input::START
    );
    assert_eq!(controls.buttons(), input::UP | input::START);
}
