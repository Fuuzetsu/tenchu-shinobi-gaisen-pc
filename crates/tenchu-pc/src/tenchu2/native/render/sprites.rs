use super::*;

pub(super) struct Sprites(BTreeMap<Image, Binding>);

struct Binding {
    region: catalog::Region,
    material: crate::visuals::models::Material,
    original: u32,
    hd: Option<u32>,
}

impl Sprites {
    pub fn new(vram: &Vram, images: impl Iterator<Item = Image>, renderer: &mut Renderer) -> Self {
        Self(
            images
                .map(|image| {
                    let region = catalog::Region::sprite(vram, image);
                    let pixels = region.pixels(vram);
                    let size = [region.rect[2], region.rect[3]];
                    let material = crate::visuals::models::Material {
                        id: catalog::image_id(size, &pixels),
                        region: None,
                        image: std::sync::Arc::new(crate::visuals::pixels::Image {
                            size: size.map(u32::from),
                            rgba: pixels,
                        }),
                        animation: None,
                    };
                    let original = renderer.bind_material(&material);
                    (
                        image,
                        Binding {
                            region,
                            material,
                            original,
                            hd: None,
                        },
                    )
                })
                .collect(),
        )
    }

    pub fn retain(&mut self, surfaces: &mut [Surface], renderer: &mut Renderer) -> Result<()> {
        for surface in surfaces {
            let Some(first) = surface.vertices.first() else {
                continue;
            };
            if first.flags & render_mesh::TEXTURED == 0 || first.material != 0 {
                continue;
            }
            let low: [u16; 2] = std::array::from_fn(|i| {
                surface
                    .vertices
                    .iter()
                    .map(|v| v.uv[i] as u16)
                    .min()
                    .unwrap()
            });
            let high: [u16; 2] = std::array::from_fn(|i| {
                surface
                    .vertices
                    .iter()
                    .map(|v| v.uv[i] as u16)
                    .max()
                    .unwrap()
            });
            let image = Image {
                page: first.page as u16,
                palette: first.palette as u16,
                uv: low.map(|v| v as u8),
                size: std::array::from_fn(|i| high[i] - low[i] + 1),
            };
            let hd = render_mesh::original_hd_blood(first.flags);
            let binding = self
                .0
                .get_mut(&image)
                .ok_or("Unbound native cinematic sprite")?;
            let region = binding.region;
            let handle = if hd {
                *binding
                    .hd
                    .get_or_insert_with(|| renderer.bind_original_hd(&binding.material))
            } else {
                binding.original
            };
            for vertex in &mut surface.vertices {
                vertex.material = handle;
                // Source quads address texel edges, retaining the shader's sample bias.
                vertex.uv = std::array::from_fn(|i| {
                    (vertex.uv[i] + if hd { 0.0 } else { 0.001 } - f32::from(region.rect[i]))
                        / f32::from(region.rect[i + 2])
                        - if hd { 0.0 } else { 0.5 }
                });
            }
        }
        Ok(())
    }
}
