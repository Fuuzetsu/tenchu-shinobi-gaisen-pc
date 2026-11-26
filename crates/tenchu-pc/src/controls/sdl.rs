use super::{Change, Device, Event, Kind, Reading};
use gilrs::Button;
use sdl3::{
    event::Event as SdlEvent,
    gamepad::{Axis, Button as SdlButton, Gamepad},
    joystick::JoystickId,
};
use serde_json::json;
use std::collections::BTreeMap;

struct Hints(Vec<&'static std::ffi::CStr>);

impl Hints {
    fn set(&mut self, name: &'static std::ffi::CStr, value: &str) {
        sdl3::hint::set_with_priority(name.to_str().unwrap(), value, &sdl3::hint::Hint::Override);
        self.0.push(name);
    }
}

impl Drop for Hints {
    fn drop(&mut self) {
        for name in &self.0 {
            // These process hints belong to this adapter. Reset restores environment defaults.
            unsafe {
                sdl3::sys::hints::SDL_ResetHint(name.as_ptr());
            }
        }
    }
}

pub(in crate::controls) struct Backend {
    pads: BTreeMap<usize, (Gamepad, Reading)>,
    events: sdl3::EventPump,
    gamepads: sdl3::GamepadSubsystem,
    joysticks: sdl3::JoystickSubsystem,
    _context: sdl3::Sdl,
    _hints: Hints,
    pub errors: Vec<String>,
}

impl Backend {
    pub fn new(kind: Kind) -> Result<Self, String> {
        if !Kind::choices().contains(&kind) {
            return Err("Backend unavailable on this platform".into());
        }
        let mut hints = Hints(Vec::new());
        // winit owns the window and focus. SDL has no video window to test.
        hints.set(c"SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
        if matches!(kind, Kind::SdlXInput | Kind::SdlWgi) {
            for name in [
                c"SDL_JOYSTICK_HIDAPI",
                c"SDL_JOYSTICK_RAWINPUT",
                c"SDL_JOYSTICK_DIRECTINPUT",
                c"SDL_JOYSTICK_GAMEINPUT",
                c"SDL_JOYSTICK_GAMEINPUT_RAW",
            ] {
                hints.set(name, "0");
            }
            hints.set(
                c"SDL_XINPUT_ENABLED",
                if kind == Kind::SdlXInput { "1" } else { "0" },
            );
            hints.set(
                c"SDL_JOYSTICK_WGI",
                if kind == Kind::SdlWgi { "1" } else { "0" },
            );
        }
        // Leave Steam device exclusions, mappings and native Steam HID opt-in untouched.
        sdl3::log::set_output_function(|priority, category, text| {
            if matches!(
                priority,
                sdl3::log::Priority::Warn
                    | sdl3::log::Priority::Error
                    | sdl3::log::Priority::Critical
            ) {
                super::super::diagnostic::message(
                    "SDL",
                    format!("{priority:?}/{category:?}: {text}"),
                );
            }
        });
        let context = sdl3::init().map_err(|e| e.to_string())?;
        let gamepads = context.gamepad().map_err(|e| e.to_string())?;
        let joysticks = context.joystick().map_err(|e| e.to_string())?;
        let events = context.event_pump().map_err(|e| e.to_string())?;
        let ids = gamepads.gamepads().map_err(|e| e.to_string())?;
        let mut backend = Self {
            pads: BTreeMap::new(),
            events,
            gamepads,
            joysticks,
            _context: context,
            _hints: hints,
            errors: Vec::new(),
        };
        for id in ids {
            backend.open(id);
        }
        Ok(backend)
    }

    fn open(&mut self, id: JoystickId) {
        if self.pads.contains_key(&(id.raw() as usize)) {
            return;
        }
        match self.gamepads.open(id) {
            Ok(pad) => {
                let mut reading = Reading::default();
                for axis in [
                    Axis::LeftX,
                    Axis::LeftY,
                    Axis::RightX,
                    Axis::RightY,
                    Axis::TriggerLeft,
                    Axis::TriggerRight,
                ] {
                    apply_axis(&mut reading, axis, pad.axis(axis));
                }
                for (sdl, button) in BUTTON_MAP {
                    reading.set_button(button, pad.button(sdl));
                }
                self.pads.insert(id.raw() as usize, (pad, reading));
            }
            Err(error) => {
                if self.errors.len() == 32 {
                    self.errors.remove(0);
                }
                self.errors.push(format!("Open device {id}: {error}"));
            }
        }
    }

    pub fn next_event(&mut self) -> Option<Event> {
        while let Some(event) = self.events.poll_event() {
            let description = format!("{event:?}");
            let (which, change) = match event {
                SdlEvent::GamepadAdded { which, .. } => {
                    self.open(which);
                    (which, Change::Connected)
                }
                SdlEvent::GamepadRemoved { which, .. } => {
                    self.pads.remove(&(which.raw() as usize));
                    return Some(Event {
                        id: which.raw() as usize,
                        change: Change::Disconnected,
                        reading: Reading::default(),
                        description,
                    });
                }
                SdlEvent::GamepadRemapped { which, .. } => {
                    self.pads.remove(&(which.raw() as usize));
                    self.open(which);
                    (which, Change::Connected)
                }
                SdlEvent::GamepadButtonDown { which, button, .. }
                | SdlEvent::GamepadButtonUp { which, button, .. } => {
                    let pressed = matches!(event, SdlEvent::GamepadButtonDown { .. });
                    let Some((_, reading)) = self.pads.get_mut(&(which.raw() as usize)) else {
                        continue;
                    };
                    let button = BUTTON_MAP
                        .iter()
                        .find(|(sdl, _)| *sdl == button)
                        .map(|(_, b)| *b)
                        .unwrap_or(Button::Unknown);
                    reading.set_button(button, pressed);
                    (which, Change::Button(button, pressed))
                }
                SdlEvent::GamepadAxisMotion {
                    which, axis, value, ..
                } => {
                    let Some((_, reading)) = self.pads.get_mut(&(which.raw() as usize)) else {
                        continue;
                    };
                    (which, apply_axis(reading, axis, value))
                }
                _ => continue,
            };
            // Apply each queued transition, not the final SDL state, to preserve short taps.
            let reading = self
                .pads
                .get(&(which.raw() as usize))
                .map(|(_, r)| *r)
                .unwrap_or_default();
            return Some(Event {
                id: which.raw() as usize,
                change,
                reading,
                description,
            });
        }
        None
    }

    pub fn devices(&self) -> Vec<Device> {
        let mut devices: Vec<_> = self.pads.iter().map(|(&id, (pad, reading))| {
            // The open gamepad owns this pointer for the duration of both calls.
            let rumble = unsafe {
                let props = sdl3::sys::gamepad::SDL_GetGamepadProperties(pad.raw());
                sdl3::sys::properties::SDL_GetBooleanProperty(props,
                    sdl3::sys::gamepad::SDL_PROP_GAMEPAD_CAP_RUMBLE_BOOLEAN, false)
            };
            Device { id, name: pad.name().unwrap_or_else(|| "Unnamed controller".into()), reading: *reading,
                details: json!({
                    "recognized_gamepad": true, "vendor_id": pad.vendor_id(), "product_id": pad.product_id(),
                    "mapping_guid": self.gamepads.guid_for_id(JoystickId::new(id as u32)).to_string(),
                    "mapping": pad.mapping(), "type": format!("{:?}", pad.r#type()), "real_type": format!("{:?}", pad.real_type()),
                    "connection": pad.connection_state().map(|state| match state { sdl3::joystick::ConnectionState::Wired => "wired", sdl3::joystick::ConnectionState::Wireless => "wireless", _ => "unknown" }).unwrap_or("unknown"), "power": format!("{:?}", pad.power_info()),
                    "vibration_supported": rumble,
                })
            }
        }).collect();
        // Include unmapped joysticks: an empty gamepad list alone hides a useful failure mode.
        if let Ok(ids) = self.joysticks.joysticks() {
            for id in ids
                .into_iter()
                .filter(|id| !self.pads.contains_key(&(id.raw() as usize)))
            {
                devices.push(Device {
                    id: id.raw() as usize,
                    name: joystick_name(id),
                    reading: Reading::default(),
                    details: unopened_details(id, self.gamepads.is_gamepad(id)),
                });
            }
        }
        devices
    }

    pub fn rumble(
        &mut self,
        id: usize,
        weak: u16,
        strong: u16,
        milliseconds: u32,
    ) -> Result<(), String> {
        self.pads
            .get_mut(&id)
            .ok_or("Controller disconnected")?
            .0
            .set_rumble(strong, weak, milliseconds)
            .map_err(|e| e.to_string())
    }

    pub fn hints(&self) -> serde_json::Value {
        let mut hints = serde_json::Map::new();
        for key in [
            "SDL_XINPUT_ENABLED",
            "SDL_JOYSTICK_WGI",
            "SDL_JOYSTICK_HIDAPI",
            "SDL_JOYSTICK_RAWINPUT",
            "SDL_JOYSTICK_DIRECTINPUT",
            "SDL_JOYSTICK_GAMEINPUT",
            "SDL_JOYSTICK_GAMEINPUT_RAW",
            "SDL_JOYSTICK_HIDAPI_STEAM",
            "SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS",
        ] {
            hints.insert(key.into(), json!(sdl3::hint::get(key)));
        }
        json!({"version": sdl3::version::version().to_string(), "hints": hints, "errors": self.errors})
    }
}

const BUTTON_MAP: [(SdlButton, Button); 15] = [
    (SdlButton::South, Button::South),
    (SdlButton::East, Button::East),
    (SdlButton::North, Button::North),
    (SdlButton::West, Button::West),
    (SdlButton::LeftShoulder, Button::LeftTrigger),
    (SdlButton::RightShoulder, Button::RightTrigger),
    (SdlButton::Back, Button::Select),
    (SdlButton::Start, Button::Start),
    (SdlButton::Guide, Button::Mode),
    (SdlButton::LeftStick, Button::LeftThumb),
    (SdlButton::RightStick, Button::RightThumb),
    (SdlButton::DPadUp, Button::DPadUp),
    (SdlButton::DPadDown, Button::DPadDown),
    (SdlButton::DPadLeft, Button::DPadLeft),
    (SdlButton::DPadRight, Button::DPadRight),
];

fn apply_axis(reading: &mut Reading, axis: Axis, value: i16) -> Change {
    let index = match axis {
        Axis::LeftX => 0,
        Axis::LeftY => 1,
        Axis::RightX => 2,
        Axis::RightY => 3,
        Axis::TriggerLeft | Axis::TriggerRight => {
            let i = usize::from(axis == Axis::TriggerRight);
            let button = [Button::LeftTrigger2, Button::RightTrigger2][i];
            reading.triggers[i] = f32::from(value.max(0)) / 32767.0;
            let was_pressed = reading.pressed(button);
            let pressed = reading.triggers[i] >= if was_pressed { 0.65 } else { 0.75 };
            reading.set_button(button, pressed);
            return if pressed != was_pressed {
                Change::Button(button, pressed)
            } else {
                Change::Other
            };
        }
    };
    let normalized = f32::from(value) / if value < 0 { 32768.0 } else { 32767.0 };
    // The host convention inherited from gilrs is positive Y = up.
    reading.axes[index] = if index % 2 == 1 {
        -normalized
    } else {
        normalized
    };
    Change::Axis(index)
}

fn unopened_details(id: JoystickId, recognized: bool) -> serde_json::Value {
    // These identity queries do not open a device or return serials/device paths.
    unsafe {
        use sdl3::sys::joystick::*;
        let guid = SDL_GetJoystickGUIDForID(id.into());
        json!({
            "recognized_gamepad": recognized, "opened": false,
            "vendor_id": SDL_GetJoystickVendorForID(id.into()),
            "product_id": SDL_GetJoystickProductForID(id.into()),
            "mapping_guid": guid.data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "joystick_type": SDL_GetJoystickTypeForID(id.into()).0,
        })
    }
}

fn joystick_name(id: JoystickId) -> String {
    // SDL owns the name; copy it before the next pump can remove the device.
    unsafe {
        let name = sdl3::sys::joystick::SDL_GetJoystickNameForID(id.into());
        if name.is_null() {
            "Unmapped joystick".into()
        } else {
            std::ffi::CStr::from_ptr(name)
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::Controls;
    use sdl3::joystick::{JoystickType, VirtualJoystickDescription};
    use tenchu_core::input;
    use winit::keyboard::KeyCode;

    #[test]
    fn virtual_devices_preserve_taps_focus_selection_triggers_and_disconnects() {
        let mut controls = Controls::default();
        controls.start_backend(Kind::Sdl);
        assert!(
            controls.backend_error.is_none(),
            "{:?}",
            controls.backend_error
        );
        let sdl = controls.sdl.as_ref().unwrap();
        let joystick_system = sdl.joysticks.clone();
        let unmapped = joystick_system
            .attach_virtual_joystick(
                VirtualJoystickDescription::new()
                    .name("Unmapped test joystick")
                    .num_hats(1),
            )
            .unwrap();
        controls.poll(true);
        let device = controls
            .devices()
            .into_iter()
            .find(|d| d.id == unmapped.id().raw() as usize)
            .unwrap();
        assert_eq!(device.details["recognized_gamepad"], false);
        assert_eq!(device.details["opened"], false);
        assert_eq!(device.details["mapping_guid"].as_str().unwrap().len(), 32);
        assert_ne!(
            device.details["mapping_guid"],
            "00000000000000000000000000000000"
        );
        assert!(
            device.details["vendor_id"].is_number() && device.details["product_id"].is_number()
        );
        drop(unmapped);
        let desc = || {
            VirtualJoystickDescription::new()
                .name("Tenchu adapter test")
                .joystick_type(JoystickType::Gamepad)
                .with_axes([
                    Axis::LeftX,
                    Axis::LeftY,
                    Axis::RightX,
                    Axis::RightY,
                    Axis::TriggerLeft,
                    Axis::TriggerRight,
                ])
                .with_buttons(BUTTON_MAP.map(|(b, _)| b))
        };
        let connection = joystick_system.attach_virtual_joystick(desc()).unwrap();
        let other_connection = joystick_system.attach_virtual_joystick(desc()).unwrap();
        let joystick = joystick_system.open(connection.id()).unwrap();
        let other = joystick_system.open(other_connection.id()).unwrap();
        for j in [&joystick, &other] {
            j.set_virtual_axis(Axis::TriggerLeft as u32, i16::MIN)
                .unwrap();
            j.set_virtual_axis(Axis::TriggerRight as u32, i16::MIN)
                .unwrap();
        }
        controls.poll(true);
        controls.select_device(None);
        joystick.set_virtual_axis(Axis::LeftX as u32, 4000).unwrap();
        controls.poll(true);
        assert_eq!(
            controls.pad.active, None,
            "small drift must not steal automatic selection"
        );
        other
            .set_virtual_button(SdlButton::Start as u32, true)
            .unwrap();
        assert_eq!(controls.poll(true).pressed & input::START, input::START);
        assert_eq!(
            controls.pad.active,
            Some(other_connection.id().raw() as usize)
        );
        other
            .set_virtual_button(SdlButton::Start as u32, false)
            .unwrap();
        controls.poll(true);
        controls.buttons();
        controls.select_device(Some(connection.id().raw() as usize));
        joystick.set_virtual_axis(Axis::LeftX as u32, 0).unwrap();
        controls.poll(true);
        // Both changes reach SDL before the host drains its queue.
        joystick
            .set_virtual_button(SdlButton::South as u32, true)
            .unwrap();
        controls.sdl.as_ref().unwrap().gamepads.update();
        joystick
            .set_virtual_button(SdlButton::South as u32, false)
            .unwrap();
        controls.sdl.as_ref().unwrap().gamepads.update();
        assert_eq!(controls.poll(true).pressed & input::CROSS, input::CROSS);
        assert_eq!(controls.buttons(), input::CROSS);
        assert_eq!(controls.buttons(), 0);
        for (button, expected) in [
            (SdlButton::DPadLeft, input::LEFT),
            (SdlButton::DPadRight, input::RIGHT),
        ] {
            joystick.set_virtual_button(button as u32, true).unwrap();
            controls.poll(true);
            assert_eq!(controls.buttons(), expected);
            joystick.set_virtual_button(button as u32, false).unwrap();
            controls.poll(true);
            assert_eq!(controls.buttons(), 0);
        }
        for (axis, expected) in [
            (Axis::TriggerLeft, input::L2),
            (Axis::TriggerRight, input::R2),
        ] {
            joystick.set_virtual_axis(axis as u32, i16::MAX).unwrap();
            controls.poll(true);
            assert_eq!(controls.buttons(), expected);
            joystick.set_virtual_axis(axis as u32, i16::MIN).unwrap();
            controls.poll(true);
            assert_eq!(controls.buttons(), 0);
        }
        joystick
            .set_virtual_axis(Axis::LeftY as u32, i16::MIN)
            .unwrap();
        controls.poll(true);
        assert_eq!(controls.buttons(), input::UP);
        assert_eq!(controls.sticks()[0].y, 1.0);
        controls.poll(false);
        assert_eq!(controls.buttons(), 0);
        controls.poll(true);
        assert_eq!(controls.buttons(), 0, "refocus must require release");
        joystick.set_virtual_axis(Axis::LeftY as u32, 0).unwrap();
        controls.poll(true);
        joystick
            .set_virtual_axis(Axis::LeftY as u32, i16::MAX)
            .unwrap();
        controls.poll(true);
        assert_eq!(controls.buttons(), input::DOWN);
        controls.editor.open = true;
        joystick
            .set_virtual_button(SdlButton::West as u32, true)
            .unwrap();
        assert_eq!(controls.poll(true).pressed, 0);
        let report = controls.diagnostic_settings();
        let attempt = &report["history"]["attempts"][0];
        assert!(
            attempt["activity"][connection.id().to_string()]["events"]
                .as_u64()
                .unwrap()
                > 8
        );
        assert_eq!(
            controls.buttons(),
            0,
            "input testing must not play the game"
        );
        controls.close_editor();
        controls.key(KeyCode::KeyE, true);
        drop(connection);
        assert!(controls.poll(true).disconnected);
        assert_eq!(
            controls.buttons(),
            input::TRIANGLE,
            "disconnect must preserve keyboard input"
        );
        drop(joystick);
        drop(other);
        drop(other_connection);
        drop(joystick_system);
        controls.start_backend(Kind::Gilrs);
        assert!(controls.sdl.is_none());
        controls.start_backend(Kind::Sdl);
        assert!(controls.gilrs.is_none());
        assert!(controls.backend_error.is_none());
        assert_eq!(
            controls.buttons(),
            input::TRIANGLE,
            "switching must preserve keyboard input"
        );
        assert_eq!(controls.pad.active, None);
        assert_eq!(
            controls.diagnostic_settings()["history"]["attempts"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn trigger_hysteresis_and_axis_extremes_match_existing_binding_conventions() {
        let mut reading = Reading::default();
        apply_axis(&mut reading, Axis::TriggerLeft, 25000);
        assert!(reading.pressed(Button::LeftTrigger2));
        apply_axis(&mut reading, Axis::TriggerLeft, 23000);
        assert!(reading.pressed(Button::LeftTrigger2));
        apply_axis(&mut reading, Axis::TriggerLeft, 20000);
        assert!(!reading.pressed(Button::LeftTrigger2));
        for axis in [Axis::LeftY, Axis::RightY] {
            apply_axis(&mut reading, axis, i16::MIN);
        }
        assert_eq!(reading.axes, [0.0, 1.0, 0.0, 1.0]);
    }
}
