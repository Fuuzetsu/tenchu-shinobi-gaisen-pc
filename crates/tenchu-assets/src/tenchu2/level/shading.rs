use crate::{ReadAt, Result, model::Part, tenchu2::volume::Volume};

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Shading(#[serde(with = "serde_big_array::BigArray")] [[u8; 3]; 256]);

impl Default for Shading {
    fn default() -> Self {
        Self(std::array::from_fn(|i| [i as u8; 3]))
    }
}

impl Shading {
    pub fn color(&self, index: u8) -> [u8; 3] {
        self.0[usize::from(index)]
    }

    pub(super) fn load(volume: &Volume<'_, impl ReadAt>, directory: &str) -> Result<Self> {
        let Some(data) = volume.optional(directory, "mapfog.tim")? else {
            return Ok(Self::default());
        };
        let colors = crate::slice(&data, 20, 256 * 3)?;
        Ok(Self(std::array::from_fn(|i| {
            colors[i * 3..i * 3 + 3].try_into().unwrap()
        })))
    }

    /// Resolve the shade indices of freshly decoded MDLs before mesh refinement.
    pub fn apply<'a>(&self, parts: impl Iterator<Item = &'a mut Part>) {
        for corner in parts
            .flat_map(|part| &mut part.geometry)
            .flat_map(|geometry| &mut geometry.faces)
            .flat_map(|face| &mut face.corners)
        {
            corner.color = self.0[usize::from(corner.color[0])];
        }
    }
}
