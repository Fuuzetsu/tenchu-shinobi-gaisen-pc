use super::Settings;
use crate::{campaign::Action, render_effects, render_effects::Geometry};
use tenchu_assets::font::Font;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Fps,
    Focus,
    Fullscreen,
    Vsync,
    Hud,
    Volume,
    Reset,
}

pub struct Row {
    pub item: Item,
    pub label: &'static str,
    pub value: String,
    pub help: &'static str,
}

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    pub row: usize,
    pub notice: String,
}

fn on(value: bool) -> String {
    if value { "On" } else { "Off" }.into()
}

pub fn rows(s: &Settings) -> Vec<Row> {
    [
        (
            Item::Fps,
            "FPS and tick counter",
            on(s.show_fps),
            "Display frames/second and game tick.",
        ),
        (
            Item::Focus,
            "Pause on defocus",
            on(s.pause_on_defocus),
            "Off: game and audio keep running.",
        ),
        (
            Item::Fullscreen,
            "Fullscreen",
            on(s.fullscreen),
            "Borderless desktop fullscreen.",
        ),
        (
            Item::Vsync,
            "VSync",
            on(s.vsync),
            "Display pacing only; logic stays 30 Hz.",
        ),
        (
            Item::Hud,
            "Gameplay HUD",
            on(s.show_hud),
            "Show health, inventory and proximity.",
        ),
        (
            Item::Volume,
            "Master volume",
            format!("{}%", s.volume),
            "Changes output volume; 0 is mute.",
        ),
        (
            Item::Reset,
            "Restore defaults",
            "Enter".into(),
            "Reset settings to the original experience.",
        ),
    ]
    .into_iter()
    .map(|(item, label, value, help)| Row {
        item,
        label,
        value,
        help,
    })
    .collect()
}

impl Menu {
    pub fn action(&mut self, action: Action, count: usize) -> Option<i32> {
        match action {
            Action::Back => self.open = false,
            Action::Up => self.row = (self.row + count - 1) % count,
            Action::Down => self.row = (self.row + 1) % count,
            Action::Left => return Some(-1),
            Action::Right | Action::Confirm => return Some(1),
            Action::Replay => {}
        }
        None
    }

    pub fn build(&self, font: &Font, settings: &Settings) -> Geometry {
        let rows = rows(settings);
        let mut lines = vec![
            (-146, -108, "SETTINGS / F1".into(), [1.0, 0.8, 0.4]),
            (
                -146,
                -90,
                "Changes apply now and are saved.".into(),
                [0.6, 0.65, 0.7],
            ),
        ];
        for (index, row) in rows.iter().enumerate() {
            let color = if index == self.row {
                [1.0, 0.8, 0.4]
            } else {
                [1.0; 3]
            };
            let y = -64 + index as i16 * 19;
            lines.push((
                -146,
                y,
                format!(
                    "{} {}",
                    if index == self.row { ">" } else { " " },
                    row.label
                ),
                color,
            ));
            lines.push((102, y, row.value.clone(), color));
        }
        let help = &rows[self.row.min(rows.len() - 1)].help;
        lines.push((-146, 70, (*help).into(), [0.6, 0.8, 0.85]));
        if !self.notice.is_empty() {
            lines.push((
                -146,
                86,
                self.notice.chars().take(46).collect(),
                [1.0, 0.65, 0.3],
            ));
        }
        lines.push((
            -146,
            102,
            "Arrows: change  F1/Esc/East: close".into(),
            [0.6, 0.65, 0.7],
        ));
        render_effects::panel(font, &lines)
    }
}
