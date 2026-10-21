use egui::{Color32, Stroke};
use std::collections::VecDeque;

const HISTORY: usize = 300;
const COLORS: [Color32; 5] = [
    Color32::from_rgb(120, 202, 150),
    Color32::from_rgb(98, 177, 244),
    Color32::from_rgb(225, 174, 95),
    Color32::from_rgb(198, 140, 230),
    Color32::from_rgb(170, 180, 192),
];

#[derive(Clone, Copy, Default)]
pub(super) struct CpuFrame {
    pub interval: f32,
    pub simulation: f32,
    pub prepare: f32,
    pub overlay: f32,
    pub draw: DrawTimes,
}

#[derive(Clone, Copy, Default)]
pub(super) struct DrawTimes {
    pub submit: f32,
    pub wait: f32,
}

#[derive(Default)]
pub(super) struct Performance {
    cpu: VecDeque<CpuFrame>,
    gpu: VecDeque<[f32; 4]>,
    serial: u64,
    frozen: bool,
}

impl Performance {
    pub fn record(&mut self, cpu: CpuFrame, gpu: Option<crate::renderer::timing::Sample>) {
        if self.frozen {
            return;
        }
        push(&mut self.cpu, cpu);
        if let Some(sample) = gpu
            && sample.serial != self.serial
        {
            self.serial = sample.serial;
            push(&mut self.gpu, sample.milliseconds);
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        interactive: bool,
        gpu_supported: bool,
        offline: bool,
    ) {
        egui::Window::new("Performance")
            .open(open)
            .default_width(440.0)
            .default_pos(egui::pos2(
                (ctx.screen_rect().right() - 470.0).max(10.0),
                10.0,
            ))
            .interactable(interactive)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.frozen, "Freeze");
                    if ui.button("Clear").clicked() {
                        self.cpu.clear();
                        self.gpu.clear();
                    }
                    ui.weak("F1 to interact · 300 frame history");
                });
                let frames: Vec<_> = self.cpu.iter().map(|f| f.interval).collect();
                let (average, p95) = stats(frames.iter().copied());
                ui.label(format!(
                    "{} interval: {average:.2} ms average · {p95:.2} ms p95",
                    if offline { "Capture" } else { "Frame" },
                ));
                let cpu: Vec<[f32; 5]> = self
                    .cpu
                    .iter()
                    .map(|f| {
                        [
                            f.simulation,
                            f.prepare,
                            f.overlay,
                            f.draw.submit,
                            f.draw.wait,
                        ]
                    })
                    .collect();
                chart(ui, &cpu, Some(&frames));
                legend(
                    ui,
                    "cpu",
                    &[
                        "Simulation + audio",
                        "Geometry + uploads",
                        "Overlay CPU",
                        "Encode + submit",
                        if offline {
                            "Capture + encoder wait"
                        } else {
                            "Acquire + present wait"
                        },
                    ],
                    &cpu,
                );
                ui.separator();
                if gpu_supported {
                    let gpu: Vec<_> = self.gpu.iter().copied().collect();
                    let (average, p95) = stats(gpu.iter().map(|v| v.iter().sum()));
                    ui.label(format!(
                        "GPU scene: {average:.2} ms average · {p95:.2} ms p95"
                    ));
                    chart(ui, &gpu, None);
                    legend(
                        ui,
                        "gpu",
                        &[
                            "Shadow maps",
                            "World + effects",
                            "Image effects",
                            "Game HUD",
                        ],
                        &gpu,
                    );
                    ui.weak("GPU samples arrive asynchronously; F1/font rendering is excluded.");
                } else {
                    ui.weak("GPU timestamps unavailable on this adapter; CPU timing still works.");
                }
                ui.weak("CPU and GPU overlap. White line: frame interval; guide: 16.7 ms.");
            });
    }
}

fn push<T>(history: &mut VecDeque<T>, value: T) {
    if history.len() == HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

fn stats(values: impl Iterator<Item = f32>) -> (f32, f32) {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let average = values.iter().sum::<f32>() / values.len() as f32;
    values.sort_unstable_by(f32::total_cmp);
    (
        average,
        values[(values.len() * 95).div_ceil(100).saturating_sub(1)],
    )
}

fn legend<const N: usize>(ui: &mut egui::Ui, id: &str, names: &[&str; N], values: &[[f32; N]]) {
    egui::Grid::new(id)
        .num_columns(3)
        .spacing([14.0, 2.0])
        .show(ui, |ui| {
            ui.weak("Stage");
            ui.weak("Avg ms");
            ui.weak("p95 ms");
            ui.end_row();
            for (i, name) in names.iter().enumerate() {
                let (average, p95) = stats(values.iter().map(|v| v[i]));
                ui.colored_label(COLORS[i], *name);
                ui.monospace(format!("{average:.2}"));
                ui.monospace(format!("{p95:.2}"));
                ui.end_row();
            }
        });
}

fn chart<const N: usize>(ui: &mut egui::Ui, values: &[[f32; N]], line: Option<&[f32]>) {
    let (response, painter) =
        ui.allocate_painter(egui::vec2(ui.available_width(), 85.0), egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 3.0, Color32::from_rgb(14, 17, 22));
    let max = values
        .iter()
        .map(|v| v.iter().sum::<f32>())
        .chain(line.into_iter().flatten().copied())
        .fold(33.34f32, f32::max);
    let y = |ms: f32| rect.bottom() - (ms / max).clamp(0.0, 1.0) * rect.height();
    let dx = rect.width() / HISTORY as f32;
    let start = rect.right() - values.len() as f32 * dx;
    for (i, frame) in values.iter().enumerate() {
        let mut total = 0.0;
        for (j, duration) in frame.iter().enumerate() {
            let bottom = y(total);
            total += duration;
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(start + i as f32 * dx, y(total)),
                    egui::pos2(start + (i + 1) as f32 * dx, bottom),
                ),
                0.0,
                COLORS[j],
            );
        }
    }
    painter.line_segment(
        [
            egui::pos2(rect.left(), y(16.667)),
            egui::pos2(rect.right(), y(16.667)),
        ],
        Stroke::new(1.0_f32, Color32::from_white_alpha(70)),
    );
    if let Some(line) = line
        && line.len() > 1
    {
        painter.add(egui::Shape::line(
            line.iter()
                .enumerate()
                .map(|(i, ms)| egui::pos2(start + (i as f32 + 0.5) * dx, y(*ms)))
                .collect(),
            Stroke::new(1.0_f32, Color32::WHITE),
        ));
    }
    painter.text(
        rect.left_top() + egui::vec2(4.0, 2.0),
        egui::Align2::LEFT_TOP,
        format!("{max:.0} ms"),
        egui::FontId::monospace(11.0),
        Color32::WHITE,
    );
}
