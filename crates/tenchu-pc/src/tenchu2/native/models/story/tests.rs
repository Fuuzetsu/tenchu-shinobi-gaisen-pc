use super::*;

#[test]
#[ignore = "requires both discs; selected cutscene equipment covers authored held/stowed states, dual swords, independent choices and unarmed overrides"]
fn native_story_equipment_follows_selection_and_authored_state() {
    use tenchu_assets::tenchu2::campaign::{Campaign, Kind};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let bodies = Arc::default();
    let mut covered = [[false; 2]; 3];
    let mut events = BTreeSet::new();
    for row in campaign
        .sequences
        .iter()
        .filter(|row| row.kind == Kind::Event)
    {
        if !events.insert(row.resource) {
            continue;
        }
        let resources = Resources::load(&disc, row.resource).unwrap();
        for (hero, states) in covered.iter_mut().enumerate() {
            for (slot, track) in resources.script.actors.iter().enumerate() {
                let Some(actor) = resources
                    .models
                    .get(&track.model)
                    .filter(|a| is_hero(&a.name, hero))
                else {
                    continue;
                };
                let held = resources
                    .attachments
                    .iter()
                    .any(|a| a.actor == slot && a.slot >= 3);
                if states[usize::from(held)] {
                    continue;
                }
                states[usize::from(held)] = true;
                eprintln!(
                    "event {} hero {hero} {} held {held}: {:?}",
                    row.resource,
                    actor.name,
                    resources
                        .attachments
                        .iter()
                        .filter(|a| a.actor == slot)
                        .map(|a| (&a.model, a.slot, a.bone))
                        .collect::<Vec<_>>()
                );
                for selected in [
                    "disc:1",
                    "tenchu2-jp:kagami",
                    "tenchu2-jp:yorojyu",
                    "tenchu2-jp:tatu2",
                    "tenchu2-jp:tatumaru",
                ] {
                    let settings = Settings {
                        player_model: "disc:1".into(),
                        weapon_model: selected.into(),
                        ..Default::default()
                    };
                    let source = library.weapon(selected).unwrap().unwrap();
                    let gear = source.equipment.as_ref().unwrap();
                    let mut model = Override::default();
                    assert!(
                        model
                            .sync(&resources, hero, &library, &bodies, &settings)
                            .unwrap()
                    );
                    assert!(model.current(hero, &settings));
                    assert!(
                        !model
                            .sync(&resources, hero, &library, &bodies, &settings)
                            .unwrap()
                    );
                    model.validate_state(&resources).unwrap();
                    if source.resource == format!("tenchu2/actors/{}", actor.name) {
                        continue;
                    }
                    let attached = &model.equipment[&slot];
                    assert!(attached.iter().all(|a| {
                        gear.attachments.iter().any(|g| {
                            g.model.id == a.model.id
                                && tenchu_assets::tenchu2::BONES[g.part] == a.bone
                                && g.position == a.position
                        })
                    }));
                    let mut visible_slots = [0, 1, 2, 3, 4];
                    if held {
                        gear.draw.swap(&mut visible_slots);
                    }
                    let visible = gear
                        .attachments
                        .iter()
                        .filter(|a| [visible_slots[0], visible_slots[1], 4].contains(&a.slot))
                        .count();
                    assert_eq!(attached.len(), visible, "{selected}, held {held}");
                    if selected == "disc:1" {
                        assert_eq!(attached.len(), if held { 2 } else { 0 });
                    }
                    let restored: Override =
                        postcard::from_bytes(&postcard::to_allocvec(&model).unwrap()).unwrap();
                    assert!(restored.same_body(&model).unwrap());
                    model
                        .sync(&resources, hero, &library, &bodies, &Settings::default())
                        .unwrap();
                    assert!(model.equipment.is_empty());
                }
            }
        }
        if covered.iter().all(|states| states[0]) && covered[..2].iter().all(|states| states[1]) {
            break;
        }
    }
    assert!(
        covered.iter().all(|states| states[0]),
        "missing sheathed cases: {covered:?}"
    );
    assert!(covered.iter().any(|states| states[1]), "missing drawn case");
}
