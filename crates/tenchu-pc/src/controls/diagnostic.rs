use super::{
    Controls,
    backend::{Device, Event, Kind, Reading},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, Once},
};

#[derive(Default, Serialize)]
struct Activity {
    events: u64,
    last_event_ms: u64,
    pressed: u32,
    released: u32,
    axes_min: [f32; 4],
    axes_max: [f32; 4],
    triggers_max: [f32; 2],
    reading: Reading,
}

#[derive(Serialize)]
struct Attempt {
    backend: Kind,
    started_ms: u64,
    initialization_error: Option<String>,
    devices: Vec<Device>,
    activity: BTreeMap<usize, Activity>,
    recent_events: VecDeque<Value>,
    notes: String,
    driver: Value,
    keyboard_pressed: u32,
    keyboard_released: u32,
}

#[derive(Serialize)]
pub(super) struct History {
    #[serde(skip)]
    started: web_time::Instant,
    pub polls: u64,
    last_poll_ms: u64,
    maximum_poll_gap_ms: u64,
    focused: bool,
    attempts: VecDeque<Attempt>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            started: web_time::Instant::now(),
            polls: 0,
            last_poll_ms: 0,
            maximum_poll_gap_ms: 0,
            focused: false,
            attempts: VecDeque::new(),
        }
    }
}

impl History {
    fn now(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn begin(&mut self, backend: Kind, error: Option<String>, devices: Vec<Device>) {
        if self.attempts.len() == 8 {
            self.attempts.pop_front();
        }
        self.attempts.push_back(Attempt {
            backend,
            started_ms: self.now(),
            initialization_error: error,
            devices,
            activity: BTreeMap::new(),
            recent_events: VecDeque::new(),
            notes: String::new(),
            driver: Value::Null,
            keyboard_pressed: 0,
            keyboard_released: 0,
        });
    }

    pub fn driver(&mut self, driver: Value) {
        if let Some(attempt) = self.attempts.back_mut() {
            attempt.driver = driver;
        }
    }

    pub fn devices(&mut self, devices: Vec<Device>) {
        if let Some(attempt) = self.attempts.back_mut() {
            for known in &mut attempt.devices {
                known.details["connected"] = json!(false);
            }
            for mut device in devices {
                device.details["connected"] = json!(true);
                if let Some(known) = attempt.devices.iter_mut().find(|d| d.id == device.id) {
                    *known = device;
                } else if attempt.devices.len() < 64 {
                    attempt.devices.push(device);
                }
            }
        }
    }

    pub fn poll(&mut self, focused: bool) {
        let now = self.now();
        if self.polls != 0 {
            self.maximum_poll_gap_ms = self.maximum_poll_gap_ms.max(now - self.last_poll_ms);
        }
        if self.focused != focused {
            self.note(format!("Window focus: {focused}"));
        }
        self.focused = focused;
        self.last_poll_ms = now;
        self.polls += 1;
    }

    pub fn event(&mut self, event: &Event) {
        let now = self.now();
        if let Some(attempt) = self.attempts.back_mut() {
            let activity = attempt.activity.entry(event.id).or_default();
            activity.events += 1;
            activity.last_event_ms = now;
            activity.pressed |= event.reading.buttons & !activity.reading.buttons;
            activity.released |= activity.reading.buttons & !event.reading.buttons;
            for i in 0..4 {
                activity.axes_min[i] = activity.axes_min[i].min(event.reading.axes[i]);
                activity.axes_max[i] = activity.axes_max[i].max(event.reading.axes[i]);
            }
            for i in 0..2 {
                activity.triggers_max[i] = activity.triggers_max[i].max(event.reading.triggers[i]);
            }
            activity.reading = event.reading;
        }
        self.push(json!({"ms": now, "device": event.id, "event": event.description, "reading": event.reading}));
    }

    pub fn note(&mut self, note: String) {
        self.push(json!({"ms": self.now(), "note": note}));
    }

    pub fn keyboard(&mut self, actions: u32, pressed: bool) {
        if let Some(attempt) = self.attempts.back_mut() {
            if pressed {
                attempt.keyboard_pressed |= actions;
            } else {
                attempt.keyboard_released |= actions;
            }
        }
        if actions != 0 {
            self.push(json!({"ms": self.now(), "keyboard_actions": actions, "pressed": pressed}));
        }
    }

    fn push(&mut self, event: Value) {
        if let Some(attempt) = self.attempts.back_mut() {
            if attempt.recent_events.len() == 96 {
                attempt.recent_events.pop_front();
            }
            attempt.recent_events.push_back(event);
        }
    }
}

#[derive(Clone, Serialize)]
struct Message {
    unix_ms: u64,
    source: String,
    message: String,
    count: u64,
}
static MESSAGES: Mutex<VecDeque<Message>> = Mutex::new(VecDeque::new());
static INSTALL: Once = Once::new();
struct Logger;
static LOGGER: Logger = Logger;

pub(super) fn message(source: &str, message: String) {
    let Ok(mut messages) = MESSAGES.lock() else {
        return;
    };
    let now = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    if let Some(last) = messages.back_mut()
        && last.source == source
        && last.message == message
    {
        last.count += 1;
        last.unix_ms = now;
        return;
    }
    if messages.len() == 64 {
        messages.pop_front();
    }
    messages.push_back(Message {
        unix_ms: now,
        source: source.into(),
        message: message.chars().take(2048).collect(),
        count: 1,
    });
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Warn && metadata.target().starts_with("gilrs")
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            message(record.target(), record.args().to_string());
        }
    }
    fn flush(&self) {}
}

