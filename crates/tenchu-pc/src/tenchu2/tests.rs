use super::*;

mod native_world;
mod source_actor;
mod source_ai;
mod source_ai_entry;
mod source_ai_runtime;
mod source_ai_status;
mod source_alert;
mod source_attack;
mod source_audio;
mod source_awareness;
mod source_bait;
mod source_behavior;
mod source_body;
mod source_caltrops;
mod source_combat;
mod source_control;
mod source_crouch;
mod source_damage;
mod source_darts;
mod source_death;
mod source_dispatch;
mod source_effects;
mod source_entry;
mod source_explosives;
mod source_hurt;
mod source_idle;
mod source_input;
mod source_item;
mod source_item_control;
mod source_item_geometry;
mod source_jump;
mod source_ledge;
mod source_math;
mod source_motion;
mod source_navigation;
mod source_perception;
mod source_physics;
mod source_projectiles;
mod source_ranged;
mod source_rice;
mod source_scene;
mod source_search;
mod source_sequence;
mod source_sight;
mod source_special;
mod source_standing;
mod source_stealth;
mod source_swimming;
mod source_terrain;
mod source_wall;
mod source_weapons;
mod source_world;

#[test]
#[ignore = "requires TENCHU2_DISC; checks every map's source background samples and schedules"]
fn stage_ambience_uses_distinct_map_banks_and_retail_scheduling() {
    use tenchu_assets::tenchu2::{Edition, music};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let edition = Edition::detect(&disc).unwrap();
    let mut signatures = std::collections::BTreeSet::new();
    for map in 0..24 {
        let source = music::ambience(&disc, map).unwrap();
        assert_eq!(
            source.ticks_per_second,
            if edition == Edition::Japan { 30 } else { 25 }
        );
        let program = source.bank.programs[0].as_ref().unwrap();
        for id in [10, 11, 12, 15] {
            let sample = &source.bank.samples[program.tones[id].sample];
            if !sample.frames.is_empty() {
                signatures.insert(
                    sample
                        .frames
                        .iter()
                        .step_by(17)
                        .copied()
                        .collect::<Vec<_>>(),
                );
            }
        }
        if map == 0 {
            assert_eq!(
                source.interval,
                if edition == Edition::Japan { 142 } else { 118 }
            );
        }
        if map == 9 {
            assert_eq!(
                source.interval,
                if edition == Edition::Japan { -1 } else { 0 }
            );
        }
        if matches!(map, 0 | 2 | 9 | 14 | 21 | 23) {
            let interval = source.interval;
            let mut mixer = tenchu_audio::Mixer::new(44100);
            mixer.tenchu2_ambience(source);
            let mut peak = 0.0f32;
            for _ in 0..44100 * 20 {
                for sample in mixer.frame() {
                    peak = peak.max(sample.abs());
                }
            }
            assert_eq!(peak == 0.0, interval == 0, "map {map}: peak {peak}");
            eprintln!("{edition:?} map {map}: interval {interval}, output peak {peak:.3}");
        }
    }
    assert!(
        signatures.len() >= 25,
        "Expected map-specific ambient samples"
    );
    assert!(music::ambience(&disc, 24).is_err());
}

