use crate::{
    Result,
    math::{Position, Vector3},
    scene::ModelId,
};
use serde::{Deserialize, Serialize};
use tenchu_assets::terrain::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    Texture,
    Opaque,
    Add,
    SubtractMask,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Plane {
    pub extent: [i16; 2],
    pub slope: [i16; 2],
}

impl Plane {
    pub fn flat() -> Self {
        Self {
            extent: [1; 2],
            slope: [0; 2],
        }
    }

    pub fn from_node(node: &Node) -> Self {
        Self {
            extent: [
                node.bounds[2].wrapping_sub(node.bounds[0]),
                node.bounds[3].wrapping_sub(node.bounds[1]),
            ],
            slope: match node.attributes & 0xc000 {
                0x8000 => [node.dy, 0],
                0xc000 => [0, node.dy],
                _ => [0; 2],
            },
        }
    }

    pub fn point(&self, x: i32, z: i32) -> Result<Vector3<i16>> {
        let mut y = 0i16;
        for (axis, coordinate) in [x, z].into_iter().enumerate() {
            let delta = coordinate
                .wrapping_mul(i32::from(self.slope[axis]))
                .checked_div(i32::from(self.extent[axis]))
                .ok_or("Invalid Tenchu 2 effect surface")?;
            y = y.wrapping_add(delta as i16);
        }
        Ok(Vector3::new(x as i16, y, z as i16))
    }

    pub fn corners(&self, radius: i32) -> Result<[Vector3<i16>; 4]> {
        Ok([
            self.point(radius.wrapping_neg(), radius.wrapping_neg())?,
            self.point(radius, radius.wrapping_neg())?,
            self.point(radius.wrapping_neg(), radius)?,
            self.point(radius, radius)?,
        ])
    }
}

#[derive(Clone, Debug)]
pub struct Quad {
    pub image: Image,
    pub position: Position,
    pub parent: Option<ModelId>,
    pub corners: [Vector3<i16>; 4],
    pub color: [u8; 3],
    pub blend: Blend,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    Effect(u8),
    Item(u8),
}

impl Quad {
    pub fn blood(self, fade: i32, drawings: &mut Vec<Self>) {
        if fade >= 128 {
            drawings.push(self);
        } else {
            drawings.push(Self {
                color: [fade as u8; 3],
                blend: Blend::Add,
                ..self.clone()
            });
            drawings.push(Self {
                color: [fade as u8; 3],
                blend: Blend::SubtractMask,
                ..self
            });
        }
    }
}
