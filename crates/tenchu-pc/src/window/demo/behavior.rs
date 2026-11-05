use crate::demo::{self, Scene};
use glam::Vec3;
use tenchu_assets::Result;
use tenchu_core::{game::Game, math::Position};

pub(super) fn setup(game: &mut Game, scene: &mut Scene) -> Result<()> {
    let id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| game.roster.get(id).kind == 0x10)
        .ok_or("Enemy behavior demo needs a sword guard")?;
    let origin = game.player().position;
    let yaw = game.player().rotation[1];
    let at = origin.wrapping_add(Position::from(
        game.math.velocity(yaw, 4000, 0).map(i32::from),
    ));
    for other in game.roster.order.clone() {
        if other != id && other != game.player_id {
            game.roster.get_mut(other).setup_think(0)?;
        }
    }
    game.move_actor(id, at, yaw.wrapping_add(2048), true)?;
    demo::sparring(game, id)?;
    game.teleport_player(origin, true)?;
    game.move_actor(id, at, yaw.wrapping_add(2048), true)?;
    scene.actor = Some(id);
    scene.player_start = origin;
    scene.target = Vec3::from(origin.to_array().map(|v| v as f32)) - Vec3::Y * 700.0;
    scene.distance = 11000.0;
    scene.pitch = -0.7;
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::{demo::Feature, settings::Settings};
    use tenchu_core::actor::State;

    #[test]
    #[ignore = "requires TENCHU_DISC; matched live combat scenes demonstrate attacks, retreats and side dashes"]
    fn behavior_profiles_demo_guards_act() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for name in [
            "original", "coward", "reckless", "cautious", "flanker", "watchman",
        ] {
            let mut demo =
                demo::Player::parse(&["--clips".into(), format!("behavior-{name}").into()])
                    .unwrap();
            let (mut game, _) = demo.load_game(&assets, None, None).unwrap();
            setup(&mut game, &mut demo.scene).unwrap();
            let mut settings = Settings::default();
            let mut debug = crate::debug::Debug::new(&game).unwrap();
            demo::animate(&demo, &mut settings, &mut debug);
            game.set_behavior_profiles(settings.behavior_profiles);
            assert!(matches!(demo.clip().feature, Feature::EnemyBehavior(_)));
            let guard = demo.scene.actor.unwrap();
            let start = game.roster.get(guard).position;
            let mut attacks = 0;
            let mut dashes = 0;
            for tick in 0..300 {
                game.step(demo.buttons(tick)).unwrap();
                attacks += usize::from(game.roster.get(guard).status == State::Attack);
                dashes += usize::from(matches!(
                    game.roster.get(guard).motion.action,
                    tenchu_core::ids::MOT_CHASE_DASH_LEFT | tenchu_core::ids::MOT_CHASE_DASH_RIGHT
                ));
            }
            let actor = game.roster.get(guard);
            eprintln!(
                "{name}: {attacks} attack frames, {dashes} side dash frames, {start:?} -> {:?}",
                actor.position
            );
            if name == "coward" {
                assert_eq!(attacks, 0);
                assert!(
                    game.math
                        .length_wide(actor.position.wrapping_sub(game.player().position))
                        > 5000
                );
            } else {
                assert!(attacks > 0, "{name} never attacked");
                if matches!(name, "flanker" | "watchman") {
                    assert!(dashes > 0, "{name} never sidestepped");
                }
            }
        }
    }
}
