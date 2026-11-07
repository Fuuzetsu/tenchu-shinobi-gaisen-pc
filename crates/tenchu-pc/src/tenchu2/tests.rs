use super::*;

#[test]
#[ignore = "requires both discs; imported patrols, ordinary combat, retry and campaign isolation"]
fn imported_routes_and_guard_combat_work_without_campaign_progress() {
    use crate::campaign::{Action, Campaign, Screen};
    use tenchu_core::{ids::ATTR_WEAPON_DRAWN, input};
    let gaisen = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let t2 = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    let disc = crate::source::open_disc(t2.as_ref()).unwrap();
    assert!(Sandbox::open(&disc, Some(4), Some(1), None).is_err());
    assert!(Sandbox::open(&disc, Some(7), None, None).is_err());
    for kind in [0x10, 0x11, 0x12] {
        let sandbox = Sandbox::open(&disc, None, Some(4), Some(kind)).unwrap();
        assert_eq!(sandbox.level.id, 1);
        let mut game = sandbox.load(&assets, Options::default()).unwrap();
        assert!(!game.mission.enemies.is_empty());
        let starts: Vec<_> = game
            .roster
            .actors
            .iter()
            .flatten()
            .filter(|a| a.patrol.is_some())
            .map(|a| (a.id, a.position))
            .collect();
        assert!(!starts.is_empty());
        game.roster.keep_all_active = true;
        game.intelligence.ignore_player = true;
        for _ in 0..180 {
            game.step(0).unwrap();
        }
        assert!(
            starts
                .iter()
                .any(|&(id, start)| game.roster.get(id).position != start)
        );
        for character in 0..2 {
            let options = Options {
                character,
                ..Default::default()
            };
            let mut duel = Sandbox::open(&disc, Some(0), None, None).unwrap();
            let layout = duel.layout.as_mut().unwrap();
            layout.actors.truncate(1);
            let actor = &mut layout.actors[0];
            actor.position = [layout.start[0], layout.start[1], layout.start[2] - 600];
            actor.facing = layout.facing;
            actor.route.clear();
            duel.guard = Some(kind);
            let mut game = duel.load(&assets, options).unwrap();
            let enemy = game
                .roster
                .order
                .iter()
                .copied()
                .find(|&id| game.roster.get(id).kind == kind)
                .unwrap();
            game.intelligence.ignore_player = true;
            game.roster.keep_all_active = true;
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let health = game.roster.get(enemy).health;
            game.roster.get_mut(enemy).attribute |= ATTR_WEAPON_DRAWN;
            for tick in 0..90 {
                game.step(if tick % 30 == 0 { input::SQUARE } else { 0 })
                    .unwrap();
            }
            assert!(
                game.roster.get(enemy).health < health,
                "hero{character}, guard{kind}"
            );
            duel.layout.as_mut().unwrap().actors[0].position[2] -= 900;
            let mut game = duel.load(&assets, options).unwrap();
            game.roster.keep_all_active = true;
            game.roster.get_mut(enemy).rotation[1] = 2048;
            let health = game.player().health;
            for tick in 0..1200 {
                game.step(if tick % 120 < 12 { input::UP } else { 0 })
                    .unwrap();
                if game.player().health < health {
                    break;
                }
            }
            assert!(
                game.player().health < health,
                "guard{kind} cannot hurt hero{character}: {:?}, {:?}, action {} at {:?}, player at {:?}",
                game.roster.get(enemy).status,
                game.roster.get(enemy).brain,
                game.roster.get(enemy).motion.action,
                game.roster.get(enemy).position,
                game.player().position
            );
        }
        let options = Options::default();
        let mut save = tenchu_core::progress::save::Save::default();
        save.progress.highest = [6, 4];
        save.progress.stock[0][1] = 47;
        let mut campaign = Campaign::new(&assets, options, Some(save.clone())).unwrap();
        campaign.start_sandbox(sandbox, &assets).unwrap();
        assert_eq!(campaign.progress, save.progress);
        let progress = postcard::to_allocvec(&campaign.progress).unwrap();
        campaign.game = game;
        campaign.screen = Screen::Playing;
        campaign.checkpoint(options);
        assert!(campaign.save_request.is_none());
        assert!(campaign.action(Action::Replay, &assets).unwrap());
        assert_eq!(campaign.game.tick, 0);
        assert!(campaign.game.mission.world_vram.is_some());
        assert!(campaign.action(Action::Back, &assets).unwrap());
        assert!(campaign.sandbox.is_none());
        assert!(campaign.game.mission.world_vram.is_none());
        assert_eq!(progress, postcard::to_allocvec(&campaign.progress).unwrap());
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; walks both heroes through every imported map"]
fn imported_maps_run_gaisen_gameplay_without_replacing_player_textures() {
    let gaisen = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let t2 = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    let disc = crate::source::open_disc(t2.as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    tenchu_assets::tenchu2::import(&disc).unwrap();
    assert_eq!(layouts.len(), 56);
    for id in Level::available(&disc).unwrap() {
        let sandbox = Sandbox::open(&disc, Some(id), None, None).unwrap();
        for character in 0..2 {
            let options = Options {
                character,
                ..Default::default()
            };
            let mut game = sandbox
                .load(&assets, options)
                .unwrap_or_else(|e| panic!("map{id},hero{character}: {e}"));
            let original = MissionAssets::load(&assets, 0, character, 1).unwrap();
            assert!(game.mission.vram == original.vram);
            assert!(game.mission.world_vram.is_some());
            let start = game.player().position;
            let surfaces = crate::visuals::surfaces::Surfaces::new(&crate::render_mesh::stage(
                &game.mission,
                &game.math,
                None,
            ));
            let p = glam::Vec3::new(start.x as f32, start.y as f32, start.z as f32);
            assert!(
                surfaces
                    .cast(p - glam::Vec3::Y * 150.0, p + glam::Vec3::Y * 150.0)
                    .is_some(),
                "map{id} start {start:?} lacks visible floor"
            );
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            assert_eq!(game.player().position.y, start.y, "map{id} start");
            if character == 1
                && matches!(id, 0 | 4 | 6 | 20)
                && let Some(dir) = std::env::var_os("TENCHU_T2_REVIEW")
            {
                std::fs::create_dir_all(&dir).unwrap();
                crate::capture::frame(
                    &game,
                    &std::path::Path::new(&dir).join(format!("map{id}.png")),
                    &assets,
                )
                .unwrap();
            }
            for buttons in [
                tenchu_core::input::UP,
                tenchu_core::input::UP | tenchu_core::input::CROSS,
                tenchu_core::input::RIGHT,
                tenchu_core::input::SQUARE,
                0,
            ] {
                for _ in 0..30 {
                    game.step(buttons)
                        .unwrap_or_else(|e| panic!("map{id},hero{character}: {e}"));
                }
            }
            let mut recorder = crate::rewind::Recorder::default();
            recorder.begin();
            game.capture(&mut recorder).unwrap();
            let snapshot = recorder.finish().unwrap();
            let mut restored: Game = snapshot.decode().unwrap();
            for _ in 0..15 {
                restored.step(0).unwrap();
                game.step(0).unwrap();
            }
            assert_eq!(
                postcard::to_allocvec(&restored).unwrap(),
                postcard::to_allocvec(&game).unwrap()
            );
            assert!(game.mission.scripts.events.is_empty());
            eprintln!(
                "Map {id}, hero {character}: {:?} -> {:?}",
                start,
                game.player().position
            );
        }
    }
}
