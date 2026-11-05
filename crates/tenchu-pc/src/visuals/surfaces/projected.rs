use crate::render_mesh::Vertex;
use std::{ops::Range, sync::Arc};

// Bitwise source equality keeps distinct UVs, shading, normals and surface heights separate.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct Source([[u32; 16]; 3]);

impl Source {
    pub(crate) fn vertices(&self) -> [Vertex; 3] {
        *bytemuck::cast_ref(&self.0)
    }

    pub(super) fn new(triangle: &[Vertex; 3]) -> Self {
        Self(*bytemuck::cast_ref(triangle))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct SourceRange {
    pub vertices: Range<usize>,
    pub source: Arc<Source>,
}

pub(crate) struct Projected {
    pub vertices: Vec<Vertex>,
    pub sources: Vec<SourceRange>,
}
