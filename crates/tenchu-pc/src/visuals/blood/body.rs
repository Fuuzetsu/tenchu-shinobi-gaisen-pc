use super::*;

pub(crate) struct BodyPart {
    pub key: Part,
    pub matrix: Matrix,
    pub vertices: Vec<Vertex>,
}

pub(crate) struct Attachment<'a> {
    pub matrix: Matrix,
    pub deformation: Option<super::super::motion::Deformation<'a>>,
}

impl Blood {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn clear_model_stains(&mut self) {
        self.marks.retain_attached(|mark| mark.1 != Kind::Body);
    }

    pub(crate) fn stain_body(
        &mut self,
        parts: &[BodyPart],
        position: Vec3,
        approach: Option<Vec3>,
        settings: &settings::Blood,
    ) -> Option<Wound> {
        let contact = approach.and_then(|direction| {
            parts
                .iter()
                .enumerate()
                .flat_map(|(index, BodyPart { vertices, .. })| {
                    vertices.chunks_exact(3).filter_map(move |t| {
                        surfaces::intersect(
                            &[t[0], t[1], t[2]],
                            position + direction * 2000.0,
                            position - direction * 600.0,
                        )
                        .map(|hit| (index, hit))
                    })
                })
                .min_by(|a, b| a.1.fraction.total_cmp(&b.1.fraction))
                .map(|(index, hit)| (index, hit.position, hit.normal))
        });
        let nearest = contact.or_else(|| {
            parts
                .iter()
                .enumerate()
                .flat_map(|(i, BodyPart { vertices, .. })| {
                    vertices.chunks_exact(3).map(move |t| {
                        let triangle = [t[0], t[1], t[2]];
                        let point = surfaces::closest(&triangle, position);
                        let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
                        (i, point, (b - a).cross(c - a).normalize_or_zero())
                    })
                })
                .min_by(|a, b| {
                    a.1.distance_squared(position)
                        .total_cmp(&b.1.distance_squared(position))
                })
        });
        let (index, center, normal) = nearest?;
        if center.distance_squared(position) > 1200.0_f32.powi(2) {
            return None;
        }
        self.random();
        let stamp = Stamp {
            center,
            normal,
            radius: 120.0 + f32::from(settings.amount.min(200)) * 0.8,
            depth: 32.0,
            seed: self.seed,
            shape: None,
        };
        let projection = stamp.prepare();
        let mut patches = Vec::new();
        for &BodyPart {
            key,
            matrix,
            ref vertices,
        } in parts
        {
            let r = rotation(matrix);
            if r.determinant().abs() < 0.0001 {
                continue;
            }
            let inverse = r.inverse();
            let offset = translation(matrix);
            let mut projected = Vec::new();
            for triangle in vertices.chunks_exact(3) {
                projection.project(&[triangle[0], triangle[1], triangle[2]], &mut projected);
            }
            for v in &mut projected {
                v.position = (inverse * (Vec3::from(v.position) - offset)).to_array();
                v.normal = (r.transpose() * Vec3::from(v.normal)).to_array();
            }
            if !projected.is_empty() {
                patches.push(Patch {
                    part: Some(key),
                    vertices: projected,
                    sources: Vec::new(),
                    bounds: Default::default(),
                });
            }
        }
        if settings.model_stains {
            self.retain(
                Mark(patches, Kind::Body),
                usize::from(settings.marks.max(32)),
            );
        }
        let BodyPart { key, matrix, .. } = &parts[index];
        let r = rotation(*matrix);
        if r.determinant().abs() < 0.0001 {
            return None;
        }
        Some(Wound {
            part: *key,
            position: r.inverse() * (center - translation(*matrix)),
            normal: r.transpose() * normal,
            age: 0,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn attached_parts(&self) -> impl Iterator<Item = Part> + '_ {
        self.marks
            .attached
            .iter()
            .flat_map(|(_, mark)| mark.0.iter().filter_map(|p| p.part))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn retain_attached_parts(&mut self, mut exists: impl FnMut(Part) -> bool) {
        self.marks
            .retain_attached(|mark| mark.0.iter().all(|p| p.part.is_none_or(&mut exists)));
    }

    pub(crate) fn append_attached<'a>(
        &self,
        view: &View,
        mut transform: impl FnMut(Part) -> Option<Attachment<'a>>,
        vertices: &mut Vec<Vertex>,
    ) {
        let frustum = view.projection.map(crate::renderer::Frustum::new);
        for (_, mark) in &self.marks.attached {
            for patch in &mark.0 {
                let attachment = if let Some(key) = patch.part {
                    let Some(attachment) = transform(key) else {
                        continue;
                    };
                    attachment
                } else {
                    Attachment {
                        matrix: Matrix::default(),
                        deformation: None,
                    }
                };
                let rotation = rotation(attachment.matrix);
                let offset = translation(attachment.matrix);
                if rotation.determinant().abs() < 0.0001 {
                    continue;
                }
                let deformation = attachment.deformation;
                if frustum.as_ref().is_some_and(|frustum| {
                    !patch.visible(frustum, rotation, offset, deformation.as_ref(), view.wind)
                }) {
                    continue;
                }
                let normal = rotation.inverse().transpose();
                vertices.extend(patch.vertices.iter().map(|v| {
                    let mut vertex = Vertex {
                        position: (rotation * Vec3::from(v.position) + offset).to_array(),
                        normal: (normal * Vec3::from(v.normal)).to_array(),
                        ..*v
                    };
                    if let Some(deformation) = &deformation {
                        deformation.vertex(&mut vertex);
                    }
                    vertex
                }));
            }
        }
    }
}
