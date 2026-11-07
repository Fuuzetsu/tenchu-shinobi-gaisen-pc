use super::*;
use crate::visuals::{appearance, interpolation::Part, models::Models, pack::Pack};
use glam::Vec3;

#[test]
#[ignore = "requires both discs; severed arms/heads on imported rigs must remain separate from the corpse"]
fn detached_imported_limbs_do_not_stretch_skin_to_the_body() {
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let path = std::env::var_os("TENCHU2_DISC").unwrap();
    let disc = crate::source::open_disc(path.as_ref()).unwrap();
    let bodies = tenchu_assets::tenchu2::characters::load(&disc, None).unwrap();
    let mut sandbox = Sandbox::open(path.as_ref(), Some(0), None, Some(0x10)).unwrap();
    sandbox.layout.as_mut().unwrap().actors.truncate(1);
    for body in bodies.iter().filter(|body| {
        matches!(
            body.name.as_str(),
            "busi" | "yasha" | "roukuno" | "ayame" | "rikimaru" | "muraa" | "byakko" | "yuki"
        )
    }) {
        sandbox.layout.as_mut().unwrap().actors[0].kind = body.kinds[0];
        // Load the selected body even when it isn't normally present on this map.
        sandbox.characters = tenchu_assets::tenchu2::characters::load(
            &disc,
            Some(&[body.kinds[0]].into_iter().collect()),
        )
        .unwrap();
        for joint in [2, 4, 6] {
            let mut game = sandbox.load(&assets, Options::default()).unwrap();
            game.intelligence.ignore_player = true;
            game.roster.keep_all_active = true;
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let id = game
                .roster
                .order
                .iter()
                .copied()
                .find(|&id| id != game.player_id)
                .unwrap();
            let actor = game.roster.get(id);
            if voices::donor_kind(&body.name).is_some() {
                let voice = actor.voice_program.expect("Imported female/hero voice");
                let bank = crate::audio::game_bank(&assets, &game).unwrap();
                let program = bank.programs[voice as usize].as_ref().unwrap();
                assert!(!bank.samples[program.tones[6].sample].frames.is_empty());
                assert_eq!(game.mission.imported_voices.len(), 1);
            }
            let root = actor.root_model;
            let part = actor.parts[joint];
            let position = actor.position;
            let parent = game.scene.models[part].parent;
            let detached_parts: Vec<_> = actor
                .parts
                .iter()
                .copied()
                .filter(|&model| {
                    let mut current = Some(model);
                    while let Some(id) = current {
                        if id == part {
                            return true;
                        }
                        current = game.scene.models[id].parent;
                    }
                    false
                })
                .collect();
            let mut detached = game.scene.world(part).unwrap();
            detached.translation.x += 12000;
            detached.translation.z += 12000;
            game.scene.models[part].parent = Some(0);
            game.scene.update(part, detached);
            game.roster.get_mut(id).severed = Some(tenchu_core::game::dismemberment::Part {
                index: joint,
                parent,
                rotation: [0; 3],
                velocity: [0; 3],
                moving: false,
            });
            let graphics = crate::settings::Graphics {
                smooth_models: true,
                fog: false,
                ..Default::default()
            };
            let mut models = Models::build(&game.mission, &Pack::default(), &graphics).unwrap();
            let mut counts = [0, 0];
            let snapshot = crate::session::tests::state(&game);
            appearance::visit_parts(&game, true, Some(&models), None, |key, geometry, matrix| {
                if let Part::Actor {
                    root: current,
                    model,
                    ..
                } = key
                    && current == root
                {
                    let cut = detached_parts.contains(&model);
                    let origin = if cut { detached.translation } else { position };
                    let origin = Vec3::from(origin.to_array().map(|n| n as f32));
                    assert!(
                        models.visit_part(key, geometry, matrix, false, 0, |triangle| {
                            for vertex in triangle {
                                counts[usize::from(cut)] += 1;
                                assert!(
                                    Vec3::from(vertex.position).distance(origin) < 5000.0,
                                    "{} joint {joint}, part {model}: stretched vertex {:?}",
                                    body.name,
                                    vertex.position
                                );
                            }
                        })
                    );
                }
            });
            assert!(counts.iter().all(|&n| n > 0));
            assert_eq!(snapshot, crate::session::tests::state(&game));
            if let Some(output) = std::env::var_os("TENCHU_T2_LIMB_REVIEW") {
                detached.translation.x -= 9500;
                detached.translation.z -= 12000;
                game.scene.update(part, detached);
                let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
                review.configure(graphics);
                review.renderer.bind_models(&mut models);
                let actors = review
                    .renderer
                    .upload(&crate::render_mesh::characters_presented(
                        &game,
                        Some(&models),
                        true,
                        None,
                    ));
                let center = Vec3::from(position.to_array().map(|n| n as f32))
                    + Vec3::new(1000.0, -700.0, 0.0);
                let eye = center + Vec3::new(2400.0, -1600.0, 3300.0);
                let camera =
                    glam::Mat4::perspective_rh(55f32.to_radians(), 1280.0 / 720.0, 10.0, 60000.0)
                        * glam::Mat4::look_at_rh(eye, center, -Vec3::Y);
                let path =
                    std::path::Path::new(&output).join(format!("{}-joint{joint}.png", body.name));
                review
                    .capture(camera, &[&actors], None, false, &path)
                    .unwrap();
            }
        }
    }
}
