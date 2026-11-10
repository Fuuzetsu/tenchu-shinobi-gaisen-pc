use super::*;

impl Builder {
    pub(super) fn particles(
        &mut self,
        world: &World,
        assets: &Assets,
        frame: &tenchu_core::tenchu2::effects::Frame,
        graphics: &Graphics,
        alpha: f32,
        reproject: bool,
    ) -> Result<()> {
        let mut scene = world.scene.clone();
        for sprite in &frame.sprites {
            if modern_contact(sprite.contact, &graphics.effects) {
                continue;
            }
            if self.blood_particle(sprite.position, sprite.blood, graphics.blood.hit, alpha) {
                continue;
            }
            let image = assets.sprites.effects[usize::from(sprite.image)];
            let blend = blend(sprite.blend, sprite.image);
            let projected = self.project(sprite.position);
            if projected[2] <= 0.0 {
                continue;
            }
            let adjustment = if reproject {
                let source_depth = (world.camera.view.apply_large(sprite.position).z
                    + world.camera.view.translation.z)
                    .max(0) as f32;
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
                    let inverse = crate::renderer::view_matrix(world.camera.view).inverse();
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
                    ribbon.colors.map(|c| c.map(|v| f32::from(v) / 255.0)),
                    None,
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
                SurfaceImage::Effect(id) => (
                    assets.sprites.effects[usize::from(id)],
                    blend(quad.blend, id),
                ),
                SurfaceImage::Item(id) => {
                    (assets.sprites.items[usize::from(id)], blend(quad.blend, 0))
                }
            };
            let mut transform = Matrix {
                translation: quad.position,
                ..Default::default()
            };
            if let Some(parent) = quad.parent {
                transform = scene.world(parent)?.compose(&transform);
            }
            let points = quad.corners.map(|v| {
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
            let blend = if ground_blood
                && graphics.blood.ground == crate::settings::BloodStyle::OriginalHd
                || death_blood && graphics.blood.death == crate::settings::BloodStyle::OriginalHd
            {
                render_effects::blood::original_hd(&mut vertices, blend)
            } else {
                blend
            };
            self.surface(vertices, blend);
        }
        Ok(())
    }
}
