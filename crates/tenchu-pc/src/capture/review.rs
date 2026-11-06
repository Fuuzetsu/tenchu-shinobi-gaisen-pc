use crate::{
    render_effects,
    render_mesh::Vertex,
    renderer::{EffectMesh, Mesh, Renderer},
    settings::Graphics,
    visuals::{models::Models, pack::Pack},
};
use glam::{Mat4, Vec3};
use std::{fs, path::Path};
use tenchu_assets::{Result, stage::MissionAssets};

pub const SIZE: [u32; 2] = [1280, 720];

pub fn effects(surfaces: &[(u8, Vec<Vertex>)], view: Mat4) -> render_effects::Geometry {
    let mut sorted: Vec<_> = surfaces.iter().collect();
    let depth = |vertices: &[Vertex]| {
        vertices
            .iter()
            .map(|v| view.transform_point3(Vec3::from(v.position)).z)
            .sum::<f32>()
            / vertices.len().max(1) as f32
    };
    sorted.sort_by(|(_, a), (_, b)| depth(a).total_cmp(&depth(b)));
    let mut effects = render_effects::Geometry {
        vertices: Vec::new(),
        batches: Vec::new(),
    };
    for (blend, vertices) in sorted {
        let start = effects.vertices.len() as u32;
        effects.vertices.extend(vertices);
        effects.batches.push(render_effects::Batch {
            blend: *blend,
            range: start..effects.vertices.len() as u32,
        });
    }
    effects
}

pub fn prepare_directory(output: &Path) -> Result<()> {
    if output.exists()
        && fs::read_dir(output)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("Preview output must be an empty directory".into());
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())
}

pub struct Review {
    pub renderer: Renderer,
    pub lights: Vec<crate::lighting::Light>,
    texture: wgpu::Texture,
    graphics: Graphics,
    pack: Pack,
}

impl Review {
    #[cfg(test)]
    pub fn configure(&mut self, graphics: Graphics) {
        self.graphics = graphics;
    }

    pub fn backdrop(&mut self, shade: f32) -> Mesh {
        self.renderer.upload(&crate::render_effects::quad(
            [
                [-1000.0, -120.0, 0.999999],
                [1000.0, -120.0, 0.999999],
                [-1000.0, 120.0, 0.999999],
                [1000.0, 120.0, 0.999999],
            ],
            [[shade; 3]; 4],
            None,
            0,
        ))
    }

    pub fn new(mission: &MissionAssets) -> Result<Self> {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .ok_or("No graphics adapter")?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer =
            pollster::block_on(Renderer::new(&adapter, format, SIZE, &mission.vram))?;
        let pack = Pack::from_env(mission)?;
        renderer.set_asset_pack(&pack);
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Asset review"),
            size: wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let graphics = if std::env::var_os("TENCHU_SETTINGS").is_some() {
            crate::settings::Store::new()?.load()?.graphics
        } else {
            let mut graphics = Graphics {
                fog: false,
                ..Default::default()
            };
            graphics.set_filtering(4);
            graphics
        };
        Ok(Self {
            renderer,
            lights: Vec::new(),
            texture,
            graphics,
            pack,
        })
    }

    pub fn models(&mut self, mission: &MissionAssets) -> Result<Models> {
        let mut graphics = self.graphics.clone();
        graphics.asset_pack = true;
        let mut models = Models::build(mission, &self.pack, &graphics)?;
        self.renderer.bind_models(&mut models);
        Ok(models)
    }

