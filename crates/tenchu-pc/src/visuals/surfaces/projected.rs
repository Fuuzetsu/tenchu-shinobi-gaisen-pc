use crate::render_mesh::Vertex;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
    sync::Arc,
};

// Bitwise source equality keeps distinct UVs, shading, normals and surface heights separate.
#[derive(Clone, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct Source {
    vertices: [[u32; 16]; 3],
    #[serde(skip)]
    hash: u64,
}

impl<'de> serde::Deserialize<'de> for Source {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_bits(serde::Deserialize::deserialize(
            deserializer,
        )?))
    }
}

impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

impl Eq for Source {}

impl Hash for Source {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Source {
    pub(crate) fn vertices(&self) -> [Vertex; 3] {
        *bytemuck::cast_ref(&self.vertices)
    }

    pub(super) fn new(triangle: &[Vertex; 3]) -> Self {
        Self::from_bits(*bytemuck::cast_ref(triangle))
    }

    fn from_bits(vertices: [[u32; 16]; 3]) -> Self {
        let mut hasher = DefaultHasher::new();
        vertices.hash(&mut hasher);
        Self {
            vertices,
            hash: hasher.finish(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn receiver_identity_and_saved_bytes_survive_hash_caching() {
        let mut vertices = [[0; 16]; 3];
        vertices[1][0] = 1000.0_f32.to_bits();
        vertices[2][1] = (-500.0_f32).to_bits();
        let bytes = postcard::to_allocvec(&vertices).unwrap();
        let source: Source = postcard::from_bytes(&bytes).unwrap();
        let mut unique = HashSet::new();
        unique.insert(source.clone());
        assert_eq!(
            postcard::to_allocvec(unique.get(&source).unwrap()).unwrap(),
            bytes
        );
        let restored: Source = postcard::from_bytes(&bytes).unwrap();
        assert!(!unique.insert(restored));
        for field in [0, 3, 6, 9, 12, 15] {
            vertices[0][field] ^= 1;
            let distinct: Source =
                postcard::from_bytes(&postcard::to_allocvec(&vertices).unwrap()).unwrap();
            assert!(unique.insert(distinct));
        }
    }
}
