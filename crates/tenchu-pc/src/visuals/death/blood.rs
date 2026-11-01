use super::Frame;
use std::sync::Arc;
use tenchu_core::{
    effects::{Billboard, Drawing, Effect, Effects, Environment, View},
    game::Game,
    math::Matrix,
    random::Random,
    scene::{ModelId, Scene},
    terrain::Cursor,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Presented {
    source: ModelId,
    effects: Effects,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub(super) struct Blood {
    presented: Arc<Presented>,
    random: Random,
    scene: Scene,
    cursor: Cursor,
}

impl Blood {
    pub fn new(game: &Game) -> Self {
        Self {
            presented: Arc::new(Presented {
                source: game.player().parts[1],
                effects: Effects::default(),
            }),
            random: Random::new(super::roll(game.random.seed, game.tick) as u32),
            scene: Scene::default(),
            cursor: Cursor::new(&game.mission.terrain),
        }
    }

    pub fn tick(
        &mut self,
        game: &Game,
        torso: Matrix,
        emit: bool,
    ) -> tenchu_assets::Result<Arc<Presented>> {
        let effects = &mut Arc::make_mut(&mut self.presented).effects;
        effects.begin_tick();
        self.scene.update(0, torso);
        if emit {
            effects.gore(
                0,
                [0, -200, -240],
                [0, 0, -100],
                game.clock,
                &mut self.random,
                &mut self.scene,
            )?;
        }
        effects.step(
            &mut self.random,
            View {
                position: game.camera.position,
                matrix: game.camera.view,
            },
            &mut self.scene,
            &mut Environment {
                terrain: &game.mission.terrain,
                cursor: &mut self.cursor,
                clock: game.clock,
                owner_position: game.player().position,
                reference: game.camera.reference,
            },
        )?;
        self.scene.finish_tick();
        Ok(Arc::clone(&self.presented))
    }
}

fn replacement(frame: Option<&Frame>) -> Option<&Presented> {
    frame.and_then(|f| f.death_blood.as_deref())
}

pub fn slots<'a>(
    effects: &'a Effects,
    frame: Option<&'a Frame>,
) -> impl Iterator<Item = Option<&'a Effect>> {
    let death = replacement(frame);
    effects
        .active_slots()
        .iter()
        .enumerate()
        .map(move |(i, slot)| {
            slot.as_ref()
                .filter(|_| death.is_none_or(|d| effects.gore_sources[i] != Some(d.source)))
        })
        .chain(
            death
                .into_iter()
                .flat_map(|d| d.effects.active_slots().iter().map(Option::as_ref)),
        )
}

pub fn drawings<'a>(
    effects: &'a Effects,
    frame: Option<&'a Frame>,
) -> impl Iterator<Item = &'a Drawing> {
    let death = replacement(frame);
    effects
        .drawings
        .iter()
        .filter(move |d| death.is_none_or(|b| d.gore_source != Some(b.source)))
        .chain(death.into_iter().flat_map(|d| &d.effects.drawings))
}