    pub fn capture(
        &mut self,
        camera: Mat4,
        meshes: &[&Mesh],
        effects: Option<&EffectMesh>,
        enabled: bool,
        output: &Path,
    ) -> Result<()> {
        self.graphics.asset_pack = enabled;
        self.renderer.configure(&self.graphics);
        self.renderer.draw_lit(
            &self.texture.create_view(&Default::default()),
            camera,
            crate::renderer::SceneMeshes::new(meshes, &[]),
            effects,
            &self.lights,
            meshes,
        )?;
        super::read_texture(&self.renderer, &self.texture, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; SA execution cameras and head detachment retain all four localized caption/audio schedules"]
    fn sa_execution_preserves_localized_dialogue() {
        use tenchu_assets::{
            executable::Executable,
            game::GameAssets,
            script::{Event, Language},
            stage::MissionAssets,
        };
        use tenchu_core::game::Game;
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        for language in [
            Language::English,
            Language::French,
            Language::Italian,
            Language::Japanese,
        ] {
            let mut reference = None;
            for enabled in [false, true] {
                let mut mission =
                    MissionAssets::load_localized(&assets, 10, 0, 1, language).unwrap();
                let original = mission.scripts.cutscenes.commands.clone();
                mission.scripts.events = vec![Event {
                    id: 0,
                    sequence: 100,
                    next: [255; 2],
                    target: 255,
                    mode: 0,
                    value: 0,
                    bounds: [[0; 2]; 3],
                }];
                let mut game =
                    Game::new(mission, &Executable::new(&assets.executable).unwrap(), 0, 1)
                        .unwrap();
                game.combat.dismemberment.execution_scene = enabled;
                let mut dialogue = Vec::new();
                let mut last = Vec::new();
                let mut music = Vec::new();
                let mut copy: Option<Game> = None;
                let mut began = false;
                for _ in 0..2500 {
                    game.step(0).unwrap();
                    if !began && game.cinematic.active {
                        let kataoka = game.roster.get(game.roster.find(0x8f).unwrap());
                        assert_eq!([kataoka.position.x, kataoka.position.z], [-21000, -3000]);
                        assert_eq!(kataoka.rotation[1], 1022);
                    }
                    if let Some(copy) = &mut copy {
                        copy.step(0).unwrap();
                        assert_eq!(game.cinematic.caption, copy.cinematic.caption);
                        assert_eq!(game.camera.position, copy.camera.position);
                        assert_eq!(game.camera.reference, copy.camera.reference);
                    }
                    if game.cinematic.caption != last {
                        dialogue.push((game.tick, game.cinematic.caption.clone()));
                        last.clone_from(&game.cinematic.caption);
                    }
                    music.extend(game.music.iter().map(|m| (game.tick, *m)));
                    began |= game.cinematic.active;
                    let actor = game.roster.get(game.roster.find(0x8f).unwrap());
                    if actor.severed.is_some() && copy.is_none() {
                        assert!(enabled);
                        copy = Some(
                            postcard::from_bytes(&postcard::to_allocvec(&game).unwrap()).unwrap(),
                        );
                    }
                    if language == Language::English
                        && [1036, 1042, 1052, 1176].contains(&game.cinematic.elapsed)
                        && let Some(path) = std::env::var_os("TENCHU_EXECUTION_REVIEW")
                    {
                        let dir = Path::new(&path);
                        std::fs::create_dir_all(dir).unwrap();
                        crate::capture::frame(
                            &game,
                            &dir.join(format!("sa-{enabled}-frame-{}.png", game.cinematic.elapsed)),
                            &assets,
                        )
                        .unwrap();
                    }
                    if began && !game.cinematic.active {
                        break;
                    }
                }
                assert!(began && !game.cinematic.active);
                assert_eq!(game.mission.scripts.cutscenes.commands, original);
                assert_eq!(
                    game.roster
                        .get(game.roster.find(0x8f).unwrap())
                        .severed
                        .is_some(),
                    enabled
                );
                if enabled {
                    assert!(copy.is_some());
                    assert_eq!(Some((dialogue, music, game.tick)), reference);
                } else {
                    reference = Some((dialogue, music, game.tick));
                }
            }
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC, TENCHU_RETAIL_TAPE, TENCHU_REVIEW_FIELD, TENCHU_REVIEW_IMAGE and a GPU; review one exact replay state without recording a video"]
    fn retail_frame_review() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let tape = std::env::var_os("TENCHU_RETAIL_TAPE").expect("TENCHU_RETAIL_TAPE");
        let field: u32 = std::env::var("TENCHU_REVIEW_FIELD")
            .unwrap()
            .parse()
            .unwrap();
        let output = std::env::var_os("TENCHU_REVIEW_IMAGE").expect("TENCHU_REVIEW_IMAGE");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let tape = crate::retail::tape::Tape::open(Path::new(&tape)).unwrap();
        let mission = tape
            .missions
            .iter()
            .find(|m| field >= m.entry_field && m.result_field.is_none_or(|end| field < end))
            .expect("field inside gameplay/cutscene");
        let mut game = mission.initial.load(&assets).unwrap();
        for update in mission.updates.iter().take_while(|u| u.field <= field) {
            update.apply(&mut game).unwrap();
        }
        let state = crate::session::tests::state(&game);
        crate::capture::frame(&game, Path::new(&output), &assets).unwrap();
        assert_eq!(state, crate::session::tests::state(&game));
    }
}