pub(super) fn install_logger() {
    INSTALL.call_once(|| {
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        } else {
            message(
                "diagnostics",
                "Another logger is installed; gilrs internal messages may be unavailable".into(),
            );
        }
    });
}

impl Controls {
    pub fn diagnostic_settings(&self) -> Value {
        let devices = self.devices();
        let details: Vec<_> = devices
            .iter()
            .map(|device| {
                let mut details = device.details.clone();
                details["id"] = json!(device.id);
                details["name"] = json!(device.name);
                details["active"] = json!(self.pad.active == Some(device.id));
                details["reading"] = json!(device.reading);
                details["held_buttons"] = json!(device.reading.names());
                details
            })
            .collect();
        let environment: BTreeMap<_, _> = [
            "TENCHU_CONTROLLER_DRIVER",
            "SteamAppId",
            "SteamGameId",
            "SteamOverlayGameId",
            "SDL_GAMECONTROLLERCONFIG",
            "SDL_GAMECONTROLLERCONFIG_FILE",
            "SDL_GAMECONTROLLER_IGNORE_DEVICES",
            "SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT",
        ]
        .map(|name| (name, std::env::var_os(name).is_some()))
        .into();
        json!({
            "bindings": self.bindings, "controllers": devices.iter().map(|d| &d.name).collect::<Vec<_>>(),
            "controller_details": details, "controller_backend": self.backend,
            "controller_backend_available": self.backend_available(), "initialization_error": self.backend_error,
            "gilrs_version": "0.11.2", "gilrs_patch": "5c38dfd3684114dba10ad281e6b3714e8334bf56 (worker shutdown only)", "gilrs_platform": if cfg!(target_os = "windows") { "WGI" } else if cfg!(target_arch = "wasm32") { "Web Gamepad API" } else if cfg!(target_os = "macos") { "IOKit" } else { "evdev" },
            "driver": self.driver_details(), "selected_device": self.selected_device, "active_device": self.pad.active,
            "gameplay_mask": self.pad.buttons(), "keyboard_action_mask": self.keys.iter().fold(0, |bits, key| bits | self.bindings.key(*key)),
            "input_test_open": self.editor.open, "vibration_error": self.vibration_error,
            "button_mask_order": super::backend::BUTTONS.iter().map(|b| format!("{b:?}")).collect::<Vec<_>>(),
            "axis_order": ["left_x", "left_y_up", "right_x", "right_y_up"], "trigger_order": ["left", "right"],
            "history": self.controller_history, "backend_messages": MESSAGES.lock().map(|m| m.clone()).unwrap_or_default(),
            "environment_flags_present": environment,
            "steam_detection_note": "Environment flags do not prove Steam Input or the overlay is active. Record launch mode and Steam layout in test notes.",
        })
    }

    fn backend_available(&self) -> bool {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if self.sdl.is_some() {
            return true;
        }
        self.gilrs.is_some()
    }

    pub(super) fn driver_details(&self) -> Value {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(sdl) = &self.sdl {
            return sdl.hints();
        }
        json!({"library": "gilrs", "version": "0.11.2"})
    }

    pub fn observe_test_key(&mut self, code: winit::keyboard::KeyCode, pressed: bool) {
        if self.editor.open && self.test_keyboard_active && self.bindings.key(code) != 0 {
            let changed = if pressed {
                self.test_keys.insert(code)
            } else {
                self.test_keys.remove(&code)
            };
            if changed {
                self.controller_history
                    .keyboard(self.bindings.key(code), pressed);
            }
        }
    }

    pub fn take_report_request(&mut self) -> bool {
        std::mem::take(&mut self.report_requested)
    }

