use super::{selection, volume::Volume};
use crate::{
    ReadAt, Result, disc::Disc, executable::Executable, frontend::Image, model::arc_entry,
};

pub struct Resources {
    pub images: Vec<Image>,
    pub sprites: Vec<selection::Sprite>,
}

impl Resources {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let bytes = selection::executable(disc)?;
        let exe = Executable::new(&bytes)?;
        let archive = Volume::open(disc)?.read(r"o:\cdimage\menu\image", "title.arc")?;
        let images = (0..4)
            .map(|i| super::texture::image(arc_entry(&archive, i)?))
            .collect::<Result<Vec<_>>>()?;
        let sprites = selection::sprites(&exe, 0x80130e0c, images.len())?;
        if sprites.len() != 6 || images[0].size != [320, 240] || images[1].size != [96, 42] {
            return Err("Invalid Japanese title layout".into());
        }
        Ok(Self { images, sprites })
    }
}
