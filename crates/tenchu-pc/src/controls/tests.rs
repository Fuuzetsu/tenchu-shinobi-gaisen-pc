use super::*;

#[test]
fn analog_input_keeps_precision_and_requires_centering_after_focus_changes() {
    let mut controls = Controls::default();
    controls.pad.axes = [0.37, 0.81, -0.23, 0.62];
    controls.pad.direct = input::SQUARE;
    controls.key(KeyCode::KeyA, true);
    assert_eq!(controls.relative_buttons(), input::SQUARE | input::LEFT);
    assert_eq!(controls.sticks()[0], glam::Vec2::new(0.37, 0.81));
    controls.clear();
    assert_eq!(controls.sticks(), [glam::Vec2::ZERO; 2]);
    assert_eq!(controls.relative_buttons(), 0);
    controls.pad.active = Some(0);
    controls.pad.disconnect(0);
    assert_eq!(controls.sticks(), [glam::Vec2::ZERO; 2]);
}

#[test]
fn taps_between_ticks_are_delivered_once_and_cleared_at_host_transitions() {
    let mut controls = Controls::default();
    controls.key(KeyCode::KeyW, true);
    controls.key(KeyCode::Space, true);
    controls.key(KeyCode::Space, false);
    controls.pad.sample(0, input::SQUARE, true);
    controls.pad.sample(0, 0, true);
    assert_eq!(controls.buttons(), input::UP | input::CROSS | input::SQUARE);
    assert_eq!(controls.buttons(), input::UP);
    controls.key(KeyCode::ArrowUp, true);
    controls.key(KeyCode::KeyW, false);
    assert_eq!(controls.buttons(), input::UP);
    controls.key(KeyCode::ArrowUp, false);
    assert_eq!(controls.buttons(), 0);

    controls.key(KeyCode::KeyE, true);
    controls.key(KeyCode::KeyE, false);
    controls.pad.sample(0, input::CROSS, true);
    controls.clear();
    assert_eq!(controls.buttons(), 0);
    assert_eq!(controls.pad.sample(0, input::CROSS, true), 0);
    assert_eq!(controls.buttons(), 0);
    controls.pad.sample(0, 0, true);
    controls.pad.sample(0, input::CROSS, true);
    controls.pad.disconnect(0);
    assert_eq!(controls.buttons(), 0);
}

#[test]
fn stick_thresholds_preserve_diagonals_and_do_not_chatter() {
    let bindings = Bindings::default();
    let mut stick = [0; 4];
    for (axes, expected) in [
        ([0.54, -0.54], 0),
        ([0.55, -0.55], input::RIGHT | input::DOWN),
        ([0.41, -0.41], input::RIGHT | input::DOWN),
        ([0.4, -0.4], 0),
        ([-1.0, 1.0], input::LEFT | input::UP),
    ] {
        for i in 0..2 {
            stick[i] = direction(axes[i], stick[i]);
        }
        assert_eq!(bindings.controller(|_| false, stick), expected);
    }
}

#[test]
fn menu_and_focus_changes_require_release_before_gameplay() {
    let mut controls = Controls::default();
    assert_eq!(controls.pad.sample(0, input::CROSS, true), input::CROSS);
    controls.key(KeyCode::Enter, true);
    controls.clear();
    assert_eq!(controls.pad.sample(0, input::CROSS, true), 0);
    assert_eq!(controls.buttons(), 0);
    controls.pad.sample(0, 0, true);
    assert_eq!(controls.pad.sample(0, input::CROSS, true), input::CROSS);
    controls.clear();
    assert_eq!(
        controls.pad.sample(0, input::START | input::RIGHT, false),
        0
    );
    assert_eq!(controls.pad.sample(0, input::START | input::RIGHT, true), 0);
    assert_eq!(controls.buttons(), 0);
    controls.pad.sample(0, 0, true);
    assert_eq!(controls.pad.sample(0, input::START, true), input::START);
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
    controls.pad.sample(1, input::CROSS | input::UP, true);
    assert_eq!(controls.pad.sample(2, input::START | input::RIGHT, true), 0);
    assert_eq!(controls.buttons(), input::CROSS | input::UP);
    assert!(!controls.pad.disconnect(2));
    controls.key(KeyCode::Space, true);
    controls.pad.sample(1, 0, true);
    assert_eq!(controls.buttons(), input::CROSS);
    controls.pad.sample(1, input::CROSS | input::UP, true);
    controls.key(KeyCode::Space, false);
    assert_eq!(controls.buttons(), input::CROSS | input::UP);
    controls.key(KeyCode::KeyW, true);
    assert!(controls.pad.disconnect(1));
    assert_eq!(controls.buttons(), input::UP);
    assert_eq!(controls.pad.sample(2, input::CIRCLE, false), 0);
    assert_eq!(controls.pad.active, None);
    assert_eq!(controls.pad.sample(2, input::START, true), input::START);
    assert_eq!(controls.buttons(), input::UP | input::START);
}

