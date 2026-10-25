use super::*;
use crate::demo::remake::Feature as F;
use crate::visuals::{models::Models, surfaces::Surfaces};
use tenchu_core::game::Game;

pub(super) fn setup(
    clip: &demo::Clip,
    game: &mut Game,
    scene: &mut demo::Scene,
    models: &Models,
    vertices: &[render_mesh::Vertex],
) -> Result<()> {
    let Feature::Remake(feature) = clip.feature else {
        return Ok(());
    };
    let origin = Vec3::from(game.player().position.map(|n| n as f32).to_array());
    match feature {
        F::Botanical | F::TreeVariation | F::BranchWind | F::SeasonalFall | F::SnowDrift => {
            let labels: &[&str] = match clip.mission {
                3 => &["AKIWOD00"],
                4 => &["KTAKE"],
                5 => &["BTUTU"],
                _ => &["SAKURA", "TUTUZI"],
            };
            let sources = crate::visuals::catalog::models(&game.mission);
            let mut plants = Vec::new();
            render_mesh::visit_stage(&game.mission, &game.math, |geometry, matrix| {
                if sources.iter().any(|s| {
                    labels.contains(&s.label.as_str()) && s.geometry.as_ptr() == geometry.as_ptr()
                }) {
                    let mut points = Vec::new();
                    render_mesh::append_geometry(&mut points, geometry, matrix, Some(models));
                    let (min, max) = points.iter().fold(
                        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                        |(min, max), v| {
                            let p = Vec3::from(v.position);
                            (min.min(p), max.max(p))
                        },
                    );
                    if min.is_finite() {
                        plants.push((min, max));
                    }
                }
            });
            plants.sort_by(|(a, b), (c, d)| {
                ((a + b) * 0.5)
                    .distance_squared(origin)
                    .total_cmp(&((c + d) * 0.5).distance_squared(origin))
            });
            let &(min, max) = plants
                .first()
                .ok_or("Botanical demo needs a visible plant model")?;
            scene.target = (min + max) * 0.5;
            scene.distance = (max - min).length().max(2500.0) * 1.7;
            scene.pitch = -0.12;
            if feature == F::SeasonalFall {
                let mut canopies = models.canopies(game);
                canopies.sort_by(|a, b| {
                    ((a.min + a.max) * 0.5)
                        .distance_squared(origin)
                        .total_cmp(&((b.min + b.max) * 0.5).distance_squared(origin))
                });
                let surfaces = Surfaces::new(vertices);
                let mut ground = None;
                'trees: for canopy in canopies {
                    let center = (canopy.min + canopy.max) * 0.5;
                    for offset in [
                        Vec3::X * 600.0,
                        Vec3::NEG_X * 600.0,
                        Vec3::Z * 900.0,
                        Vec3::NEG_Z * 900.0,
                        Vec3::ZERO,
                    ] {
                        let start = center + offset;
                        let Some(hit) = surfaces.cast_excluding(
                            start,
                            start + Vec3::Y * 20000.0,
                            render_mesh::FOLIAGE,
                        ) else {
                            continue;
                        };
                        if hit.flags & render_mesh::WATER != 0 || hit.normal.y > -0.7 {
                            continue;
                        }
                        if game
                            .teleport_player(hit.position.as_ivec3().to_array().into(), true)
                            .is_ok()
                        {
                            ground = Some(hit.position);
                            break 'trees;
                        }
                    }
                }
                let ground =
                    ground.ok_or("Seasonal demo could not find walkable ground under foliage")?;
                for _ in 0..5 {
                    game.step(0)?;
                }
                scene.target = ground - Vec3::Y * 1400.0;
                scene.distance = 7500.0;
                scene.pitch = -0.25;
            } else if matches!(feature, F::TreeVariation | F::SnowDrift) {
                if let Some(&(min2, max2)) = plants.get(1) {
                    scene.target = (scene.target + (min2 + max2) * 0.5) * 0.5;
                    scene.distance = scene.distance.max((max2 - min).length() * 1.4);
                }
                scene.distance = scene.distance.min(19000.0);
                scene.pitch = -0.65;
            }
            demo::framing::unobstructed_area(scene, vertices, 700.0);
        }
        F::SightBounds => {
            let canopies = models.canopies(game);
            let position =
                |id| Vec3::from(game.roster.get(id).position.map(|n| n as f32).to_array());
            let actor = game
                .roster
                .order
                .iter()
                .copied()
                .filter(|&id| {
                    let actor = game.roster.get(id);
                    id != game.player_id
                        && actor.health > 0
                        && actor.patrol.is_some()
                        && actor.weapons.iter().any(Option::is_some)
                        && actor.attribute & tenchu_core::ids::ATTR_CUSTOMAI != 0
                        && canopies.iter().all(|c| {
                            let delta = position(id) - (c.min + c.max) * 0.5;
                            Vec3::new(delta.x, 0.0, delta.z).length() > 6000.0
                        })
                })
                .min_by(|&a, &b| {
                    position(a)
                        .distance_squared(origin)
                        .total_cmp(&position(b).distance_squared(origin))
                });
            if let Some(actor) = actor {
                scene.actor = Some(actor);
                scene.target = position(actor) - Vec3::Y * 900.0;
                scene.distance = 11000.0;
                scene.pitch = -0.85;
                demo::framing::unobstructed_area(scene, vertices, 1200.0);
            }
        }
        F::SnowTracks | F::ClearWeather | F::SnowScatter | F::PuddleScatter => {
            let forward = Vec3::from(
                game.math
                    .velocity(game.player().rotation[1], 2000, 0)
                    .map(f32::from),
            );
            scene.target = origin + forward - Vec3::Y * 300.0;
            scene.distance = 10500.0;
            scene.pitch = -0.85;
            demo::framing::unobstructed_area(scene, vertices, 1800.0);
        }
        F::Grapple => {
            game.set_inventory(0, 255)?;
            game.step(tenchu_core::input::TRIANGLE)?;
            for _ in 0..30 {
                game.step(tenchu_core::input::TRIANGLE | tenchu_core::input::RIGHT)?;
            }
            scene.target = origin + Vec3::new(3000.0, -2500.0, 0.0);
            scene.distance = 10000.0;
            scene.pitch = -0.30;
            demo::framing::unobstructed_area(scene, vertices, 1500.0);
        }
        F::WaterContact => {
            let water = render_mesh::stage(&game.mission, &game.math, Some(models));
            let center = water
                .chunks_exact(3)
                .filter(|t| {
                    t[0].flags & render_mesh::WATER != 0 && Vec3::from(t[0].normal).y.abs() > 0.7
                })
                .map(|t| t.iter().map(|v| Vec3::from(v.position)).sum::<Vec3>() / 3.0)
                .min_by(|a, b| {
                    a.distance_squared(origin)
                        .total_cmp(&b.distance_squared(origin))
                })
                .ok_or("Water-contact demo needs a water surface")?;
            game.teleport_player(center.as_ivec3().to_array().into(), true)?;
            for _ in 0..60 {
                game.step(0)?;
            }
            scene.target =
                Vec3::from(game.player().position.map(|n| n as f32).to_array()) - Vec3::Y * 300.0;
            scene.distance = 6000.0;
            scene.pitch = -0.65;
            demo::framing::unobstructed_area(scene, vertices, 1000.0);
        }
        _ => {}
    }
    scene.player_start = game.player().position;
    Ok(())
}

