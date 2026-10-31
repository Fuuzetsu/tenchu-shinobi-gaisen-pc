use super::*;
use tenchu_assets::{
    executable::Executable,
    game::GameAssets,
    trial::{Mission, Objective, Tables, Theme},
};
use tenchu_core::{game::Game, random::Random};

#[test]
#[ignore = "requires TENCHU_DISC; audits all TRIAL themes and built-in challenges, optionally exports TENCHU_TRIAL_CATALOG"]
fn trial_asset_inventory() {
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let tables = Tables::load(&assets).unwrap();
    let output = std::env::var_os("TENCHU_TRIAL_CATALOG").map(std::path::PathBuf::from);
    let review = std::env::var_os("TENCHU_TRIAL_REVIEW").map(std::path::PathBuf::from);
    let manifest: super::super::pack::Manifest = serde_json::from_str(
        &fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../artwork/pack.json"))
            .unwrap(),
    )
    .unwrap();
    let mut report = BTreeMap::new();
    for theme_id in 0..12 {
        let theme = Theme::load(&assets, theme_id).unwrap();
        let mut palette = Mission::new(
            theme_id,
            Objective::ReachLord,
            tables.appearance[theme_id][2],
        )
        .unwrap();
        for (index, tile) in theme.tiles.iter().enumerate() {
            let cell = &mut palette.map[(2 + index / 8 * 3) * 32 + 2 + index % 8 * 3];
            if tile.is_ornament() {
                cell.ornament = index as u8;
            } else {
                cell.ground = index as u8;
            }
        }
        let mut missions = vec![(format!("theme-{theme_id:02}"), palette)];
        missions.extend(
            tables
                .presets(&assets)
                .unwrap()
                .into_iter()
                .enumerate()
                .filter(|(_, m)| m.theme == theme_id)
                .map(|(i, m)| (format!("preset-{i:02}"), m)),
        );
        for (name, mission) in missions {
            let mut random = Random::new(1);
            let mut prepared =
                MissionAssets::load_trial(&assets, &tables, &theme, &mission, 0, || {
                    random.next15() as u16
                })
                .unwrap();
            if name.starts_with("theme") {
                prepared.images.extend(&theme.images);
            }
            if let Some(output) = &output {
                export(&prepared, &output.join(&name)).unwrap();
            }
            let regions = catalog::regions(&prepared);
            let missing: BTreeSet<_> = regions
                .values()
                .filter(|id| {
                    !manifest.textures.contains_key(*id) && !manifest.recipes.contains_key(*id)
                })
                .cloned()
                .collect();
            let stage_models: Vec<_> = catalog::models(&prepared)
                .into_iter()
                .filter(|model| model.id.starts_with("stages/"))
                .collect();
            let missing_terrain: BTreeSet<_> =
                catalog::model_regions(&prepared.vram, &stage_models)
                    .into_values()
                    .filter(|id| missing.contains(id))
                    .collect();
            let game = Game::with_random(
                prepared,
                &Executable::new(&assets.executable).unwrap(),
                0,
                random,
            )
            .unwrap();
            let lights = super::super::lights::Emitters::build(&game, &Default::default(), false)
                .sources(&game);
            if name.starts_with("theme") {
                assert!(
                    !lights.is_empty(),
                    "{name}: the theme's lamp must emit light"
                );
            }
            assert!(
                lights.iter().all(|s| !s.name.contains("TOROKKO")),
                "Mine carts are not lanterns"
            );
            if let Some(output) = &review {
                review_scene(&game, &name, output);
            }
            eprintln!(
                "{name}: {} regions, {} missing, {} lights",
                regions.len(),
                missing.len(),
                lights.len()
            );
            assert!(
                missing_terrain.is_empty(),
                "{name}: missing terrain/ornament HD identities: {missing_terrain:?}"
            );
            report.insert(name, serde_json::json!({"missing": missing, "lights": lights.iter().map(|s| &s.name).collect::<Vec<_>>(), "objects": game.mission.object_names, "tiles": theme.tiles.iter().map(|t| serde_json::json!({"name":t.name,"textures":t.textures})).collect::<Vec<_>>() }));
        }
    }
    if let Some(output) = output {
        fs::write(
            output.join("audit.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

fn review_scene(game: &Game, name: &str, output: &Path) {
    use crate::{
        render_mesh,
        settings::{Graphics, presets},
        visuals::{lights::Emitters, pack::Pack},
    };
    use glam::{Mat4, Vec3};
    fs::create_dir_all(output).unwrap();
    let pack = Pack::from_env(&game.mission).unwrap();
    let sources = Emitters::build(game, &pack, true).sources(game);
    let player = Vec3::from(game.player().position.map(|v| v as f32).to_array());
    let focus = sources
        .iter()
        .min_by(|a, b| {
            a.light
                .position
                .distance_squared(player)
                .total_cmp(&b.light.position.distance_squared(player))
        })
        .map_or(player, |source| source.light.position);
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let mut graphics = Graphics::default();
    graphics.set_preset(presets::MODERN_HIGH);
    graphics.asset_pack = true;
    graphics.sky.enabled = false;
    graphics.lighting.directional.intensity = 0;
    review.configure(graphics.clone());
    let models = review.models(&game.mission).unwrap();
    let stage = review.renderer.upload(&render_mesh::stage(
        &game.mission,
        &game.math,
        Some(&models),
    ));
    let actors = review.renderer.upload(&render_mesh::characters_presented(
        game,
        Some(&models),
        true,
        None,
    ));
    let camera = Mat4::perspective_rh(55.0_f32.to_radians(), 1280.0 / 720.0, 50.0, 250000.0)
        * Mat4::look_at_rh(
            focus + Vec3::new(4200.0, -2800.0, -4200.0),
            focus,
            Vec3::NEG_Y,
        );
    for (artwork, lighting) in [(false, false), (true, false), (true, true)] {
        graphics.lighting.enabled = lighting;
        review.lights = sources
            .iter()
            .filter(|_| lighting)
            .map(|s| s.light)
            .collect();
        review.configure(graphics.clone());
        review
            .capture(
                camera,
                &[&stage, &actors],
                None,
                artwork,
                &output.join(format!("{name}-art{artwork}-light{lighting}.png")),
            )
            .unwrap();
    }
}