#[test]
fn remapping_moves_conflicts_preserves_alternates_and_records_virtual_buttons() {
    use bindings::{StickAxis, Target};
    let mut controls = Controls::default();
    controls
        .bindings
        .bind_key(Target::Square, 0, KeyCode::Space)
        .unwrap();
    controls
        .bindings
        .bind_key(Target::Cross, 0, KeyCode::KeyK)
        .unwrap();
    controls
        .bindings
        .bind_controller(Target::Square, 0, PadInput::Button(Button::South));
    controls.bindings.bind_controller(
        Target::Cross,
        0,
        PadInput::Stick {
            axis: StickAxis::RightY,
            positive: true,
        },
    );
    assert_eq!(controls.bindings.key(KeyCode::KeyJ), 0);
    assert_eq!(controls.bindings.key(KeyCode::ArrowUp), input::UP);
    assert_eq!(
        controls.bindings.controller(|b| b == Button::South, [0; 4]),
        input::SQUARE
    );
    assert_eq!(
        controls.bindings.controller(|_| false, [0, 0, 0, 1]),
        input::CROSS
    );
    assert_eq!(controls.bindings.controller(|_| false, [0, 0, 0, -1]), 0);
    let json = serde_json::to_vec(&controls.bindings).unwrap();
    controls.bindings = Bindings::decode(&json).unwrap();
    controls.key(KeyCode::Space, true);
    controls.key(KeyCode::Space, false);
    controls.key(KeyCode::KeyK, true);
    controls.key(KeyCode::KeyK, false);
    assert_eq!(controls.buttons(), input::SQUARE | input::CROSS);
    assert_eq!(controls.buttons(), 0);
    assert!(
        controls
            .bindings
            .bind_key(Target::Cross, 0, KeyCode::F1)
            .is_err()
    );
    assert_eq!(controls.bindings.key(KeyCode::KeyK), input::CROSS);
    controls.bindings.0.get_mut(&Target::Circle).unwrap().keys[0] = Some(KeyCode::KeyK);
    assert!(Bindings::decode(&serde_json::to_vec(&controls.bindings).unwrap()).is_err());
}

#[test]
fn pause_is_remappable_while_menu_escape_and_enter_remain_available() {
    use bindings::Target;
    let mut controls = Controls::default();
    controls
        .bindings
        .bind_key(Target::Start, 0, KeyCode::KeyO)
        .unwrap();
    assert!(controls.key_command(KeyCode::KeyP, true, false).is_none());
    assert!(matches!(
        controls.key_command(KeyCode::KeyO, true, false),
        Some(Command::Pause)
    ));
    controls
        .bindings
        .bind_key(Target::Square, 0, KeyCode::KeyP)
        .unwrap();
    controls.key(KeyCode::KeyP, true);
    assert_eq!(controls.buttons(), input::SQUARE);
    assert!(controls.key_command(KeyCode::KeyP, true, false).is_none());
    assert!(matches!(
        controls.key_command(KeyCode::Escape, true, false),
        Some(Command::Menu(Action::Back))
    ));
    assert!(matches!(
        controls.key_command(KeyCode::Enter, false, false),
        Some(Command::Menu(Action::Confirm))
    ));
}
