use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use tenchu_assets::{
    model::{Geometry, Texture},
    stage::MissionAssets,
    texture::Vram,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Region {
    pub page: u16,
    pub palette: u16,
    pub rect: [u16; 4],
}

impl Region {
    pub fn find(vram: &Vram, texture: Texture, bounds: [u16; 4]) -> Self {
        let page = texture.page & 0x19f;
        let palette = if page & 0x180 == 0x100 {
            0
        } else {
            texture.palette
        };
        let scale = 4 >> ((page >> 7) & 3);
        let origin = [i32::from(page & 15) * 64 * scale, i32::from(page & 16) * 16];
        let rect = vram
            .images
            .iter()
            .rev()
            .filter_map(|image| {
                if image.page & 0x180 != page & 0x180
                    || (page & 0x180 != 0x100 && image.palette != palette)
                {
                    return None;
                }
                let x =
                    i32::from(image.page & 15) * 64 * scale + i32::from(image.uv[0]) - origin[0];
                let y = i32::from(image.page & 16) * 16 + i32::from(image.uv[1]) - origin[1];
                let end = [x + i32::from(image.size[0]), y + i32::from(image.size[1])];
                let start = [x.max(0), y.max(0)];
                let end = end.map(|n| n.min(256));
                if start[0] > i32::from(bounds[0])
                    || start[1] > i32::from(bounds[1])
                    || end[0] < i32::from(bounds[2])
                    || end[1] < i32::from(bounds[3])
                    || end[0] <= i32::from(bounds[0])
                    || end[1] <= i32::from(bounds[1])
                {
                    return None;
                }
                Some([
                    start[0] as u16,
                    start[1] as u16,
                    (end[0] - start[0]) as u16,
                    (end[1] - start[1]) as u16,
                ])
            })
            .min_by_key(|rect| u32::from(rect[2]) * u32::from(rect[3]))
            .unwrap_or([
                bounds[0],
                bounds[1],
                bounds[2] - bounds[0] + 1,
                bounds[3] - bounds[1] + 1,
            ]);
        Self {
            page,
            palette,
            rect,
        }
    }

    pub fn pixels(self, vram: &Vram) -> Vec<u8> {
        let [x, y, width, height] = self.rect;
        let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
        for v in y..y + height {
            for u in x..x + width {
                let word = vram.texel(self.page, self.palette, [u, v]);
                pixels.extend(
                    [0, 5, 10].map(|shift| (((word >> shift) & 31) as u32 * 255 / 31) as u8),
                );
                pixels.push(if word == 0 {
                    0
                } else if word & 0x8000 != 0 {
                    128
                } else {
                    255
                });
            }
        }
        pixels
    }
}

pub fn face_region(vram: &Vram, face: &tenchu_assets::model::Face) -> Option<Region> {
    let texture = face.texture?;
    let mut bounds = [255, 255, 0, 0];
    for corner in &face.corners {
        for axis in 0..2 {
            bounds[axis] = bounds[axis].min(u16::from(corner.uv[axis]));
            bounds[axis + 2] = bounds[axis + 2].max(u16::from(corner.uv[axis]));
        }
    }
    Some(Region::find(vram, texture, bounds))
}

pub fn image_id(size: [u16; 2], pixels: &[u8]) -> String {
    let mut hash = Sha256::new();
    for value in size {
        hash.update(value.to_le_bytes());
    }
    hash.update(pixels);
    format!("{:x}", hash.finalize())[..32].to_owned()
}

pub struct ModelSource<'a> {
    pub id: String,
    pub label: String,
    pub geometry: &'a [Geometry],
}

pub fn models(mission: &MissionAssets) -> Vec<ModelSource<'_>> {
    let mut out = Vec::new();
    for (index, part) in mission.map.iter().enumerate() {
        out.push(ModelSource {
            id: format!("stages/{}/map/{index}", mission.stage.directory),
            label: format!("Map part {index}"),
            geometry: &part.geometry,
        });
    }
    for (index, part) in mission.objects.iter().enumerate() {
        out.push(ModelSource {
            id: format!("stages/{}/objects/{index}", mission.stage.directory),
            label: mission
                .object_names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("Object {index}")),
            geometry: &part.geometry,
        });
    }
    for (&kind, parts) in &mission.models {
        for (index, part) in parts.iter().enumerate() {
            out.push(ModelSource {
                id: format!("actors/{kind:04x}/{index}"),
                label: format!(
                    "{}, part {index}",
                    mission
                        .characters
                        .iter()
                        .find(|c| c.kind == kind)
                        .map_or("Actor", |c| c.model.as_str())
                ),
                geometry: &part.geometry,
            });
        }
    }
    for (&kind, geometry) in &mission.weapons.models {
        out.push(ModelSource {
            id: format!("weapons/{kind:04x}"),
            label: mission.weapons.names[&kind].clone(),
            geometry,
        });
    }
    for (&kind, geometry) in &mission.common_models {
        out.push(ModelSource {
            id: format!("common/{kind}"),
            label: format!("Common model {kind}"),
            geometry,
        });
    }
    out
}

pub fn regions(mission: &MissionAssets) -> BTreeMap<Region, String> {
    let mut out = BTreeMap::new();
    for model in models(mission) {
        for geometry in model.geometry {
            for face in crate::render_mesh::draw_tmd_faces(geometry) {
                if let Some(region) = face_region(&mission.vram, face) {
                    out.entry(region).or_insert_with(|| {
                        image_id(
                            [region.rect[2], region.rect[3]],
                            &region.pixels(&mission.vram),
                        )
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::texture::Image;

    #[test]
    fn regions_cross_page_origins_and_keep_palette_and_transparency_classes() {
        let mut vram = Vram::default();
        vram.images.push(Image {
            page: 0x86,
            palette: 64,
            uv: [0, 16],
            size: [32, 16],
        });
        let texture = Texture {
            page: 0xe5,
            palette: 64,
        };
        let region = Region::find(&vram, texture, [128, 16, 160, 32]);
        assert_eq!(region.rect, [128, 16, 32, 16]);
        assert_eq!(region.page, 0x85);
        vram.words[1024] = 0;
        vram.words[1025] = 31;
        vram.words[1026] = 0x8000;
        vram.words[1027] = 0x83e0;
        vram.words[16 * 1024 + 384] = 0x0100;
        vram.words[16 * 1024 + 385] = 0x0302;
        assert_eq!(
            &region.pixels(&vram)[..16],
            &[0, 0, 0, 0, 255, 0, 0, 255, 0, 0, 0, 128, 0, 255, 0, 128,]
        );
        assert_eq!(
            Region::find(
                &vram,
                Texture {
                    palette: 65,
                    ..texture
                },
                [128, 16, 143, 23]
            )
            .rect,
            [128, 16, 16, 8]
        );
    }
}
