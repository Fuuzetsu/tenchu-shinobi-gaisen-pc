pub(crate) fn preview(ui: &mut egui::Ui, seconds: &mut u16) -> Option<u16> {
    let mut seek = None;
    ui.label("Preview weather elapsed time");
    ui.small("Start from dry ground at the current weather settings. Preview snow, wetness and puddles without advancing the mission.");
    let response = ui.add(
        egui::Slider::new(seconds, 0..=3600)
            .custom_formatter(|n, _| format!("{}:{:02}", n as u32 / 60, n as u32 % 60)),
    );
    if response.drag_stopped() || response.changed() && !ui.input(|i| i.pointer.any_down()) {
        seek = Some(*seconds);
    }
    ui.horizontal_wrapped(|ui| {
        for (value, label) in [(0, "Dry"), (30, "30 s"), (60, "1 min"), (300, "5 min")] {
            if ui.button(label).clicked() {
                *seconds = value;
                seek = Some(value);
            }
        }
        if ui.button("Apply").clicked() {
            seek = Some(*seconds);
        }
    });
    seek
}