#[test]
#[ignore = "requires Europe TENCHU2_DISC and TENCHU2_ASSET_PACK; checks the published pack's maps/81 models and reconstructed cutouts"]
fn artwork_covers_published_catalog_and_preserves_cutouts() {
    use crate::visuals::{catalog, pack::Pack};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let check =
        |name: &str, parts: &[tenchu_assets::model::Part], vram: &tenchu_assets::texture::Vram| {
            let sources: Vec<_> = parts
                .iter()
                .enumerate()
                .map(|(index, part)| catalog::ModelSource {
                    id: format!("tenchu2/{name}/{index}"),
                    label: name.into(),
                    geometry: &part.geometry,
                })
                .collect();
            let regions = catalog::model_regions(vram, &sources);
            let pack = Pack::for_models(vram, &sources).unwrap();
            for (region, id) in regions {
                let image = pack
                    .textures
                    .get(&id)
                    .unwrap_or_else(|| panic!("{name}: missing {id}"));
                let width = usize::from(region.rect[2]);
                let height = usize::from(region.rect[3]);
                assert_eq!(image.size[0] as usize, width * 4, "{name}: {id}");
                assert_eq!(image.size[1] as usize, height * 4, "{name}: {id}");
                let original = region.original_pixels(vram);
                for y in 0..height * 4 {
                    for x in 0..width * 4 {
                        assert_eq!(
                            image.rgba[(y * width * 4 + x) * 4 + 3],
                            original[((y / 4) * width + x / 4) * 4 + 3],
                            "{name}: {id} cutout at {x},{y}"
                        );
                    }
                }
            }
        };
    for id in Level::available(&disc).unwrap() {
        let level = Level::load(&disc, id).unwrap();
        check(&format!("map{id}/terrain"), &level.map, &level.vram);
        check(&format!("map{id}/props"), &level.objects, &level.vram);
    }
    for body in tenchu_assets::tenchu2::characters::load(&disc, None).unwrap() {
        // These newly supported costumes use original textures until the pack adds them.
        if matches!(body.name.as_str(), "tatua_m" | "tatu2_m") {
            continue;
        }
        check(&format!("actors/{}", body.name), &body.parts, &body.vram);
    }
}

#[test]
#[ignore = "requires both discs and TENCHU2_ASSET_PACK; verifies separate scenery/character artwork and original fallback"]
fn separate_artwork_keeps_source_atlases_and_original_fallback() {
    use crate::visuals::{catalog, models::Models, pack::Pack};
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let path = std::env::var_os("TENCHU2_DISC").unwrap();
    let sandbox = Sandbox::open(path.as_ref(), Some(0), None, Some(0x10)).unwrap();
    let game = sandbox.load(&assets, Options::default()).unwrap();
    let snapshot = postcard::to_allocvec(&game).unwrap();
    let sources = catalog::world_models(&game.mission);
    let pack = Pack::for_models(game.mission.world_vram.as_ref().unwrap(), &sources).unwrap();
    assert!(
        !pack.textures.is_empty(),
        "Tenchu 2 pack is missing or lacks map 0"
    );
    let originals: BTreeMap<_, _> =
        catalog::model_regions(game.mission.world_vram.as_ref().unwrap(), &sources)
            .into_iter()
            .map(|(region, id)| (id, region))
            .collect();
    let mut changed = 0;
    for enabled in [false, true] {
        let graphics = crate::settings::Graphics {
            asset_pack: enabled,
            ..Default::default()
        };
        let mut models = Models::build(&game.mission, &Pack::default(), &graphics).unwrap();
        let mut next = 0;
        models.bind(|material| {
            next += 1;
            if material.region.is_none()
                && let Some(replacement) = pack.textures.get(&material.id)
            {
                if enabled {
                    assert!(
                        material.image.rgba == replacement.rgba,
                        "{}: imported artwork differs",
                        material.id
                    );
                    changed += 1;
                } else {
                    let region = originals[&material.id];
                    assert!(
                        material.image.rgba
                            == region.original_pixels(game.mission.world_vram.as_ref().unwrap()),
                        "{}: imported original differs",
                        material.id
                    );
                }
            }
            next
        });
    }
    assert!(changed > 0);
    assert_eq!(snapshot, postcard::to_allocvec(&game).unwrap());
}

