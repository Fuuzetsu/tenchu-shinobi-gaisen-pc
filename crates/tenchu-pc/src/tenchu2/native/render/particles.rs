use super::*;

const TRAIL_IMAGE: u8 = 0;

pub(super) struct Particles<'a> {
    pub scene: &'a tenchu_core::scene::Scene,
    pub view: Matrix,
    pub sprites: &'a tenchu_assets::tenchu2::effects::Sprites,
    pub frame: &'a tenchu_core::tenchu2::effects::Frame,
}

impl Builder {
    pub(super) fn particle_sprite(
        &mut self,
        sprite: &tenchu_core::tenchu2::effects::sprites::Drawing,
        image: Image,
        view: Matrix,
        reproject: bool,
    ) {
        let blend = blend(sprite.blend, sprite.image);
        let projected = self.project(sprite.position);
        if projected[2] <= 0.0 {
            return;
        }
        let adjustment = if reproject {
            let source_depth =
                (view.apply_large(sprite.position).z + view.translation.z).max(0) as f32;
            source_depth / projected[2] * self.distance / 300.0
        } else {
            1.0
        };
        let scale = sprite.scale.map(|v| f32::from(v) / 4096.0 * adjustment);
        self.screen_sprite(
            projected,
            image.size.map(f32::from),
            scale,
            sprite.rotation,
            sprite.color.map(|v| f32::from(v) / 128.0),
            Some(image),
            blend,
        );
    }

