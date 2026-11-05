use crate::demo::{self, Scene};
use glam::Vec3;
use tenchu_assets::Result;
use tenchu_core::{game::Game, math::Position};

pub(super) fn setup(game: &mut Game, scene: &mut Scene) -> Result<()> {
    let actors: Vec<_> = game
        .roster
        .order
        .iter()
        .copied()
        .filter(|&id| game.roster.get(id).kind == 0x10)
        .take(2)
        .collect();
    if actors.len() != 2 {
        return Err("Attack tracking demo needs two sword guards".into());
    }
    let origin = game.player().position;
    let yaw = game.player().rotation[1];
    let positions = [(1500, 700), (-1100, -250)].map(|(forward, side)| {
        origin.wrapping_add(Position::from(
            game.math.velocity(yaw, forward, side).map(i32::from),
        ))
    });
    for (&id, &at) in actors.iter().zip(&positions) {
        game.move_actor(id, at, yaw, true)?;
        demo::sparring(game, id)?;
        game.roster.get_mut(id).setup_think(0)?;
    }
    for _ in 0..90 {
        game.step(0)?;
    }
    game.roster.get_mut(game.player_id).rotation[1] = yaw;
    game.teleport_player(origin, true)?;
    for (&id, &at) in actors.iter().zip(&positions) {
        game.move_actor(id, at, yaw, true)?;
        game.roster.get_mut(id).invulnerable = false;
        game.roster.get_mut(id).attribute &= !tenchu_core::ids::ATTR_CUSTOMAI;
    }
    scene.actor = None;
    scene.player_start = origin;
    scene.target = Vec3::from(origin.to_array().map(|v| v as f32)) - Vec3::Y * 500.0;
    scene.distance = 6500.0;
    scene.pitch = -0.8;
    Ok(())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; matched positions/inputs select the rear guard originally and front guard with assist"]
    fn attack_assist_demo_selects_the_expected_guard() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for enabled in [false, true] {
            let id = if enabled {
                "tracking-assisted"
            } else {
                "tracking-original"
            };
            let mut demo = demo::Player::parse(&["--clips".into(), id.into()]).unwrap();
            let (mut game, _) = demo.load_game(&assets, None, None).unwrap();
            setup(&mut game, &mut Scene::default()).unwrap();
            game.combat.attack_assist = enabled;
            let origin = game.player().position;
            let yaw = game.player().rotation[1];
            for tick in 0..240 {
                game.step(demo.buttons(tick)).unwrap();
                if tick == 45 {
                    assert_eq!(game.player().motion.action, tenchu_core::ids::MOT_ATTACK);
                    let model = game.player().target.expect("expected a target");
                    let target = game
                        .roster
                        .order
                        .iter()
                        .map(|&id| game.roster.get(id))
                        .find(|a| a.root_model == model)
                        .unwrap();
                    let delta = target.position.wrapping_sub(origin);
                    let angle = game.math.direction(delta.x, delta.z, yaw).unwrap();
                    assert_eq!(angle.abs() < 682, enabled, "{id}: angle {angle}");
                    eprintln!(
                        "{id}: guard {}, angle {angle}, position {:?}",
                        target.id, target.position
                    );
                }
                if tick == 60 {
                    assert_eq!(game.player().status, tenchu_core::actor::State::Attack);
                    assert_eq!(game.player().rotation[1].wrapping_sub(yaw) < 0, enabled);
                    eprintln!(
                        "{id}: swing yaw {} (started {yaw})",
                        game.player().rotation[1]
                    );
                }
            }
        }
    }
}
