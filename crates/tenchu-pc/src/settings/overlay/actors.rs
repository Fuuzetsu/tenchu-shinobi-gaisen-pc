use super::*;
use crate::visuals::appearance::picker;

pub(super) fn show(
    ui: &mut Ui,
    menu: &mut Menu,
    models: Option<&picker::State<'_>>,
    edits: &mut Vec<Edit>,
) {
    ui.collapsing("Place / edit mission actors", |ui| {
        ui.weak("Changes last until retry. Ends input recording; no TRIAL records. Story actors are protected.");
        if let Some(models) = models {
            if let Some(library) = models.library {
                ui.horizontal(|ui| {
                    ui.label("Search models");
                    ui.add(egui::TextEdit::singleline(&mut menu.spawn_search).desired_width(170.0));
                });
                let search = menu.spawn_search.to_lowercase();
                let selected = library.get(&menu.spawn_model).map_or("Choose a model", |b| b.name.as_str());
                egui::ComboBox::from_id_salt("spawn-actor-model")
                    .width(270.0).selected_text(selected).show_ui(ui, |ui| {
                        for body in &library.bodies {
                            if body.name.to_lowercase().contains(&search) || body.id.to_lowercase().contains(&search) {
                                ui.selectable_value(&mut menu.spawn_model, body.id.clone(), &body.name);
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Facing (degrees)");
                    ui.add(egui::DragValue::new(&mut menu.spawn_yaw).range(0..=359));
                    if ui.add_enabled(!menu.spawn_model.is_empty(), egui::Button::new("Place new actor")).clicked() {
                        edits.push(Edit::Change(Item::SpawnActor, 1));
                    }
                });
            } else {
                ui.label("Loading the disc's model library…");
            }
        }
        ui.horizontal_wrapped(|ui| {
            for (label, item) in [
                ("Pick point on floor…", Item::PickActorPosition),
                ("Copy selected position", Item::ActorPosition),
                ("Move selected here", Item::MoveActor),
                ("Remove selected", Item::RemoveActor),
            ] {
                if ui.button(label).clicked() { edits.push(Edit::Change(item, 1)); }
            }
        });
        ui.weak("Uses the X/Y/Z and floor-snap controls below. Moving a patrol resets its route to guarding the new position.");
    });
    ui.separator();
}
