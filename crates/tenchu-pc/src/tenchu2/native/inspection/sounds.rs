use super::{Session, point};
use crate::debug::{Anchor, Label, overlay::Builder};
use glam::Vec3;
use tenchu_assets::tenchu2::tutorial::{Command, Lesson};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cue {
    Narration(u16),
    Effect(i16),
    Stop,
}

pub(crate) struct Marker {
    pub position: Vec3,
    pub cue: Cue,
    pub label: String,
    bounds: Option<[Vec3; 2]>,
    active: bool,
}

const COLOR: [f32; 3] = [0.15, 0.85, 1.0];
const HALF_SIZE: Vec3 = Vec3::splat(220.0);

fn lesson_bounds(lesson: &Lesson) -> Option<[Vec3; 2]> {
    if lesson.radius <= 0 {
        return None;
    }
    // DATAMNG's near() divides by 1000 toward zero, with strict X/Z radii.
    let axes = std::array::from_fn::<_, 3, _>(|axis| {
        let center = i32::from(lesson.position[axis]);
        let radius = if axis == 1 {
            0
        } else {
            i32::from(lesson.radius) - 1
        };
        let (lo, hi) = (center - radius, center + radius);
        [
            (lo * 1000 - if lo <= 0 { 999 } else { 0 }) as f32,
            (hi * 1000 + if hi >= 0 { 999 } else { 0 }) as f32,
        ]
    });
    Some([
        Vec3::from(axes.map(|v| v[0])),
        Vec3::from(axes.map(|v| v[1])),
    ])
}

pub(super) fn effect(kind: i16) -> Option<(&'static str, i16)> {
    Some(match kind {
        1 | 2 => ("Door", 0x28),
        6 | 7 => ("Collapsing platform", 0x3c),
        9 => ("Rolling rock", 0x37),
        10 | 11 => ("Spear trap", 0x3f),
        12 | 13 => ("Arrow trap", 0x3d),
        _ => return None,
    })
}

pub(crate) fn markers(session: &Session) -> Vec<Marker> {
    let mut markers = Vec::new();
    if let Some(training) = &session.world.tutorial {
        let data = &training.data;
        let mut lesson_marker = |target: &Lesson, cue, label, active| {
            if let Some([min, max]) = lesson_bounds(target) {
                markers.push(Marker {
                    position: (min + max) * 0.5 - Vec3::Y * 800.0,
                    cue: Cue::Narration(cue),
                    label,
                    bounds: Some([min, max]),
                    active,
                });
            }
        };
        for (course, &index) in data.starts.iter().enumerate() {
            let lesson = &data.lessons[index];
            lesson_marker(
                lesson,
                lesson.cue,
                format!("Sign {} · narration {}", course + 1, lesson.cue),
                training.state.lesson.is_none(),
            );
        }
        for (index, lesson) in data.lessons.iter().enumerate() {
            if matches!(lesson.command, Command::Reach | Command::ReachNext)
                && let Some(next) = data.lessons.get(index + 1)
            {
                let target = if matches!(lesson.command, Command::ReachNext) {
                    next
                } else {
                    lesson
                };
                lesson_marker(
                    target,
                    next.cue,
                    format!("Lesson {} target · next narration {}", index + 1, next.cue),
                    training.state.lesson == Some(index),
                );
            }
        }
    }
    for (index, trigger) in session.world.environment.triggers.iter().enumerate() {
        let Some((name, cue)) = effect(trigger.kind) else {
            continue;
        };
        let object =
            session
                .world
                .environment
                .objects
                .get(usize::from(if matches!(trigger.kind, 1 | 2) {
                    trigger.target & 255
                } else {
                    trigger.target
                }));
        let position = if matches!(trigger.kind, 1 | 2 | 6 | 7 | 9) {
            object.map_or(trigger.position, |object| object.position)
        } else {
            trigger.position
        };
        markers.push(Marker {
            position: point(position) - Vec3::Y * 800.0,
            cue: Cue::Effect(cue),
            label: format!("{name} #{index} · sound {cue:02x}"),
            bounds: None,
            active: trigger.clock & 0x8000 != 0,
        });
    }
    markers
}

pub(super) fn draw(session: &Session, drawing: &mut Builder) {
    for marker in markers(session) {
        let color = if marker.active {
            COLOR
        } else {
            [0.45, 0.65, 0.75]
        };
        if let Some([min, max]) = marker.bounds {
            drawing.box_at(min, max, color);
            drawing.line((min + max) * 0.5, marker.position, color);
        }
        drawing.box_at(
            marker.position - HALF_SIZE,
            marker.position + HALF_SIZE,
            COLOR,
        );
        if let Some(p) = drawing.project(marker.position - Vec3::Y * 330.0) {
            drawing.labels.push(Label(
                Anchor::Projected(p),
                format!(
                    "{}{}",
                    marker.label,
                    if marker.active { " [active]" } else { "" }
                ),
                color,
            ));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn pick(session: &Session, origin: Vec3, direction: Vec3) -> Option<Marker> {
    markers(session)
        .into_iter()
        .filter_map(|marker| {
            let distance = crate::debug::ray_box(
                origin,
                direction,
                marker.position - HALF_SIZE,
                marker.position + HALF_SIZE,
            )?;
            Some((distance, marker))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, marker)| marker)
}

#[cfg(test)]
mod tests;
