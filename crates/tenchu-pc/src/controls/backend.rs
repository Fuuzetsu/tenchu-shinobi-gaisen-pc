use super::{Controls, Pad, bindings::StickAxis};
use gilrs::{Button, EventType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
#[path = "sdl.rs"]
pub(super) mod sdl;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(super) enum Kind {
    #[default]
    Gilrs,
    Sdl,
    SdlXInput,
    SdlWgi,
}

impl Kind {
    pub fn parse(name: &str) -> Option<Self> {
        let kind = match name {
            "gilrs" => Self::Gilrs,
            "sdl" => Self::Sdl,
            "sdl-xinput" => Self::SdlXInput,
            "sdl-wgi" => Self::SdlWgi,
            _ => return None,
        };
        Self::choices().contains(&kind).then_some(kind)
    }

    pub fn choices() -> &'static [Self] {
        #[cfg(target_os = "windows")]
        return &[Self::Gilrs, Self::Sdl, Self::SdlXInput, Self::SdlWgi];
        #[cfg(all(
            not(target_os = "windows"),
            not(target_arch = "wasm32"),
            not(target_os = "android")
        ))]
        return &[Self::Gilrs, Self::Sdl];
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        &[Self::Gilrs]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Gilrs => "gilrs (default)",
            Self::Sdl => "SDL automatic",
            Self::SdlXInput => "SDL XInput only",
            Self::SdlWgi => "SDL WGI only",
        }
    }
}

