use super::{directory, model, upload_textures};
use crate::{ReadAt, Result, model::Part, tenchu2::volume::Volume, texture::Vram};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Background {
    pub sky: Vec<Sky>,
    pub landscape: Option<Part>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Sky {
    pub model: Part,
    pub yaw: i16,
    pub drift: bool,
}

impl Background {
    pub(super) fn load(
        volume: &Volume<'_, impl ReadAt>,
        map: usize,
        sequence: Option<usize>,
        flags: u32,
        vram: &mut Vram,
    ) -> Result<Self> {
        let read = |name: &str| volume.read(&directory(map), name);
        let part = |name: &str| -> Result<Part> {
            Ok(Part {
                parent: -1,
                child: -1,
                position: [0; 3],
                geometry: vec![model::background(&read(name)?)?],
            })
        };
        let mut background = Self::default();
        if flags & 2 != 0 {
            let archive = if matches!(sequence, Some(16 | 17 | 26 | 117 | 186)) {
                "mapBGe.arc"
            } else {
                "mapBG.arc"
            };
            upload_textures(&read(archive)?, vram)?;
            for index in [2, 1] {
                let mut model = part(&format!("mapBG{index}.mdl"))?;
                if index == 1 {
                    model.position[1] = 300;
                }
                let yaw = match (index, map) {
                    (_, 16) => -1408,
                    (1, 17) | (2, 21 | 23) => 1664,
                    _ => 0,
                };
                background.sky.push(Sky {
                    model,
                    yaw,
                    drift: index == 1 && flags & 8 != 0,
                });
            }
        }
        if flags & 4 != 0 {
            background.landscape = Some(part("mapBGW.mdl")?);
        }
        model::resolve_transparency(
            background
                .sky
                .iter_mut()
                .map(|sky| &mut sky.model)
                .chain(background.landscape.iter_mut()),
            vram,
        );
        Ok(background)
    }
}
