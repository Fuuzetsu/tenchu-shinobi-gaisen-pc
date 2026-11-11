use super::*;
use crate::visuals::appearance::picker;

pub(super) fn show(
    ui: &mut Ui,
    menu: &mut Menu,
    models: Option<&picker::State<'_>>,
    runtime: Runtime,
    edits: &mut Vec<Edit>,
) {
    ui.collapsing("Place / edit mission actors", |ui| {
        ui.weak(if runtime == Runtime::Tenchu2 {
            "Changes last until retry or the next area. Story actors and mission targets are protected."
        } else {
            "Changes last until retry. Ends input recording; no TRIAL records. Story actors are protected."
        });
        available_ui(ui, runtime, Item::SpawnActor, |ui| {
            let choices: Vec<_> = if runtime == Runtime::Tenchu2 {
                menu.actor_models.iter().map(|(id, name)| (id.as_str(), name.as_str()))
                    .chain(models.and_then(|models| models.library).into_iter()
                        .flat_map(|library| &library.bodies).map(|body| (body.id.as_str(), body.name.as_str())))
                    .collect()
            } else {
                models.and_then(|models| models.library).into_iter()
                    .flat_map(|library| &library.bodies)
                    .map(|body| (body.id.as_str(), body.name.as_str())).collect()
            };
            if choices.is_empty() {
                ui.label(if runtime == Runtime::Tenchu2 { "Start a mission to load its native actor catalog." } else { "Loading the disc's model library…" });
                return;
            }
            ui.horizontal(|ui| {
                ui.label("Search actors");
                ui.add(egui::TextEdit::singleline(&mut menu.spawn_search).desired_width(170.0));
            });
            let search = menu.spawn_search.to_lowercase();
            let selected = choices.iter().find(|(id, _)| *id == menu.spawn_model)
                .map_or("Choose an actor", |(_, name)| *name);
            egui::ComboBox::from_id_salt("spawn-actor-model")
                .width(270.0).selected_text(selected).show_ui(ui, |ui| {
                    for &(id, name) in &choices {
                        if name.to_lowercase().contains(&search) || id.to_lowercase().contains(&search) {
                            ui.selectable_value(&mut menu.spawn_model, id.to_owned(), name);
                        }
                    }
                });
            let chosen = choices.iter().any(|(id, _)| *id == menu.spawn_model);
            if ui.add_enabled(chosen, egui::Button::new("Place new actor")).clicked() {
                edits.push(Edit::Change(Item::SpawnActor, 1));
            }
            if runtime == Runtime::Tenchu2 {
                ui.weak("Uses native T2 AI, weapons and animations with the selected body and voice. Missing disc resources are reported when placing.");
            }
        });
        ui.horizontal(|ui| {
            ui.label("Facing (degrees)");
            ui.add(egui::DragValue::new(&mut menu.spawn_yaw).range(0..=359));
        });
        ui.horizontal_wrapped(|ui| {
            for (label, item) in [
                ("Pick point on floor…", Item::PickActorPosition),
                ("Copy selected position", Item::ActorPosition),
                ("Move selected here", Item::MoveActor),
                ("Remove selected", Item::RemoveActor),
            ] {
                available_ui(ui, runtime, item, |ui| {
                    if ui.button(label).clicked() { edits.push(Edit::Change(item, 1)); }
                });
            }
        });
        ui.weak("Uses the X/Y/Z and floor-snap controls below. Moving a patrol resets its route to guarding the new position.");
    });
    ui.separator();
}
