use egui::{Color32, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};
use tenchu_core::input;
use winit::event::TouchPhase;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    #[default]
    Auto,
    Always,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub visibility: Visibility,
    pub size: u16,
    pub opacity: u16,
    pub left_handed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            visibility: Visibility::Auto,
            size: 100,
            opacity: 55,
            left_handed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Settings,
    Pause,
    Back,
    Recenter,
    QuickMenu,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Menus,
    Gameplay,
    Paused,
    FreeCamera,
}

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub buttons: u32,
    pub sticks: [glam::Vec2; 2],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Stick(usize),
    Button(u32),
    Action(Action),
}

struct Control {
    target: Target,
    center: Pos2,
    radius: f32,
    label: &'static str,
}

struct Finger {
    id: u64,
    target: Target,
    position: Pos2,
}

#[derive(Default)]
pub struct Overlay {
    detected: bool,
    controls: Vec<Control>,
    fingers: Vec<Finger>,
}

impl Overlay {
    pub fn clear(&mut self) {
        self.fingers.clear();
    }

    pub fn visible(&self, settings: Settings) -> bool {
        settings.visibility == Visibility::Always
            || settings.visibility == Visibility::Auto
                && (self.detected || cfg!(target_os = "android"))
    }

    pub fn layout(
        &mut self,
        rect: Rect,
        settings: Settings,
        mode: Mode,
        tools: bool,
        camera: bool,
    ) {
        self.controls.clear();
        if !(self.visible(settings) || self.detected || cfg!(target_os = "android")) {
            return;
        }
        let scale = (f32::from(settings.size.clamp(70, 140)) / 100.0)
            .min(rect.width() / 650.0)
            .min(rect.height() / 310.0)
            .max(0.5);
        let radius = 25.0 * scale;
        let y = rect.top() + radius + 6.0;
        for (offset, action, label) in [
            (0.0, Action::Settings, "Settings"),
            (65.0, Action::Pause, "Pause"),
            (130.0, Action::Back, "Back"),
        ] {
            self.controls.push(Control {
                target: Target::Action(action),
                center: Pos2::new(rect.right() - radius - 8.0 - offset * scale, y),
                radius,
                label,
            });
        }
        if tools || !self.visible(settings) {
            return;
        }
        let playing = matches!(mode, Mode::Gameplay | Mode::FreeCamera);
        let free = mode == Mode::FreeCamera;
        let mut add = |x: f32, y: f32, r: f32, target, label| {
            let x = if settings.left_handed {
                rect.width() - x
            } else {
                x
            };
            self.controls.push(Control {
                target,
                center: rect.min + Vec2::new(x, y),
                radius: r,
                label,
            });
        };
        let bottom = rect.height();
        let w = rect.width();
        add(
            95.0 * scale,
            bottom - 100.0 * scale,
            65.0 * scale,
            Target::Stick(0),
            "Move",
        );
        for (x, y, button, label) in [
            (
                w - 150.0 * scale,
                bottom - 110.0 * scale,
                input::SQUARE,
                "Attack",
            ),
            (
                w - 42.0 * scale,
                bottom - 110.0 * scale,
                input::CIRCLE,
                "Crouch",
            ),
            (
                w - 96.0 * scale,
                bottom - 56.0 * scale,
                input::CROSS,
                "Jump",
            ),
            (
                w - 96.0 * scale,
                bottom - 164.0 * scale,
                input::TRIANGLE,
                "Item",
            ),
        ] {
            if free && button == input::TRIANGLE {
                continue;
            }
            let (button, label) = if free {
                match button {
                    input::SQUARE => (input::L1, "Descend"),
                    input::CIRCLE => (button, "Boost"),
                    _ => (button, "Rise"),
                }
            } else {
                (button, label)
            };
            add(
                x,
                y,
                radius,
                Target::Button(u32::from(button)),
                if playing {
                    label
                } else {
                    match button {
                        input::CROSS => "×",
                        input::CIRCLE => "○",
                        input::SQUARE => "□",
                        _ => "△",
                    }
                },
            );
        }
        if playing {
            if !free {
                for (x, button, label) in [
                    (32.0, input::L1, "Look"),
                    (92.0, input::R1, "R1"),
                    (152.0, input::L2, "Item −"),
                    (212.0, input::R2, "Item +"),
                ] {
                    add(
                        x * scale,
                        42.0 * scale,
                        radius,
                        Target::Button(u32::from(button)),
                        label,
                    );
                }
                add(
                    210.0 * scale,
                    bottom - 36.0 * scale,
                    radius,
                    Target::Button(u32::from(input::SELECT)),
                    "Map",
                );
            }
            if camera || free {
                add(
                    w - 260.0 * scale,
                    bottom - 90.0 * scale,
                    52.0 * scale,
                    Target::Stick(1),
                    "Camera",
                );
                add(
                    w - 260.0 * scale,
                    bottom - 177.0 * scale,
                    radius,
                    Target::Action(Action::Recenter),
                    "Center",
                );
            }
        } else {
            add(
                210.0 * scale,
                bottom - 36.0 * scale,
                radius,
                Target::Button(u32::from(input::START)),
                "Start",
            );
            if mode == Mode::Menus {
                add(
                    w * 0.5,
                    42.0 * scale,
                    radius,
                    Target::Action(Action::QuickMenu),
                    "Quick",
                );
            }
        }
        self.fingers
            .retain(|f| self.controls.iter().any(|c| c.target == f.target));
    }