pub fn billboards<'a>(
    effects: &'a Effects,
    frame: Option<&'a Frame>,
) -> impl Iterator<Item = &'a Billboard> {
    let death = replacement(frame);
    effects
        .billboards
        .iter()
        .filter(move |b| death.is_none_or(|d| b.gore_source != Some(d.source)))
        .chain(death.into_iter().flat_map(|d| &d.effects.billboards))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::math::Position;

    #[test]
    #[ignore = "requires TENCHU_DISC; death blood follows its emitted pose, keeps world trajectories, filters only its owner and leaves gameplay/RNG unchanged"]
    fn alternate_death_blood_follows_the_visible_pose_without_dragging_particles() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        game.effects = Effects::default();
        for source in [game.player().parts[1], game.player().parts[2]] {
            game.effects
                .gore(
                    source,
                    [0, -200, -240],
                    [0, 0, -100],
                    0,
                    &mut game.random,
                    &mut game.scene,
                )
                .unwrap();
        }
        game.effects
            .step(
                &mut game.random,
                View {
                    position: game.camera.position,
                    matrix: game.camera.view,
                },
                &mut game.scene,
                &mut Environment {
                    terrain: &game.mission.terrain,
                    cursor: &mut Cursor::new(&game.mission.terrain),
                    clock: game.clock,
                    owner_position: game.roster.get(game.player_id).position,
                    reference: game.camera.reference,
                },
            )
            .unwrap();
        let before = crate::session::tests::state(&game);
        let mut blood = Blood::new(&game);
        let torso = Matrix {
            translation: game
                .player()
                .position
                .wrapping_add(Position::new(0, -2000, 0)),
            ..Matrix::default()
        };
        let mut frame = Frame::new(Default::default(), game.rendered_view);
        frame.death_blood = Some(blood.tick(&game, torso, true).unwrap());
        let (index, first) = blood
            .presented
            .effects
            .active_slots()
            .iter()
            .enumerate()
            .find_map(|(i, e)| {
                if let Some(Effect::Blood(b)) = e {
                    Some((i, b.clone()))
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            first.position,
            torso
                .transform([0, -200, -240])
                .wrapping_add(Position::new(0, 10, -100))
        );
        let source = game.player().parts[1];
        for (i, effect) in slots(&game.effects, Some(&frame)).take(200).enumerate() {
            assert_eq!(
                effect.is_some(),
                game.effects.slots[i].is_some() && game.effects.gore_sources[i] != Some(source)
            );
        }
        assert!(
            game.effects
                .billboards
                .iter()
                .any(|b| b.gore_source == Some(source))
        );
        assert!(billboards(&game.effects, Some(&frame)).all(|b| b.gore_source != Some(source)));
        assert!(
            billboards(&game.effects, Some(&frame))
                .any(|b| b.gore_source == Some(game.player().parts[2]))
        );
        assert!(drawings(&game.effects, Some(&frame)).all(|d| d.gore_source != Some(source)));
        let camera = crate::debug::camera::Camera {
            position: glam::Vec3::from(torso.translation.to_array().map(|v| v as f32))
                - glam::Vec3::Z * 5000.0,
            yaw: 0.0,
            pitch: 0.0,
            fov: 55.0,
        };
        for hit in [
            crate::settings::BloodStyle::Off,
            crate::settings::BloodStyle::Original,
            crate::settings::BloodStyle::OriginalHd,
            crate::settings::BloodStyle::Modern,
        ] {
            let geometry = crate::render_effects::build_presented(
                &game,
                false,
                Some(camera.source_view()),
                false,
                None,
                Some(&frame),
                crate::render_effects::Style {
                    blood: crate::settings::Blood {
                        hit,
                        ground: crate::settings::BloodStyle::Off,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            let flag = match hit {
                crate::settings::BloodStyle::Off => {
                    crate::render_mesh::STAIN | crate::render_mesh::SOFT
                }
                crate::settings::BloodStyle::Original => crate::render_mesh::STAIN,
                crate::settings::BloodStyle::OriginalHd => crate::render_mesh::BLOOD_HD,
                crate::settings::BloodStyle::Modern => crate::render_mesh::SOFT,
            };
            assert_eq!(
                geometry.vertices.iter().any(|v| v.flags & flag != 0),
                hit != crate::settings::BloodStyle::Off,
                "{hit:?}"
            );
            assert!(
                geometry
                    .vertices
                    .iter()
                    .flat_map(|v| v.position)
                    .all(f32::is_finite)
            );
        }
        let history = frame.clone();
        let moved = Matrix {
            translation: torso.translation.wrapping_add(Position::new(10000, 0, 0)),
            rotation: [[0, 0, 4096], [0, 4096, 0], [-4096, 0, 0]],
        };
        frame.death_blood = Some(blood.tick(&game, moved, false).unwrap());
        let Some(Effect::Blood(next)) = &blood.presented.effects.slots[index] else {
            panic!("airborne droplet");
        };
        assert_eq!(
            next.position,
            first
                .position
                .wrapping_add(Position::from(first.velocity.map(i32::from)))
                .wrapping_add(Position::new(0, 10, 0))
        );
        let Some(Effect::Blood(previous)) = slots(&game.effects, Some(&history))
            .nth(200 + index)
            .flatten()
        else {
            panic!("history snapshot");
        };
        assert_eq!(previous.position, first.position);
        // Continue the cosmetic stream until everything expires without more emission.
        for _ in 0..300 {
            blood.tick(&game, moved, false).unwrap();
        }
        assert!(
            blood
                .presented
                .effects
                .active_slots()
                .iter()
                .all(Option::is_none)
        );
        assert_eq!(crate::session::tests::state(&game), before);
    }
}