#[test]
#[ignore = "requires both discs; imported patrols, ordinary combat, retry and campaign isolation"]
fn imported_routes_and_guard_combat_work_without_campaign_progress() {
    use crate::campaign::{Action, Campaign, Screen};
    use tenchu_core::{ids::ATTR_WEAPON_DRAWN, input};
    let gaisen = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let t2 = std::env::var_os("TENCHU2_DISC").expect("Set TENCHU2_DISC");
    let assets = GameAssets::open(crate::source::open_disc(gaisen.as_ref()).unwrap()).unwrap();
    assert!(Sandbox::open(t2.as_ref(), Some(4), Some(1), None).is_err());
    assert!(Sandbox::open(t2.as_ref(), Some(7), None, None).is_err());
    for kind in [0x10, 0x11, 0x12] {
        let sandbox = Sandbox::open(t2.as_ref(), None, Some(4), Some(kind)).unwrap();
        assert_eq!(sandbox.level.id, 1);
        let mut game = sandbox.load(&assets, Options::default()).unwrap();
        assert!(!game.mission.enemies.is_empty());
        let choices = crate::visuals::appearance::choices(&game, &Default::default(), None);
        assert!(choices.iter().any(|(id, _)| id == "disc:0"));
        assert!(
            game.mission
                .imported_models
                .keys()
                .all(|kind| { !choices.iter().any(|(id, _)| id == &format!("disc:{kind}")) })
        );
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
        let models = crate::visuals::models::Models::build(
            &game.mission,
            &Default::default(),
            &Default::default(),
        )
        .unwrap();
        let mut rendered = std::collections::BTreeSet::new();
        crate::visuals::appearance::visit_parts(
            &game,
            true,
            Some(&models),
            None,
            |key, geometry, matrix| {
                if let crate::visuals::interpolation::Part::Actor { root, kind, .. } = key
                    && game.mission.imported_models.contains_key(&kind)
                {
                    let actor = game
                        .roster
                        .actors
                        .iter()
                        .flatten()
                        .find(|a| a.root_model == root)
                        .unwrap();
                    let center = glam::Vec3::from(actor.position.to_array().map(|n| n as f32));
                    models.visit_part(key, geometry, matrix, false, 0, |triangle| {
                        assert!(
                            triangle
                                .iter()
                                .all(|v| glam::Vec3::from(v.position).distance(center) < 20000.0)
                        );
                        rendered.insert(root);
                    });
                }
            },
        );
        assert_eq!(rendered.len(), game.mission.enemies.len());
        for character in 0..2 {
            let options = Options {
                character,
                ..Default::default()
            };
            let mut duel = Sandbox::open(t2.as_ref(), Some(0), None, None).unwrap();
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
        let sandbox = Sandbox::open(t2.as_ref(), Some(id), None, None).unwrap();
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

#[test]
#[ignore = "requires TENCHU2_DISC; decodes every source character and costume, textures and idle motion"]
fn all_source_models_have_valid_rigs_and_textures() {
    let path = std::env::var_os("TENCHU2_DISC").unwrap();
    let disc = crate::source::open_disc(path.as_ref()).unwrap();
    let models = tenchu_assets::tenchu2::characters::load(&disc, None).unwrap();
    assert!(models.len() >= 78);
    for body in &models {
        assert_eq!(body.parts.len(), 15, "{}", body.name);
        assert!(
            body.parts
                .iter()
                .flat_map(|p| &p.geometry)
                .any(|g| !g.faces.is_empty())
        );
        for geometry in body.parts.iter().flat_map(|p| &p.geometry) {
            assert_eq!(geometry.joints.len(), geometry.vertices.len());
            assert!(geometry.joints.iter().all(|&j| j < 15));
            assert!(geometry.faces.iter().all(|f| {
                f.corners
                    .iter()
                    .all(|c| usize::from(c.vertex) < geometry.vertices.len())
            }));
        }
        assert!(body.vram.words.iter().any(|&v| v != 0));
        if let Some(idle) = &body.idle {
            assert_eq!(idle.bones.len(), 15);
        }
    }
    for name in ["rikimaru", "ayame", "tatumaru", "busi", "cat"] {
        assert!(
            models
                .iter()
                .find(|m| m.name == name)
                .unwrap()
                .idle
                .is_some(),
            "{name}"
        );
    }
}

#[test]
#[ignore = "requires both discs; animates every imported body on both heroes without changing gameplay, optional TENCHU_T2_MODEL_REVIEW GPU captures"]
fn imported_character_models_animate_with_isolated_textures() {
    use crate::settings::Graphics;
    use crate::visuals::{appearance, appearance::library::Library, models::Models, pack::Pack};
    use glam::{Mat4, Vec3};
    let assets = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let prefix = tenchu_assets::tenchu2::Edition::detect(&disc)
        .unwrap()
        .model_prefix();
    let bodies: Vec<_> = library
        .bodies
        .iter()
        .filter(|b| b.id.starts_with(prefix))
        .collect();
    eprintln!("{} Tenchu 2 models", bodies.len());
    assert!(bodies.len() >= 78);
    let graphics = Graphics {
        fog: false,
        asset_pack: std::env::var_os("TENCHU2_ASSET_PACK").is_some_and(|path| !path.is_empty()),
        smooth_models: true,
        ..Default::default()
    };
    for character in 0..2 {
        let mut game = Options {
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let copy = |game: &Game| {
            postcard::from_bytes::<Game>(&postcard::to_allocvec(game).unwrap()).unwrap()
        };
        let mut poses = vec![copy(&game)];
        for _ in 0..12 {
            game.step(tenchu_core::input::UP).unwrap();
        }
        poses.push(copy(&game));
        for tick in 0..10 {
            game.step(if tick == 0 {
                tenchu_core::input::SQUARE
            } else {
                0
            })
            .unwrap();
        }
        poses.push(game);
        let output = std::env::var_os("TENCHU_T2_MODEL_REVIEW");
        let mut review = output
            .as_ref()
            .map(|_| crate::capture::review::Review::new(&poses[0].mission).unwrap());
        if let Some(review) = &mut review {
            review.configure(graphics.clone());
        }
        for body in &bodies {
            assert!(!body.has_moveset());
            let mut models = Models::default()
                .with_player(
                    &poses[0],
                    &body.id,
                    &Pack::default(),
                    &Pack::default(),
                    &graphics,
                    Some(&library),
                )
                .unwrap();
            if let Some(review) = &mut review {
                review.renderer.bind_models(&mut models);
            }
            let mut previous = Vec::new();
            let mut changed = false;
            for (pose, game) in poses.iter().enumerate() {
                let before = crate::session::tests::state(game);
                let mut vertices = Vec::new();
                appearance::visit_parts(
                    game,
                    true,
                    Some(&models),
                    None,
                    |key, geometry, matrix| {
                        if let crate::visuals::interpolation::Part::Actor { root, .. } = key
                            && root == game.player().root_model
                            && !models.visit_part(key, geometry, matrix, false, 0, |tri| {
                                vertices.extend(tri.into_iter().copied())
                            })
                        {
                            crate::render_mesh::append_geometry(
                                &mut vertices,
                                geometry,
                                matrix,
                                None,
                            );
                        }
                    },
                );
                assert!(!vertices.is_empty(), "{}", body.name);
                let position = Vec3::from(game.player().position.to_array().map(|v| v as f32));
                assert!(
                    vertices.iter().all(|v| Vec3::from(v.position).is_finite()
                        && Vec3::from(v.position).distance(position) < 20000.0),
                    "{}",
                    body.name
                );
                let points: Vec<_> = vertices.iter().map(|v| v.position).collect();
                if !previous.is_empty() {
                    changed |= previous != points;
                }
                previous = points;
                assert_eq!(before, crate::session::tests::state(game));
                if ["rikimaru", "ayame", "tatumaru", "kagami", "cat"]
                    .contains(&body.id.trim_start_matches(prefix))
                    && let (Some(output), Some(review)) = (&output, &mut review)
                {
                    let path = std::path::Path::new(output);
                    std::fs::create_dir_all(path).unwrap();
                    let center = position + Vec3::new(0.0, -900.0, 0.0);
                    let mesh = review.renderer.upload(&vertices);
                    for (suffix, offset) in [
                        ("", Vec3::new(1800.0, -250.0, -3300.0)),
                        ("-front", Vec3::new(3300.0, -250.0, 1800.0)),
                    ] {
                        if !suffix.is_empty() && pose != 0 {
                            continue;
                        }
                        let camera =
                            Mat4::perspective_rh(42f32.to_radians(), 16.0 / 9.0, 10.0, 20000.0)
                                * Mat4::look_at_rh(center + offset, center, Vec3::NEG_Y);
                        review
                            .capture(
                                camera,
                                &[&mesh],
                                None,
                                false,
                                &path.join(format!(
                                    "hero-{character}-{}-pose-{pose}{suffix}.png",
                                    body.id.trim_start_matches(prefix)
                                )),
                            )
                            .unwrap();
                    }
                }
            }
            assert!(changed, "{}", body.name);
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; audits source actor kinds and all 72 gameplay motion bundles"]
fn source_actor_registries_and_full_motion_tracks() {
    use tenchu_assets::tenchu2::{Edition, motions, registry::Registry};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    assert_eq!(registry.actors.len(), 130);
    assert_eq!(
        registry
            .actors
            .iter()
            .map(|a| a.model.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        78
    );
    let sword = registry.actor(20).unwrap();
    let spear = registry.actor(21).unwrap();
    assert_eq!(sword.model, spear.model);
    assert_ne!(sword.weapon, spear.weapon);
    assert_ne!(sword.motions, spear.motions);
    let expected = if registry.edition == Edition::Japan {
        1349
    } else {
        40
    };
    assert_eq!(registry.clip(0, 0xa00), Some(expected));
    let mut clips = BTreeMap::new();
    for hero in 0..3 {
        for map in 0..24 {
            for (id, clip) in motions::for_map(&disc, hero, map).unwrap() {
                if let Some(previous) = clips.insert(id, clip.clone()) {
                    assert_eq!(previous, clip, "{hero}/{map}: inconsistent clip {id}");
                }
                assert!(!clip.root.is_empty());
                assert!(clip.bones.iter().all(|track| !track.is_empty()));
            }
        }
    }
    assert_eq!(
        clips.len(),
        if registry.edition == Edition::Japan {
            1191
        } else {
            1208
        }
    );
    assert_eq!(clips[&1304].forward_speed, 231);
    assert_eq!(clips[&139].forward_speed, -185);
    assert_eq!(clips.values().map(|clip| clip.bones.len()).max(), Some(20));
    let missing: std::collections::BTreeSet<_> = registry
        .actors
        .iter()
        .flat_map(|a| &a.motions)
        .filter_map(|entry| (!clips.contains_key(&entry.clip)).then_some(entry.clip))
        .collect();
    // Japan's campaign CABs omit the horse and hawk/crow moves still listed in DATAMNG.
    let expected: std::collections::BTreeSet<_> = if registry.edition == Edition::Japan {
        [776, 777, 778].into_iter().chain(1190..=1205).collect()
    } else {
        Default::default()
    };
    assert_eq!(missing, expected);
    eprintln!(
        "{:?}: {} kinds, {} source clips; missing from campaign bundles: {missing:?}",
        registry.edition,
        registry.actors.len(),
        clips.len()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_MOTION_ORACLE from the selected edition's MIPS routine"]
fn motion_registration_matches_source_selection() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = tenchu_assets::tenchu2::registry::Registry::load(&disc).unwrap();
    let data = std::fs::read_to_string(std::env::var_os("TENCHU2_MOTION_ORACLE").unwrap()).unwrap();
    let mut checked = 0;
    for line in data.lines() {
        let fields: Vec<i16> = line
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(fields.len(), 3);
        let [kind, action, expected] = fields[..] else {
            unreachable!()
        };
        assert_eq!(
            registry.clip(kind, action).unwrap_or(-1),
            expected,
            "kind {kind}, action {action:#x}"
        );
        checked += 1;
    }
    assert!(checked >= 3000);
    eprintln!(
        "{:?}: {checked} MIPS motion selections match",
        registry.edition
    );
}

mod source_objects;

mod source_camera;
mod source_visibility;

mod source_scenery;

mod source_markers;
mod source_special_items;
mod source_teleport;
mod source_textures;
