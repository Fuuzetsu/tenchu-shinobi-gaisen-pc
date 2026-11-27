use super::{Axis, Button, Change, Reading};

// Match gilrs' fallback deadzone and jitter thresholds. SDL supplies raw axes
// without device deadzones; keep those separate from the values consumers see.
// Reference: vendor/gilrs/src/gamepad.rs and ev/filter.rs (gilrs 0.11.2).
const DEADZONE: f32 = 0.1;
const JITTER: f32 = 0.01;

#[derive(Default)]
pub(super) struct State {
    pub raw: Reading,
    pub reading: Reading,
}

impl State {
    pub fn button(&mut self, button: Button, pressed: bool) {
        self.raw.set_button(button, pressed);
        self.reading.set_button(button, pressed);
    }

    pub fn axis(&mut self, axis: Axis, value: i16) -> Change {
        let index = match axis {
            Axis::LeftX => 0,
            Axis::LeftY => 1,
            Axis::RightX => 2,
            Axis::RightY => 3,
            Axis::TriggerLeft | Axis::TriggerRight => {
                let i = usize::from(axis == Axis::TriggerRight);
                let button = [Button::LeftTrigger2, Button::RightTrigger2][i];
                let value = f32::from(value.max(0)) / 32767.0;
                self.raw.triggers[i] = value;
                self.reading.triggers[i] = ((value - DEADZONE) / (1.0 - DEADZONE)).max(0.0);
                // Like gilrs, digital trigger hysteresis uses the pre-filter value.
                let was_pressed = self.reading.pressed(button);
                let pressed = value >= if was_pressed { 0.65 } else { 0.75 };
                self.button(button, pressed);
                return if pressed != was_pressed {
                    Change::Button(button, pressed)
                } else {
                    Change::Other
                };
            }
        };
        let normalized = f32::from(value) / if value < 0 { 32768.0 } else { 32767.0 };
        // The host convention inherited from gilrs is positive Y = up.
        self.raw.axes[index] = if index % 2 == 1 {
            -normalized
        } else {
            normalized
        };
        let start = index / 2 * 2;
        let [x, y] = [self.raw.axes[start], self.raw.axes[start + 1]];
        let magnitude = (x * x + y * y).sqrt().min(1.0);
        let scale = if magnitude <= DEADZONE {
            0.0
        } else {
            (magnitude - DEADZONE) / (1.0 - DEADZONE) / magnitude
        };
        // Recompute the pair from raw values, never from an already filtered axis.
        for (value, previous) in [x * scale, y * scale]
            .into_iter()
            .zip(&mut self.reading.axes[start..start + 2])
        {
            if value == 0.0 || value.abs() == 1.0 || (value - *previous).abs() >= JITTER {
                *previous = value;
            }
        }
        Change::Axis(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_deadzones_preserve_directions_endpoints_and_release() {
        let mut state = State::default();
        state.axis(Axis::LeftX, 3200);
        state.axis(Axis::LeftY, -3200);
        assert!(state.reading.axes[0] > 0.0 && state.reading.axes[1] > 0.0);
        assert!((state.reading.axes[0] - state.reading.axes[1]).abs() < 0.0001);
        state.axis(Axis::LeftX, 0);
        assert_eq!(state.reading.axes[..2], [0.0; 2]);
        state.axis(Axis::LeftY, 0);
        state.axis(Axis::LeftX, 16384);
        assert!((state.reading.axes[0] - 4.0 / 9.0).abs() < 0.0001);
        let stable = state.reading.axes;
        state.axis(Axis::LeftX, 16400);
        assert_eq!(state.reading.axes, stable);
        for value in [i16::MIN, i16::MAX] {
            for axis in [Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY] {
                state.axis(axis, value);
            }
            let sign = f32::from(value.signum());
            assert_eq!(state.reading.axes, [sign, -sign, sign, -sign]);
        }
        for axis in [Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY] {
            state.axis(axis, 100);
        }
        assert_eq!(state.reading.axes, [0.0; 4]);
        assert_ne!(state.raw.axes, [0.0; 4]);
    }

    #[test]
    fn triggers_filter_analog_noise_without_changing_digital_thresholds() {
        let mut state = State::default();
        for (axis, button, i) in [
            (Axis::TriggerLeft, Button::LeftTrigger2, 0),
            (Axis::TriggerRight, Button::RightTrigger2, 1),
        ] {
            for (raw, pressed) in [(1000, false), (25000, true), (22000, true), (21000, false)] {
                state.axis(axis, raw);
                assert_eq!(state.reading.pressed(button), pressed);
                assert_eq!(state.raw.pressed(button), pressed);
                if raw == 1000 {
                    assert_eq!(state.reading.triggers[i], 0.0);
                    assert!(state.raw.triggers[i] > 0.0);
                }
            }
            state.axis(axis, i16::MAX);
            assert_eq!(state.reading.triggers[i], 1.0);
            state.axis(axis, 0);
            assert_eq!(state.reading.triggers[i], 0.0);
            assert!(!state.reading.pressed(button));
        }
    }
}