    /// A finger owns its original control until release, even when another finger enters it.
    pub fn event(&mut self, id: u64, phase: TouchPhase, position: Pos2) -> (bool, Option<Action>) {
        self.detected = true;
        match phase {
            TouchPhase::Started => {
                let Some(control) = self
                    .controls
                    .iter()
                    .find(|c| c.center.distance(position) <= c.radius)
                else {
                    return (false, None);
                };
                if matches!(control.target, Target::Stick(_))
                    && self.fingers.iter().any(|f| f.target == control.target)
                {
                    return (true, None);
                }
                self.fingers.push(Finger {
                    id,
                    target: control.target,
                    position,
                });
                (true, None)
            }
            TouchPhase::Moved => {
                let Some(finger) = self.fingers.iter_mut().find(|f| f.id == id) else {
                    return (false, None);
                };
                finger.position = position;
                (true, None)
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some(index) = self.fingers.iter().position(|f| f.id == id) else {
                    return (false, None);
                };
                let finger = self.fingers.remove(index);
                let action = (phase == TouchPhase::Ended)
                    .then(|| {
                        self.controls.iter().find(|c| {
                            c.target == finger.target && c.center.distance(position) <= c.radius
                        })
                    })
                    .flatten()
                    .and_then(|c| match c.target {
                        Target::Action(action) => Some(action),
                        _ => None,
                    });
                (true, action)
            }
        }
    }

    pub fn sample(&self) -> Sample {
        let mut sample = Sample::default();
        for finger in &self.fingers {
            let Some(control) = self.controls.iter().find(|c| c.target == finger.target) else {
                continue;
            };
            match finger.target {
                Target::Stick(index) => {
                    let delta = (finger.position - control.center) / control.radius;
                    sample.sticks[index] = glam::Vec2::new(delta.x, -delta.y).clamp_length_max(1.0);
                }
                Target::Button(button)
                    if control.center.distance(finger.position) <= control.radius * 1.5 =>
                {
                    sample.buttons |= button
                }
                _ => {}
            }
        }
        sample
    }

    pub fn paint(&self, ctx: &egui::Context, settings: Settings) {
        if self.controls.is_empty() {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("touch-controller"),
        ));
        let alpha = (u32::from(settings.opacity.clamp(15, 90)) * 255 / 100) as u8;
        for control in &self.controls {
            let active = self.fingers.iter().any(|f| f.target == control.target);
            painter.circle_filled(
                control.center,
                control.radius,
                Color32::from_black_alpha(if active { alpha } else { alpha / 2 }),
            );
            painter.circle_stroke(
                control.center,
                control.radius,
                egui::Stroke::new(1.5_f32, Color32::from_white_alpha(alpha)),
            );
            painter.text(
                control.center,
                egui::Align2::CENTER_CENTER,
                control.label,
                egui::FontId::proportional((control.radius * 0.44).clamp(10.0, 15.0)),
                Color32::from_white_alpha(alpha.max(180)),
            );
            if let Target::Stick(_) = control.target
                && let Some(finger) = self.fingers.iter().find(|f| f.target == control.target)
            {
                let delta = finger.position - control.center;
                let delta = delta.normalized() * delta.length().min(control.radius);
                painter.circle_filled(
                    control.center + delta,
                    control.radius * 0.32,
                    Color32::from_white_alpha(alpha),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multitouch_holds_movement_and_attack_independently_and_cancellation_releases() {
        let mut overlay = Overlay::default();
        overlay.layout(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 400.0)),
            Settings {
                visibility: Visibility::Always,
                ..Default::default()
            },
            Mode::Gameplay,
            false,
            true,
        );
        let move_at = overlay
            .controls
            .iter()
            .find(|c| c.target == Target::Stick(0))
            .unwrap()
            .center
            + Vec2::new(25.0, -50.0);
        let attack_at = overlay
            .controls
            .iter()
            .find(|c| c.target == Target::Button(input::SQUARE.into()))
            .unwrap()
            .center;
        overlay.event(1, TouchPhase::Started, move_at);
        overlay.event(2, TouchPhase::Started, attack_at);
        let sample = overlay.sample();
        assert!(sample.sticks[0].x > 0.2 && sample.sticks[0].y > 0.5);
        assert_eq!(sample.buttons, u32::from(input::SQUARE));
        overlay.event(2, TouchPhase::Cancelled, attack_at);
        assert_eq!(overlay.sample().buttons, 0);
        assert_eq!(overlay.sample().sticks[0], sample.sticks[0]);
        overlay.clear();
        assert_eq!(overlay.sample().sticks, [glam::Vec2::ZERO; 2]);
    }
    #[test]
    fn cancelled_settings_gesture_does_not_open_the_menu() {
        let mut overlay = Overlay::default();
        overlay.layout(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 400.0)),
            Settings {
                visibility: Visibility::Always,
                ..Default::default()
            },
            Mode::Gameplay,
            true,
            false,
        );
        let at = overlay.controls[0].center;
        overlay.event(8, TouchPhase::Started, at);
        assert_eq!(overlay.event(8, TouchPhase::Cancelled, at), (true, None));
        overlay.event(9, TouchPhase::Started, at);
        assert_eq!(
            overlay.event(9, TouchPhase::Ended, at),
            (true, Some(Action::Settings))
        );
    }
}
