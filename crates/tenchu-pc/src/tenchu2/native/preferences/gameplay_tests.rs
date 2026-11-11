use super::*;
use crate::tenchu2::native::{
    campaign::{Content, Playback},
    gaisen,
};
use tenchu_core::tenchu2::mission::{Outcome, Phase};

fn apply(menu: &mut Menu, settings: &mut Settings, edit: Edit) -> Action {
    let mut action = Action::default();
    let mut paused = true;
    let mut controls = Controls::default();
    controls.touch(crate::touch::Sample {
        buttons: u32::from(tenchu_core::input::SQUARE),
        ..Default::default()
    });
    let mut presets =
        Manager::new(std::env::temp_dir().join("tenchu-native-gameplay-presets.json"));
    menu.apply(
        edit,
        &mut State {
            settings,
            paused: &mut paused,
            controls: &mut controls,
            presets: &mut presets,
            lights: 0,
            samples: &[1, 4],
            models: None,
        },
        &mut action,
    );
    if menu.panel.notice.is_empty() {
        assert_eq!(controls.buttons(), 0);
    }
    assert!(paused);
    action
}

#[test]
fn native_gameplay_presets_reject_unavailable_scenes_before_editing() {
    let mut menu = Menu::default();
    menu.tools.debug.speed = 200;
    for edit in [
        Edit::GameplayPreset(0),
        Edit::GameplayPreset(4),
        Edit::GameplayPreset(usize::MAX),
        Edit::Change(Item::GameplayPreset, 1),
        Edit::Reset(Item::GameplayPreset),
    ] {
        let mut settings = Settings {
            keep_enemies_active: true,
            ..Default::default()
        };
        let original = settings.clone();
        apply(&mut menu, &mut settings, edit);
        assert!(!menu.panel.notice.is_empty());
        assert_eq!(settings, original);
        assert_eq!(menu.tools.debug.speed, 200);
        assert!(!menu.tools.debug.click_teleport);
        menu.tools.apply(&mut Content::Failed, [800, 600]).unwrap();
    }
}

fn playback(imported: bool) -> Playback<crate::source::DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut loadout = [0; 21];
    loadout[0] = 255;
    loadout[1] = 7;
    loadout[4] = 9;
    loadout[20] = 2;
    let mut p = if imported {
        let source = gaisen::Source::open(
            std::env::var_os("TENCHU_DISC").unwrap().as_ref(),
            0,
            0,
            1,
            tenchu_assets::script::Language::English,
        )
        .unwrap();
        Playback::gaisen(disc, source, 1, 2, 17, Some(loadout), 4000).unwrap()
    } else {
        Playback::load(disc, 9, 2, 17, Some(loadout), None).unwrap()
    };
    for _ in 0..4000 {
        p.step(0).unwrap();
        if matches!(&p.content, Content::Gameplay(s) if !s.cinematic() && s.world.mission.input_allowed())
        {
            return p;
        }
    }
    panic!("could not reach playable scene");
}

