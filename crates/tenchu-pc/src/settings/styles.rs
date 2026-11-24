use super::Settings;
use crate::visuals::appearance::library::{Body, Library};
use tenchu_assets::Result;

pub const UNUSED: &str = "unused";
pub const HELP: &str = "Choose up to three complete movesets, including Campaign original. Bind Next combat style in F1 → Controls (unbound by default). Each press cycles different available styles while standing or moving on the ground; attacks, guard dodges, crouching, injury, jumping and scenes cannot be interrupted. Missing/incompatible sources are skipped. Body, independent equipment/voice and unarmed preferences stay selected. No switching during input recording, rewind preview, demos or randomized runs. Reset a slot to remove it.";

pub fn choices(models: &[(String, String)]) -> Vec<(String, String)> {
    let mut choices = vec![
        (UNUSED.into(), "Unused".into()),
        (String::new(), "Campaign original".into()),
    ];
    choices.extend(models.iter().filter(|(id, _)| !id.is_empty()).cloned());
    choices
}

pub fn change(settings: &mut Settings, slot: usize, models: &[(String, String)], delta: i32) {
    let choices = choices(models);
    let current = settings.combat_styles[slot].as_deref().unwrap_or(UNUSED);
    let index = choices
        .iter()
        .position(|(id, _)| id == current)
        .unwrap_or(0);
    let id = &choices[(index as i32 + delta).rem_euclid(choices.len() as i32) as usize].0;
    settings.combat_styles[slot] = (id != UNUSED).then(|| id.clone());
}

pub fn next(
    settings: &Settings,
    library: &Library,
    compatible: impl Fn(&Body) -> bool,
) -> Result<(String, String)> {
    let mut cycle = Vec::new();
    for id in settings.combat_styles.iter().flatten() {
        let choice = if id.is_empty() {
            (String::new(), "Campaign original".into())
        } else if let Some(body) = library.get(id).filter(|b| compatible(b)) {
            (body.id.clone(), body.name.clone())
        } else {
            continue;
        };
        if !cycle.iter().any(|(id, _)| id == &choice.0) {
            cycle.push(choice);
        }
    }
    if cycle.len() < 2 {
        return Err("Choose at least two different available styles in F1 → Player model → Combat style slots. Missing or incompatible sources are skipped.".into());
    }
    let current = settings.moveset_id().unwrap_or_default();
    let current = library.get(current).map_or(current, |body| &body.id);
    let next = cycle
        .iter()
        .position(|(id, _)| id == current)
        .map_or(0, |index| (index + 1) % cycle.len());
    Ok(cycle.swap_remove(next))
}

pub fn select(settings: &mut Settings, id: String) {
    settings.model_moveset = !id.is_empty();
    settings.moveset_model = id;
}

pub fn gaisen_ready(game: &tenchu_core::game::Game) -> bool {
    use tenchu_core::{actor::State, ai::Halt};
    let player = game.player();
    game.player_can_steer()
        && !game.intelligence.cinematic
        && player.map.height == 0
        && matches!(
            player.status,
            State::Normal | State::Walk | State::Engage | State::Chase
        )
        && game.intelligence.action_halt == Halt::Running
}

pub fn switch_gaisen(
    game: &mut tenchu_core::game::Game,
    settings: &mut Settings,
    library: &Library,
) -> Result<String> {
    if !gaisen_ready(game) {
        return Err("Combat style unchanged: stand or move on the ground after the current action finishes.".into());
    }
    let (id, name) = next(settings, library, Body::has_moveset)?;
    let mut selected = settings.clone();
    select(&mut selected, id);
    if let Err(error) = library.apply_player(game, &selected) {
        let _ = library.apply_player(game, settings);
        return Err(error);
    }
    *settings = selected;
    Ok(format!("Combat style: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{edit::Preference, menu::Item};

    #[test]
    fn combat_style_slots_default_restore_and_reset_without_changing_active_moves() {
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.combat_styles, [None, None, None]);
        let models = vec![
            (String::new(), "Selected body".into()),
            ("disc:1".into(), "Ayame".into()),
        ];
        change(&mut settings, 0, &models, 1);
        change(&mut settings, 1, &models, -1);
        Preference::CombatStyle(2, Some("missing:source".into())).apply(&mut settings);
        assert_eq!(
            settings.combat_styles,
            [
                Some(String::new()),
                Some("disc:1".into()),
                Some("missing:source".into())
            ]
        );
        assert!(!settings.model_moveset);
        let saved: Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        settings.set_visual_preset(3);
        settings.retro = true;
        assert_eq!(settings.combat_styles, saved.combat_styles);
        assert!(Item::CombatStyle(1).reset_setting(&mut settings));
        assert_eq!(settings.combat_styles[1], None);
        settings.restore_playback_preferences(&saved);
        assert_eq!(settings.combat_styles, saved.combat_styles);
    }
}
