use crate::visuals::appearance::{self, Appearance};
#[cfg(not(target_arch = "wasm32"))]
use crate::{renderer::Renderer, settings::Graphics, visuals::pack::Pack};
#[cfg(not(target_arch = "wasm32"))]
use tenchu_assets::{Result, texture::Vram};
use tenchu_assets::{model::Part, tenchu2::BONES};
use tenchu_core::math::Matrix;

pub(super) struct Player {
    pub appearance: Appearance,
    source: Vec<Part>,
}

impl Player {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(
        id: &str,
        pack: &Pack,
        parts: &[Part],
        vram: &Vram,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let appearance = Appearance::custom(id, pack, vram, graphics)?;
        Self::from_appearance(appearance, parts, renderer)
    }

    pub fn from_appearance(
        mut appearance: Appearance,
        parts: &[Part],
        renderer: &mut crate::renderer::Renderer,
    ) -> tenchu_assets::Result<Self> {
        if parts.len() < BONES.len() {
            return Err("The player has no compatible humanoid rig".into());
        }
        let parents = appearance::parents(parts);
        let source = BONES
            .iter()
            .enumerate()
            .map(|(i, &bone)| Part {
                child: i as i16,
                parent: parents[bone]
                    .and_then(|p| BONES.iter().position(|&b| b == p))
                    .map_or(-1, |p| p as i16),
                position: parts[bone].position,
                geometry: Vec::new(),
            })
            .collect();
        renderer.bind_meshes(&mut appearance.meshes);
        Ok(Self { appearance, source })
    }

    pub fn pose(&self, native: &[Matrix], root_local: Matrix) -> Vec<Matrix> {
        self.pose_detached(native, root_local, None)
    }

    pub fn pose_detached(
        &self,
        native: &[Matrix],
        root_local: Matrix,
        detached: Option<usize>,
    ) -> Vec<Matrix> {
        let matrices = BONES.map(|b| native[b]);
        self.appearance.retarget(
            &self.source,
            &matrices,
            root_local,
            detached.and_then(|b| BONES.iter().position(|&n| n == b)),
        )
    }

    pub fn attach(matrix: Matrix, bone: usize, native: &[Matrix], target: &[Matrix]) -> Matrix {
        BONES.iter().position(|&b| b == bone).map_or(matrix, |i| {
            appearance::relocate(matrix, native[bone], target[i])
        })
    }
}