    pub(super) fn particles(
        &mut self,
        source: Particles<'_>,
        graphics: &Graphics,
        alpha: f32,
        reproject: bool,
    ) -> Result<()> {
        let Particles {
            scene,
            view,
            sprites,
            frame,
        } = source;
        let mut scene = scene.clone();
        for sprite in &frame.sprites {
            if modern_contact(sprite.contact, &graphics.effects) {
                continue;
            }
            if self.blood_particle(sprite.position, sprite.blood, graphics.blood.hit, alpha) {
                continue;
            }
            let image = sprites.effects[usize::from(sprite.image)];
            self.particle_sprite(sprite, image, view, reproject);
            if sprite.blood.is_some()
                && graphics.blood.hit == crate::settings::BloodStyle::OriginalHd
            {
                self.blood_original_hd();
            }
        }
        for point in &frame.points {
            if modern_contact(point.contact, &graphics.effects) {
                continue;
            }
            if self.blood_particle(point.position, point.blood, graphics.blood.hit, alpha) {
                continue;
            }
            let projected = self.project(point.position);
            if projected[2] <= 0.0 {
                continue;
            }
            self.screen_sprite(
                projected,
                point.size.map(f32::from),
                [1.0; 2],
                0,
                point.color.map(|v| f32::from(v) / 255.0),
                None,
                if point.blend & 4 != 0 {
                    (point.blend & 3) + 1
                } else {
                    0
                },
            );
            if point.blood.is_some()
                && graphics.blood.hit == crate::settings::BloodStyle::OriginalHd
            {
                self.blood_original_hd();
            }
        }
        for ribbon in &frame.ribbons {
            let depth = (ribbon.depth * 16) as f32;
            let points = ribbon.corners.map(|p| {
                if reproject {
                    let inverse = crate::renderer::view_matrix(view).inverse();
                    let world = inverse.transform_point3(glam::Vec3::new(
                        f32::from(p[0]) * depth / 300.0,
                        -f32::from(p[1]) * depth / 300.0,
                        -depth,
                    ));
                    let projected = self.project(world.to_array().map(|v| v as i32).into());
                    [
                        projected[0],
                        projected[1],
                        render_effects::depth(projected[2].max(1.0)),
                    ]
                } else {
                    [
                        f32::from(p[0]),
                        f32::from(p[1]),
                        render_effects::depth(depth),
                    ]
                }
            });
            self.surfaces.push(Surface {
                depth,
                blend: 2,
                vertices: render_effects::quad(
                    points,
                    ribbon.colors.map(|c| c.map(|v| f32::from(v) / 128.0)),
                    Some(sprites.effects[usize::from(TRAIL_IMAGE)]),
                    2,
                ),
            });
        }
        for quad in &frame.quads {
            let death_blood =
                quad.blood == Some(tenchu_core::tenchu2::effects::surface::Blood::Death);
            if death_blood
                && (!graphics.blood.death.uses_original()
                    || graphics.blood.custom_death(false) && quad.blend != Blend::Opaque)
            {
                continue;
            }
            let ground_blood = quad.blood
                == Some(tenchu_core::tenchu2::effects::surface::Blood::Ground)
                && quad.parent.is_none();
            if ground_blood
                && (!graphics.blood.ground.uses_original()
                    || graphics.blood.retained_ground() && quad.blend != Blend::Opaque)
            {
                continue;
            }
            if graphics.effects.splashes
                && quad.parent.is_none()
                && quad.image
                    == SurfaceImage::Effect(tenchu_core::tenchu2::effects::water::RIPPLE_IMAGE)
            {
                continue;
            }
            let (image, blend) = match quad.image {
                SurfaceImage::Effect(id) => {
                    (sprites.effects[usize::from(id)], blend(quad.blend, id))
                }
                SurfaceImage::Item(id) => (sprites.items[usize::from(id)], blend(quad.blend, 0)),
            };
            let mut transform = Matrix {
                translation: quad.position,
                ..Default::default()
            };
            if let Some(parent) = quad.parent {
                transform = scene.world(parent)?.compose(&transform);
            }
            let mut points = quad.corners.map(|v| {
                let v = if death_blood && graphics.blood.death_size != 100 {
                    v.map(|n| (i32::from(n) * i32::from(graphics.blood.death_size) / 100) as i16)
                } else {
                    v
                };
                transform
                    .transform(v.to_array())
                    .map(|v| v as f32)
                    .to_array()
            });
            if quad.image == SurfaceImage::Effect(TRAIL_IMAGE) {
                let projected = points.map(|p| self.project(p.map(|v| v as i32).into()));
                if projected.iter().any(|p| p[2] <= 0.0) {
                    continue;
                }
                // GAME.EXE 80058858 submits the cloth at half the first joint's
                // ordering depth, in front of the body, with the same screen shape.
                let depth = f32::from((projected[0][2].clamp(0.0, 65535.0) as u16) >> 5) * 16.0;
                self.surfaces.push(Surface {
                    depth,
                    blend,
                    vertices: render_effects::quad(
                        projected.map(|p| [p[0], p[1], render_effects::depth(depth)]),
                        [quad.color.map(|v| f32::from(v) / 128.0); 4],
                        Some(image),
                        blend,
                    ),
                });
                continue;
            }
            if ground_blood || death_blood {
                let [a, b, c, _] = points.map(glam::Vec3::from);
                let mut normal = (b - a).cross(c - a).normalize_or_zero();
                let eye = crate::renderer::view_matrix(self.view)
                    .inverse()
                    .transform_point3(glam::Vec3::ZERO);
                if normal.dot(eye - a) < 0.0 {
                    normal = -normal;
                }
                // Source wall quads have no outward winding. Separate double-sided
                // contacts from the receiver on the viewed side, as for floor decals.
                for point in &mut points {
                    *point = (glam::Vec3::from(*point)
                        + normal * crate::visuals::surfaces::DECAL_LIFT)
                        .to_array();
                }
            }
            let mut vertices = render_effects::quad(
                points,
                [quad.color.map(|v| f32::from(v) / 128.0); 4],
                Some(image),
                blend,
            );
            for vertex in &mut vertices {
                vertex.flags &= !render_mesh::SCREEN;
                vertex.flags |= render_mesh::UNLIT;
                if ground_blood || death_blood {
                    vertex.flags |= render_mesh::DOUBLE_SIDED;
                }
            }
            let mut blend = if ground_blood
                && graphics.blood.ground == crate::settings::BloodStyle::OriginalHd
                || death_blood && graphics.blood.death == crate::settings::BloodStyle::OriginalHd
            {
                render_effects::blood::original_hd(&mut vertices, blend)
            } else {
                blend
            };
            if ground_blood || death_blood {
                for vertex in &mut vertices {
                    vertex.flags |= render_mesh::STAIN;
                    if blend == 0 {
                        vertex.flags |= render_mesh::ALPHA;
                    }
                }
                // Contact marks share a plane. Test against the scene without
                // writing depth that would make overlapping stains fight each other.
                if blend == 0 {
                    blend = 5;
                }
            }
            self.surface(vertices, blend);
        }
        Ok(())
    }
}