impl Application {
    pub(super) fn advance_remake_tools(&mut self) -> Result<()> {
        let player = self.demo.as_ref().unwrap();
        let Feature::Remake(feature) = player.clip().feature else {
            return Ok(());
        };
        let frame = player.frame;
        let length = u64::from(player.clip().seconds) * 60;
        match feature {
            F::ClearWeather if frame == length / 2 => self.view.as_mut().unwrap().clear_weather(),
            F::Grapple | F::WaterContact if frame == length / 2 => self.start_demo_clip()?,
            F::GameplayProfiles => {
                let index = demo::section(player.progress(), 5);
                if frame == 1 || frame.is_multiple_of(length / 5) {
                    self.set_gameplay_preset(index)?;
                }
                self.menu.page = 0;
                self.menu.row = 0;
                self.menu.open = true;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; verifies showcase contacts use real gameplay events and stay in frame"]
    fn remake_contact_demo_preflight() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        for id in [
            "remake-grapple",
            "remake-water-contact",
            "remake-snow-tracks",
            "remake-petal-fall",
            "remake-leaf-fall",
        ] {
            let mut player = demo::Player::parse(&["--clips".into(), id.into()]).unwrap();
            let clip = player.clip().clone();
            let (mut game, _) = player.load_game(&assets, None).unwrap();
            let mut settings = Settings::default();
            settings.set_visual_preset(8);
            let models =
                Models::build(&game.mission, &Default::default(), &settings.graphics).unwrap();
            let vertices = render_mesh::stage(&game.mission, &game.math, Some(&models));
            let mut scene = demo::Scene::default();
            setup(&clip, &mut game, &mut scene, &models, &vertices).unwrap();
            scene.current_player = Vec3::from(game.player().position.map(|n| n as f32).to_array());
            scene.previous_player = scene.current_player;
            player.scene = scene;
            let mut wire = false;
            let mut splash = false;
            let mut impacts = 0;
            let surfaces = Surfaces::new(&vertices);
            let mut contact = crate::visuals::contact::Contact::default();
            let mut flash = false;
            let camera = demo::camera(&player, &game, true)
                .unwrap_or_else(|| debug::camera::Camera::from_game(&game));
            let matrix = camera.matrix(16.0 / 9.0);
            let mut bounds = Vec3::ZERO;
            for tick in 0..u64::from(clip.seconds) * 15 {
                game.step(player.buttons(tick)).unwrap();
                if id == "remake-grapple" {
                    let camera = debug::camera::Camera::from_game(&game);
                    contact.tick(
                        &game,
                        &surfaces,
                        &settings.graphics,
                        Default::default(),
                        camera.position,
                    );
                    if !flash && let Some(source) = contact.lights().first() {
                        let ndc = camera
                            .matrix(16.0 / 9.0)
                            .project_point3(source.light.position);
                        eprintln!(
                            "grapple impact at tick {tick}: {:?}, NDC {ndc}",
                            source.light.position
                        );
                        assert!(
                            ndc.x.abs() < 0.9 && ndc.y.abs() < 0.9 && (0.0..1.0).contains(&ndc.z)
                        );
                        flash = true;
                    }
                }
                for effect in game.effects.slots.iter().flatten() {
                    match effect {
                        tenchu_core::effects::Effect::FlyWire(w) => {
                            wire = true;
                            impacts += u32::from(w.mode == 1);
                        }
                        tenchu_core::effects::Effect::Splash(_) => splash = true,
                        _ => {}
                    }
                }
                let actor = Vec3::from(game.player().position.map(|n| n as f32).to_array())
                    - Vec3::Y * 600.0;
                bounds = bounds.max(matrix.project_point3(actor).abs());
            }
            eprintln!(
                "{id}: player {:?}, camera {:?}, target {:?}, NDC bounds {bounds}, wire {wire}, impacts {impacts}, splash {splash}",
                game.player().position,
                camera.position,
                player.scene.target
            );
            if id == "remake-grapple" {
                assert!(
                    wire && impacts > 0 && flash,
                    "grapple demo did not show a visible lit impact"
                );
            }
            if id == "remake-water-contact" {
                assert!(splash, "water demo did not generate a splash");
            }
            if id == "remake-snow-tracks" {
                assert!(bounds.x < 0.9 && bounds.y < 0.9, "snow walk left the frame");
            }
        }
    }
}
