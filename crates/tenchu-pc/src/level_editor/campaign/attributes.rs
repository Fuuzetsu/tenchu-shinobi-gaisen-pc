use egui::Ui;

// T1 names follow decomp game_types.h / CONFLICT.C, verified against core/terrain
// and actor/{body,shadow,movement/chase}. T2 material is a field, not independent bits.
pub(super) fn show(ui: &mut Ui, attributes: &mut u16, t2: bool) {
    egui::CollapsingHeader::new("Collision attributes")
        .id_salt("campaign-collision-attributes")
        .default_open(true)
        .show(ui, |ui| {
            if t2 {
                let current = *attributes & 0x0f00;
                let name = |value| match value {
                    0 => "Solid ground",
                    0x100 => "Blocked / no floor",
                    0x200 => "Water surface",
                    0x300 => "Underwater",
                    0x400 => "Hazard",
                    _ => "Unidentified source material",
                };
                let mut material = current;
                egui::ComboBox::from_id_salt("collision-material")
                    .selected_text(name(current))
                    .show_ui(ui, |ui| {
                        for value in [0, 0x100, 0x200, 0x300, 0x400] {
                            ui.selectable_value(&mut material, value, name(value));
                        }
                    });
                *attributes = (*attributes & !0x0f00) | material;
                ui.small("T2 source materials are preserved. Studio's Gaisen playtest adapts solid/blocked collision; native T2 water and hazard rules are not simulated.");
            } else {
                for (mask, name, help) in [
                    (2, "Buoyant / no normal floor", "Uses the source buoyancy and no-floor handling."),
                    (4, "Swimming water", "Uses the original swimming-water behavior."),
                    (8, "Wood / hollow footsteps", "Uses hollow footsteps on authored wood and decks. The game also reuses this bit for shadow handling."),
                    (0x100, "Damage surface", "Uses the original damage-surface behavior."),
                    (0x200, "Death / kill floor", "Uses the original lethal-floor behavior."),
                    (0x2000, "Stop searching overlapping surfaces", "Accepts this source result immediately instead of scanning the remaining surfaces in the row."),
                ] {
                    let mut enabled = *attributes & mask != 0;
                    if ui.checkbox(&mut enabled, name).on_hover_text(help).changed() {
                        *attributes = (*attributes & !mask) | if enabled { mask } else { 0 };
                    }
                }
            }
            ui.small("Slope direction is set by Shape above. Other source bits are retained.");
            egui::CollapsingHeader::new("Advanced · raw source value")
                .id_salt("campaign-raw-attributes")
                .show(ui, |ui| {
                    ui.add(egui::DragValue::new(attributes).hexadecimal(4, false, true).prefix("0x"));
                });
        });
}