    pub(super) fn controller_ui(&mut self, ui: &mut egui::Ui) {
        self.test_keyboard_active = false;
        ui.collapsing("Controller connection and input test", |ui| {
            self.test_keyboard_active = !ui.ctx().wants_keyboard_input();
            let mut backend = self.backend;
            egui::ComboBox::from_id_salt("controller-backend").selected_text(backend.label()).show_ui(ui, |ui| {
                for &kind in Kind::choices() { ui.selectable_value(&mut backend, kind, kind.label()); }
            });
            if backend != self.backend { self.change_backend(backend); }
            if let Some(error) = &self.backend_error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
            let devices = self.devices();
            let mut selected = self.selected_device;
            egui::ComboBox::from_id_salt("controller-device")
                .selected_text(selected.map_or_else(|| "Automatic — press a button".into(), |id| devices.iter().find(|d| d.id == id).map_or_else(|| format!("#{id} disconnected"), |d| format!("#{id} {}", d.name))))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, "Automatic — press a button");
                    for device in &devices { ui.selectable_value(&mut selected, Some(device.id), format!("#{} {}", device.id, device.name)); }
                });
            if selected != self.selected_device { self.select_device(selected); }
            ui.horizontal(|ui| {
                if ui.button("Retry connection").clicked() { self.start_backend(self.backend); }
                if ui.button("Release stuck input").clicked() { self.pad.suppress(); self.stop_rumble(); }
            });
            ui.weak("Switches apply immediately. SDL automatic supports more devices; XInput/WGI only isolate Windows drivers. Keyboard bindings work alongside either backend.");
            ui.weak("Steam: use a Gamepad layout, with extra buttons mapped to keyboard keys. Keep gamepad outputs enabled. SDL automatic respects Steam device exclusions.");
            if devices.is_empty() { ui.label("No controllers detected by this backend."); }
            for device in &devices {
                if device.details["opened"] == false {
                    ui.weak("Joystick detected but not opened as a gamepad. Its identity and mapping status are included in the report.");
                }
                let held = device.reading.names().join(", ");
                ui.label(format!("#{} {}{}", device.id, device.name, if self.pad.active == Some(device.id) { " (active)" } else { "" }));
                ui.monospace(format!("Buttons: {}\nSticks: {:+.2?}  Triggers L/R: {:.2?}",
                    if held.is_empty() { "released" } else { &held }, device.reading.axes, device.reading.triggers));
                if let Some(activity) = self.controller_history.attempts.back().and_then(|a| a.activity.get(&device.id)) {
                    ui.weak(format!("{} events; last {:.1}s ago; seen press/release: {:05x}/{:05x}", activity.events,
                        (self.controller_history.now() - activity.last_event_ms) as f32 / 1000.0, activity.pressed, activity.released));
                }
            }
            ui.label("Test each D-pad direction, Start, L2/R2 and extra buttons; release each. Readings continue while this panel pauses gameplay.");
            let keyboard = self.test_keys.iter().fold(0, |bits, key| bits | self.bindings.key(*key));
            ui.monospace(format!("Bound keyboard actions held: {keyboard:05x}"));
            if let Some(attempt) = self.controller_history.attempts.back_mut() {
                ui.add(egui::TextEdit::multiline(&mut attempt.notes).char_limit(2048).desired_rows(2)
                    .hint_text("Test notes: controller model, USB/dongle/Bluetooth, Steam launch/layout, what works or fails"));
            }
            if ui.button("Save diagnostic report…").clicked() { self.report_requested = true; }
            ui.weak("Includes the last 8 backend attempts, device activity and errors. Save once after comparing modes; save a separate report after restarting or changing Steam launch mode. Nothing is uploaded.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::backend::Change;
    use gilrs::Button;

    #[test]
    fn report_retains_failures_and_press_release_evidence_when_recent_events_roll_over() {
        let mut history = History::default();
        history.begin(Kind::Gilrs, Some("backend unavailable".into()), vec![]);
        history.begin(Kind::Sdl, None, vec![]);
        let mut event = Event {
            id: 7,
            change: Change::Other,
            reading: Reading::default(),
            description: "test input".into(),
        };
        event.reading.set_button(Button::DPadLeft, true);
        history.event(&event);
        event.reading.set_button(Button::DPadLeft, false);
        history.event(&event);
        history.keyboard(1 << 17, true);
        history.keyboard(1 << 17, false);
        for _ in 0..120 {
            history.event(&event);
        }
        let value = serde_json::to_value(&history).unwrap();
        assert_eq!(
            value["attempts"][0]["initialization_error"],
            "backend unavailable"
        );
        let attempt = &value["attempts"][1];
        assert_eq!(attempt["recent_events"].as_array().unwrap().len(), 96);
        assert_eq!(attempt["activity"]["7"]["events"], 122);
        assert_ne!(attempt["activity"]["7"]["pressed"], 0);
        assert_eq!(
            attempt["activity"]["7"]["pressed"],
            attempt["activity"]["7"]["released"]
        );
        assert_eq!(attempt["keyboard_pressed"], 1 << 17);
        assert_eq!(attempt["keyboard_released"], 1 << 17);
        for _ in 0..10 {
            history.begin(Kind::Gilrs, None, vec![]);
        }
        assert_eq!(history.attempts.len(), 8);
    }
}