// Keep the existing serialized gilrs button names in controls.json.
pub(super) const BUTTONS: [Button; 19] = [
    Button::South,
    Button::East,
    Button::North,
    Button::West,
    Button::C,
    Button::Z,
    Button::LeftTrigger,
    Button::LeftTrigger2,
    Button::RightTrigger,
    Button::RightTrigger2,
    Button::Select,
    Button::Start,
    Button::Mode,
    Button::LeftThumb,
    Button::RightThumb,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub(super) struct Reading {
    pub buttons: u32,
    pub axes: [f32; 4],
    pub triggers: [f32; 2],
}

impl Reading {
    pub fn pressed(self, button: Button) -> bool {
        BUTTONS
            .iter()
            .position(|&b| b == button)
            .is_some_and(|i| self.buttons & (1 << i) != 0)
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        if let Some(i) = BUTTONS.iter().position(|&b| b == button) {
            self.buttons = (self.buttons & !(1 << i)) | (u32::from(pressed) << i);
        }
    }

    pub fn names(self) -> Vec<String> {
        BUTTONS
            .iter()
            .filter(|&&b| self.pressed(b))
            .map(|b| format!("{b:?}"))
            .collect()
    }

    fn gilrs(pad: &gilrs::Gamepad<'_>) -> Self {
        let mut reading = Self {
            axes: StickAxis::ALL.map(|axis| finite(pad.value(axis.axis()))),
            triggers: [Button::LeftTrigger2, Button::RightTrigger2].map(|button| {
                pad.button_data(button)
                    .map_or(0.0, |b| finite(b.value()).max(0.0))
            }),
            ..Self::default()
        };
        for button in BUTTONS {
            reading.set_button(button, pad.is_pressed(button));
        }
        reading
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Change {
    Connected,
    Disconnected,
    Button(Button, bool),
    Axis(usize),
    Other,
}

pub(super) struct Event {
    pub id: usize,
    pub change: Change,
    pub reading: Reading,
    pub raw_reading: Option<Reading>,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Device {
    pub id: usize,
    pub name: String,
    pub details: Value,
    pub reading: Reading,
}

impl Controls {
    pub(super) fn start_backend(&mut self, kind: Kind) {
        self.controller_history.driver(self.driver_details());
        self.controller_history.devices(self.devices());
        self.stop_rumble();
        self.gilrs = None;
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            self.sdl = None;
        }
        self.pad = Pad::default();
        self.selected_device = None;
        self.cancel_capture();
        self.backend = kind;
        self.vibration_error = None;
        super::diagnostic::install_logger();
        let result = if kind == Kind::Gilrs {
            gilrs::Gilrs::new()
                .map(|g| self.gilrs = Some(g))
                .map_err(|e| e.to_string())
        } else {
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            {
                sdl::Backend::new(kind).map(|s| self.sdl = Some(s))
            }
            #[cfg(any(target_arch = "wasm32", target_os = "android"))]
            {
                Err("SDL controller input is available on desktop builds only".into())
            }
        };
        self.backend_error = result.err();
        if let Some(error) = &self.backend_error {
            eprintln!("{} controller initialization: {error}", kind.label());
        }
        self.controller_history
            .begin(kind, self.backend_error.clone(), self.devices());
        self.controller_history.driver(self.driver_details());
    }

    pub(super) fn change_backend(&mut self, kind: Kind) {
        self.start_backend(kind);
        let save = || {
            let path = crate::storage::UserFile::ControllerBackend.path()?;
            crate::storage::atomic_write(
                &path,
                &serde_json::to_vec(&kind).map_err(|e| e.to_string())?,
            )
        };
        self.editor.notice = match save() {
            Ok(()) => "Controller backend saved. Release held inputs before continuing.".into(),
            Err(error) => format!("Backend applies for this session; could not save: {error}"),
        };
    }

    pub(super) fn select_device(&mut self, id: Option<usize>) {
        self.stop_rumble();
        self.selected_device = id;
        self.pad = Pad {
            active: id,
            ..Pad::default()
        };
        if let Some(reading) = self.current_reading() {
            self.pad.axes = reading.axes;
            self.pad.stick = reading.axes.map(|v| super::direction(v, 0));
            self.pad.raw = self
                .bindings
                .controller(|b| reading.pressed(b), self.pad.stick);
            self.pad.direct = self.bindings.controller(|b| reading.pressed(b), [0; 4]);
            self.pad.menu_raw =
                super::bindings::menu_buttons(|b| reading.pressed(b), self.pad.stick);
            self.pad.suppress();
        }
        self.cancel_capture();
        self.controller_history
            .note(format!("Device selection: {id:?} (None = automatic)"));
    }

    pub(super) fn devices(&self) -> Vec<Device> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(sdl) = &self.sdl {
            return sdl.devices();
        }
        self.gilrs.iter().flat_map(|g| g.gamepads()).map(|(id, pad)| Device {
            id: usize::from(id), name: pad.name().into(), reading: Reading::gilrs(&pad),
            details: json!({
                "vendor_id": pad.vendor_id(), "product_id": pad.product_id(),
                "mapping_source": format!("{:?}", pad.mapping_source()),
                "mapping_guid": pad.uuid().iter().map(|b| format!("{b:02x}")).collect::<String>(),
                "power": format!("{:?}", pad.power_info()), "vibration_supported": pad.is_ff_supported(),
            }),
        }).collect()
    }

    pub(super) fn current_reading(&self) -> Option<Reading> {
        self.devices()
            .into_iter()
            .find(|d| Some(d.id) == self.pad.active)
            .map(|d| d.reading)
    }

    pub(super) fn next_controller_event(&mut self) -> Option<Event> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(sdl) = &mut self.sdl {
            return sdl.next_event();
        }
        let gilrs = self.gilrs.as_mut()?;
        loop {
            let event = gilrs.next_event()?;
            let change = match event.event {
                EventType::Connected => Change::Connected,
                EventType::Disconnected => Change::Disconnected,
                EventType::ButtonPressed(b, _) => Change::Button(b, true),
                EventType::ButtonReleased(b, _) => Change::Button(b, false),
                EventType::AxisChanged(axis, _, _) => StickAxis::ALL
                    .iter()
                    .position(|a| a.axis() == axis)
                    .map_or(Change::Other, Change::Axis),
                EventType::ButtonChanged(..) => Change::Other,
                _ => continue,
            };
            let reading = gilrs
                .connected_gamepad(event.id)
                .as_ref()
                .map(Reading::gilrs)
                .unwrap_or_default();
            return Some(Event {
                id: usize::from(event.id),
                change,
                reading,
                raw_reading: None,
                description: format!("{:?}", event.event),
            });
        }
    }
}
