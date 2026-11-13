use egui::{Color32, Frame, Response, RichText, Stroke, Ui};

pub const INK: Color32 = Color32::from_rgb(22, 23, 25);
pub const PANEL: Color32 = Color32::from_rgb(30, 31, 33);
pub const GOLD: Color32 = Color32::from_rgb(213, 184, 127);
pub const BORDER: Color32 = Color32::from_rgb(76, 69, 56);
const PAPER: Color32 = Color32::from_rgb(234, 228, 214);

pub fn style(ui: &mut Ui) {
    let style = ui.style_mut();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style.visuals.override_text_color = Some(PAPER);
    style.visuals.selection.bg_fill = Color32::from_rgb(91, 43, 39);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, GOLD);
    style.visuals.widgets.inactive.weak_bg_fill = PANEL;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(65, 48, 41);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, GOLD);
    style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(104, 44, 38);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, GOLD);
    style.visuals.extreme_bg_color = INK;
}

pub fn tools<R>(ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(INK.gamma_multiply(0.96))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(5)
        .inner_margin(10)
        .show(ui, |ui| {
            style(ui);
            ui.set_width(244.0);
            ui.label(RichText::new("SHINOBI TOOLS").size(11.0).color(GOLD));
            body(ui)
        })
        .inner
}

pub fn tool(ui: &mut Ui, label: &str, shortcut: &str) -> Response {
    let response = ui.add_sized([244.0, 42.0], egui::Button::new("").corner_radius(3));
    let rect = response.rect.shrink2(egui::vec2(12.0, 0.0));
    ui.painter().text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(15.0),
        PAPER,
    );
    ui.painter().text(
        rect.right_center(),
        egui::Align2::RIGHT_CENTER,
        shortcut,
        egui::FontId::proportional(11.0),
        GOLD,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}