fn session(content: &Content) -> &super::super::Session {
    let Content::Gameplay(session) = content else {
        panic!("not gameplay")
    };
    session
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; all presets, cycling and reset affect native and imported stages without restarting or changing inventory/graphics"]
fn native_gameplay_presets_apply_reset_and_preserve_mission() {
    for imported in [false, true] {
        let mut p = playback(imported);
        let mut menu = Menu::default();
        let mut settings = Settings::default();
        settings.set_visual_preset(settings::presets::MODERN_ULTRA);
        settings.player_model = "disc:1".into();
        settings.weapon_model = "disc:133".into();
        settings.voice_model = "tenchu2-jp:ayamea".into();
        settings.moveset_model = "tenchu2-jp:rikimaru".into();
        let graphics = settings.graphics.clone();
        let models = (
            settings.player_model.clone(),
            settings.weapon_model.clone(),
            settings.voice_model.clone(),
            settings.moveset_model.clone(),
        );
        for (index, preset) in settings::gameplay::PRESETS.iter().enumerate() {
            settings.independent_attacks = true;
            settings.quick_shuriken_dodge = true;
            settings.strict_corpse_sight = true;
            settings.guard_controls = true;
            settings.circle_action = tenchu_core::input::CircleAction::Crouch;
            settings.walking.enabled = true;
            settings.sheathing.enabled = true;
            settings.bleeding.enabled = true;
            settings.dismemberment.enabled = true;
            settings.dismemberment.stealth_head = 100;
            settings.damage_escalation.increment = 50;
            settings.multi_target_attacks = true;
            settings.attack_assist = true;
            settings.behavior_profiles.enabled = true;
            settings.reinforcement_limit = 100;
            settings.reinforcement_call.count = 3;
            p.configure(&settings).unwrap();
            let Content::Gameplay(s) = &mut p.content else {
                unreachable!()
            };
            s.world.controls.armor = true;
            s.world.actors[s.world.player].actor.health = 37;
            let before = (
                s.world.tick,
                s.world.random,
                s.world.actors[s.world.player].actor.position,
                postcard::to_allocvec(&s.world.actors[s.world.player].actor.inventory).unwrap(),
                postcard::to_allocvec(&s.world.mission).unwrap(),
                postcard::to_allocvec(&p.run).unwrap(),
            );
            menu.tools.sync(&p.content, p.generation);
            menu.tools.debug.speed = 300;
            menu.tools.debug.click_kill = true;
            menu.tools.debug.actors_edited = true;
            menu.tools.debug.hitboxes = true;
            menu.tools.change(Item::GiveItems, 1).unwrap();
            let action = apply(&mut menu, &mut settings, Edit::GameplayPreset(index));
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            assert!(!action.leave && !action.select_mission && !action.step);
            menu.apply_tools(&mut p.content, [800, 600]);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            p.configure(&settings).unwrap();
            menu.tools.sync(&p.content, p.generation);
            assert_eq!(
                settings::gameplay::current(&settings, &menu.tools.debug),
                Some(index)
            );
            assert_eq!(settings.graphics, graphics);
            assert_eq!(
                (
                    &settings.player_model,
                    &settings.weapon_model,
                    &settings.voice_model,
                    &settings.moveset_model
                ),
                (&models.0, &models.1, &models.2, &models.3)
            );
            let w = &session(&p.content).world;
            assert_eq!(
                (w.tick, w.random, w.actors[w.player].actor.position),
                (before.0, before.1, before.2)
            );
            assert_eq!(w.actors[w.player].actor.health, 37);
            assert_eq!(
                postcard::to_allocvec(&w.actors[w.player].actor.inventory).unwrap(),
                before.3
            );
            assert_eq!(postcard::to_allocvec(&w.mission).unwrap(), before.4);
            assert_eq!(postcard::to_allocvec(&p.run).unwrap(), before.5);
            assert_eq!(w.actors[w.player].actor.invulnerable, preset.practice);
            assert_eq!(w.ai.ignore_player, preset.practice);
            assert_eq!(w.ai.sight_percent, preset.sight);
            assert!(!w.controls.armor);
            assert!(w.controls.walking.is_none() && w.actors[w.player].actor.sheathing.is_none());
            assert!(w.actors[w.player].actor.bleeding.is_none());
            assert!(
                !w.ai.damage_escalation.enabled()
                    && !w.ai.quick_shuriken_dodge
                    && !w.ai.strict_corpse_sight
            );
            assert!(
                !w.controls.multi_target_attacks
                    && !w.controls.attack_assist
                    && !w.controls.dismemberment.active()
            );
            assert!(menu.tools.debug.actors_edited && menu.tools.debug.hitboxes);
            p.step(0).unwrap();
            assert_eq!(session(&p.content).world.keep_all_active, preset.active);
            assert_eq!(session(&p.content).circle_action, Default::default());
        }
        for (edit, expected) in [
            (Edit::Change(Item::GameplayPreset, 1), 0),
            (Edit::Change(Item::GameplayPreset, -1), 4),
            (Edit::Reset(Item::GameplayPreset), 0),
        ] {
            menu.tools.sync(&p.content, p.generation);
            apply(&mut menu, &mut settings, edit);
            assert!(menu.panel.notice.is_empty(), "{}", menu.panel.notice);
            menu.apply_tools(&mut p.content, [800, 600]);
            p.configure(&settings).unwrap();
            menu.tools.sync(&p.content, p.generation);
            assert_eq!(
                settings::gameplay::current(&settings, &menu.tools.debug),
                Some(expected)
            );
        }
        for dead in [false, true] {
            let Content::Gameplay(s) = &mut p.content else {
                unreachable!()
            };
            if dead {
                s.world.mission.phase = Phase::Playing { pending: None };
                s.world.actors[s.world.player].actor.health = 0;
            } else {
                s.world.mission.phase = Phase::Playing {
                    pending: Some(Outcome::Arrived),
                };
            }
            let before = postcard::to_allocvec(&s.world).unwrap();
            let previous = settings.clone();
            menu.tools.sync(&p.content, p.generation);
            apply(&mut menu, &mut settings, Edit::GameplayPreset(4));
            assert!(!menu.panel.notice.is_empty());
            assert_eq!(settings, previous);
            menu.tools.apply(&mut p.content, [800, 600]).unwrap();
            assert!(postcard::to_allocvec(&session(&p.content).world).unwrap() == before);
        }
        if imported {
            let Content::Gameplay(s) = &mut p.content else {
                unreachable!()
            };
            s.world.actors[s.world.player].actor.health = 37;
            s.gaisen.as_mut().unwrap().director.presenting = true;
            let before = postcard::to_allocvec(&s.world).unwrap();
            let previous = settings.clone();
            menu.tools.sync(&p.content, p.generation);
            apply(&mut menu, &mut settings, Edit::GameplayPreset(4));
            assert!(!menu.panel.notice.is_empty());
            assert_eq!(settings, previous);
            menu.tools.apply(&mut p.content, [800, 600]).unwrap();
            assert!(postcard::to_allocvec(&session(&p.content).world).unwrap() == before);
        }
    }
}
