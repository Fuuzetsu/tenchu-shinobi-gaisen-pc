use crate::{renderer::Renderer, visuals::models::Meshes};
use tenchu_assets::texture::{Image, TextureScroll as Animation};
use tenchu_core::effects::TextureScroll;

pub(super) struct Scrolling(Vec<(usize, u32)>);

impl Scrolling {
    pub fn bind(meshes: &mut Meshes, scrolls: &[TextureScroll], renderer: &mut Renderer) -> Self {
        let indices: Vec<_> = meshes
            .materials_mut()
            .iter_mut()
            .map(|material| {
                let region = material.region?;
                let image = Image {
                    page: region.page,
                    palette: region.palette,
                    uv: [region.rect[0] as u8, region.rect[1] as u8],
                    size: [region.rect[2], region.rect[3]],
                };
                let index = scrolls
                    .iter()
                    .position(|s| s.destination == image.word_rect())?;
                // The source scalar accumulator supplies the phase. Image coordinates
                // distinguish identical artwork scrolling independently in VRAM.
                material.animation = Some(Animation {
                    image,
                    velocity: [0; 2],
                    delay: 0,
                    water: false,
                });
                Some(index)
            })
            .collect();
        meshes.detach_textures();
        let mut indices = indices.into_iter();
        let mut bindings = Vec::new();
        meshes.bind(|material| {
            let handle = renderer.bind_material(material);
            if let Some(index) = indices.next().flatten() {
                bindings.push((index, handle));
            }
            handle
        });
        let scrolling = Self(bindings);
        scrolling.update(scrolls, renderer);
        scrolling
    }

    pub fn update(&self, scrolls: &[TextureScroll], renderer: &mut Renderer) {
        for &(index, handle) in &self.0 {
            let scroll = &scrolls[index];
            renderer.set_material_phase(
                handle,
                std::array::from_fn(|axis| {
                    f32::from(scroll.offset[axis] / 16) / f32::from(scroll.destination[axis + 2])
                }),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        render_mesh::{self, Vertex},
        settings::Graphics,
    };
    use tenchu_assets::{game::GameAssets, stage::MissionAssets};
    use tenchu_core::{math::Matrix, tenchu2::world::Options};

    #[test]
    #[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; native imported materials match source VRAM copies, including slow and negative scrolls"]
    fn gaisen_scrolling_materials_match_original_texture_copies() {
        let game = GameAssets::open(
            crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
        )
        .unwrap();
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        for stage in [0, 7, 9, 10] {
            let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
            let mut session = super::super::super::Session::gaisen(
                &disc,
                &source,
                Options {
                    hero: 0,
                    difficulty: 2,
                    seed: 1,
                    inventory: Default::default(),
                },
                false,
            )
            .unwrap();
            let mut images = Vec::new();
            for mode in ["initial", "animated", "source-copy"] {
                if mode == "animated" {
                    for scroll in &mut session.world.environment.texture_scrolls {
                        for _ in 0..17 {
                            scroll.step();
                        }
                    }
                } else if mode == "source-copy" {
                    for scroll in &mut session.world.environment.texture_scrolls {
                        session
                            .assets
                            .level
                            .vram
                            .copy_rect(
                                [
                                    scroll.source[0].wrapping_add((scroll.offset[0] / 16) as u16),
                                    scroll.source[1].wrapping_add((scroll.offset[1] / 16) as u16),
                                    scroll.destination[2],
                                    scroll.destination[3],
                                ],
                                [scroll.destination[0], scroll.destination[1]],
                            )
                            .unwrap();
                        scroll.offset = [0; 2];
                    }
                }
                let snapshot = postcard::to_allocvec(&session.world).unwrap();
                let path = output.join(format!("scroll-{stage}-{mode}.png"));
                super::super::capture_frame(
                    &session.assets.sprites.vram,
                    &graphics,
                    &path,
                    |renderer| {
                        let models = super::super::Models::new(
                            &session.assets,
                            &session.world,
                            &graphics,
                            renderer,
                        )?;
                        let mut vertices = Vec::new();
                        let mut seen = std::collections::BTreeSet::new();
                        for &(index, handle) in &models.scrolling.0 {
                            if !seen.insert(index) {
                                continue;
                            }
                            let x = -140.0 + index as f32 * 95.0;
                            for corner in [0, 1, 2, 2, 1, 3] {
                                let u = (corner & 1) as f32;
                                let v = (corner >> 1) as f32;
                                vertices.push(Vertex {
                                    // A power-of-two pixel span avoids nearest-sampling
                                    // ties when comparing shifted UVs to copied texels.
                                    position: [
                                        x + u * (256.0 / 3.0),
                                        -45.0 + v * (256.0 / 3.0),
                                        0.01,
                                    ],
                                    color: [0.5; 3],
                                    uv: [u - 0.5, v - 0.5],
                                    page: 256,
                                    palette: 0,
                                    material: handle,
                                    normal: [0.0, 0.0, -1.0],
                                    opacity: 1.0,
                                    flags: render_mesh::TEXTURED
                                        | render_mesh::SCREEN
                                        | render_mesh::UI
                                        | render_mesh::DOUBLE_SIDED,
                                });
                            }
                        }
                        assert_eq!(
                            seen.len(),
                            session.world.environment.texture_scrolls.len(),
                            "stage {stage}: source scroll has no native material"
                        );
                        let geometry = super::super::Geometry {
                            lamps: Vec::new(),
                            backgrounds: Vec::new(),
                            terrain: vertices,
                            props: Vec::new(),
                            actors: Vec::new(),
                            effects: crate::render_effects::Geometry {
                                vertices: Vec::new(),
                                batches: Vec::new(),
                            },
                        };
                        Ok((
                            super::super::SceneDrawing::new(
                                geometry,
                                renderer,
                                &session.assets.level,
                            ),
                            Matrix::default(),
                            Vec::new(),
                        ))
                    },
                )
                .unwrap();
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), snapshot);
                images.push(
                    crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                        .unwrap()
                        .rgba,
                );
            }
            assert!(
                images[0] != images[1],
                "stage {stage}: imported textures did not scroll"
            );
            assert!(
                images[1] == images[2],
                "stage {stage}: native phase differs from original VRAM copies"
            );
        }
    }
}
