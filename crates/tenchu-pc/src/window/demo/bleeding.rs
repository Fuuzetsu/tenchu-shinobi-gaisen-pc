use crate::demo::Scene;
use tenchu_assets::Result;
use tenchu_core::{game::Game, math::Position};

pub(super) fn setup(game: &mut Game, scene: &mut Scene) -> Result<()> {
    super::behavior::setup(game, scene)?;
    let yaw = game.player().rotation[1];
    let at = game.player().position.wrapping_add(Position::from(
        game.math.velocity(yaw, 2000, 0).map(i32::from),
    ));
    let guard = scene.actor.unwrap();
    game.move_actor(guard, at, yaw.wrapping_add(2048), true)?;
    game.roster.get_mut(guard).setup_think(0x4444)?;
    let player = game.roster.get_mut(game.player_id);
    player.invulnerable = false;
    player.health = 200;
    player.max_health = 200;
    scene.distance = 7000.0;
    Ok(())
}

pub(super) fn advance(game: &mut Game, tick: u64) {
    if tick == 150 {
        game.intelligence.ignore_player = true;
    }
    if tick == 240
        && let Some(state) = &mut game.roster.get_mut(game.player_id).bleeding
    {
        state.request_bandage();
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; the showcase must start a real wound and spend exactly one bandage"]
    fn bleeding_showcase_uses_actual_combat_and_treatment() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut demo =
            crate::demo::Player::parse(&["--clips".into(), "bleeding-bandages".into()]).unwrap();
        let (mut game, _) = demo.load_game(&assets, None, None).unwrap();
        setup(&mut game, &mut demo.scene).unwrap();
        let mut settings = Settings::default();
        crate::demo::animate(
            &demo,
            &mut settings,
            &mut crate::debug::Debug::new(&game).unwrap(),
        );
        game.set_bleeding(settings.bleeding);
        let mut seen = false;
        for tick in 0..360 {
            advance(&mut game, tick);
            let hp = game.player().health;
            game.step(demo.buttons(tick)).unwrap();
            let state = game.player().bleeding.as_ref().unwrap();
            seen |= state.active;
            if tick == 240 {
                assert!(!state.active);
                assert_eq!(state.bandages, 2);
                assert_eq!(game.player().health, hp);
            }
        }
        assert!(seen);
        assert!(!game.player().bleeding.as_ref().unwrap().active);
        assert!(game.player().health > 0 && game.player().health < 200);
    }

    use crate::settings::Settings;
}
