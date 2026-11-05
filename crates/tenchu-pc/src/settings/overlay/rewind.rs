use super::{Debug, Edit, Item};
use crate::rewind::Timeline;

pub(super) fn overlay(ctx: &egui::Context, debug: &Debug, edits: &mut Vec<Edit>) {
    let Some(state) = debug.rewind.filter(|s| s.playing || s.preview) else {
        return;
    };
    if debug.photo_mode {
        return;
    }
    egui::Area::new("rewind-timeline".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width((ctx.screen_rect().width() - 48.0).clamp(250.0, 640.0));
                timeline(ui, state, edits);
            });
        });
}

pub(super) fn timeline(ui: &mut egui::Ui, state: Timeline, edits: &mut Vec<Edit>) {
    ui.label(format!(
        "Rewind timeline · {:.2}s before latest · {:.1}s retained",
        state.end.saturating_sub(state.tick) as f64 / 30.0,
        state.end.saturating_sub(state.start) as f64 / 30.0,
    ));
    let mut tick = state.tick.clamp(state.start, state.end);
    ui.scope(|ui| {
        ui.spacing_mut().slider_width = ui.available_width();
        if ui.add_enabled(state.start < state.end,
            egui::Slider::new(&mut tick, state.start..=state.end)
                .show_value(false)
                .step_by(1.0)
        ).on_hover_text("Seek backward or forward without discarding history. Resume here replaces the recorded future.").changed() {
            edits.push(Edit::SeekRewind(tick));
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (label, target, enabled) in [
            ("Oldest", state.start, tick > state.start),
            ("−1 frame", tick.saturating_sub(1), tick > state.start),
            ("+1 frame", tick.saturating_add(1), tick < state.end),
            ("Latest", state.end, tick < state.end),
        ] {
            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                edits.push(Edit::SeekRewind(target));
            }
        }
        if ui
            .add_enabled(
                state.playing || tick > state.start,
                egui::Button::new(if state.playing {
                    "Pause playback"
                } else {
                    "Rewind"
                }),
            )
            .clicked()
        {
            edits.push(Edit::Change(Item::Rewind, 1));
        }
        if ui
            .add_enabled(
                !state.playing && tick < state.end,
                egui::Button::new("Play forward"),
            )
            .clicked()
        {
            edits.push(Edit::PlayRewindForward);
        }
        if ui.button("Resume here (F3)").clicked() {
            edits.push(Edit::ResumeRewind);
        }
    });
    if state.playing {
        ui.weak(if state.forward {
            "Playing recorded states forward (audio muted)"
        } else {
            "Playing recorded states backward (audio muted)"
        });
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button("Start here").clicked() {
            edits.push(Edit::SelectRewind([tick, state.selection[1].max(tick)]));
        }
        if ui.button("End here").clicked() {
            edits.push(Edit::SelectRewind([state.selection[0].min(tick), tick]));
        }
        if ui.button("Whole interval").clicked() {
            edits.push(Edit::SelectRewind([state.start, state.end]));
        }
        if ui
            .add_enabled(
                state.selection[0] < state.selection[1],
                egui::Button::new("Export selected…"),
            )
            .clicked()
        {
            edits.push(Edit::ExportRewind);
        }
    });
    ui.weak(format!(
        "Export ticks {}–{} ({:.2}s). Includes restorable game data: share privately.",
        state.selection[0],
        state.selection[1],
        state.selection[1].saturating_sub(state.selection[0]) as f64 / 30.0
    ));
    if let Some(mut recorded) = state.recorded_visuals
        && ui.checkbox(&mut recorded, "Recorded graphics and camera").on_hover_text("Disable to inspect the recording with your own graphics or free camera. Replacements still come from your installed artwork pack.").changed() {
        edits.push(Edit::RecordedVisuals(recorded));
    }
    ui.weak("Seeking keeps the recorded future. Resume discards only the part after this point.");
}
