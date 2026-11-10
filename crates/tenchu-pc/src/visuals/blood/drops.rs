use super::*;
use tenchu_assets::texture::Image;

impl Blood {
    pub(super) fn advance_drops(&mut self, tick: u64, images: &[Image], receivers: &Receivers<'_>) {
        let limit = usize::from(self.settings.marks.max(32));
        let amount = f32::from(self.settings.amount.min(200)) / 100.0;
        let mut drops = std::mem::take(&mut self.drops);
        drops.retain_mut(|drop| {
            drop.previous = drop.position;
            drop.velocity.y += 7.0;
            drop.position += drop.velocity;
            drop.life = drop.life.saturating_sub(1);
            if drop.life == 0 {
                return false;
            }
            let hit = self.stain(
                images,
                receivers,
                drop.previous,
                drop.position,
                40.0 + amount * 25.0,
                limit,
            );
            if hit && self.sounds.is_empty() && tick.is_multiple_of(4) {
                self.sounds
                    .push((drop.position.to_array().map(|n| n as i32).into(), 0x37));
            }
            !hit
        });
        self.drops = drops;
    }

    pub(crate) fn append_drops(
        &self,
        camera: Matrix,
        alpha: f32,
        image: Image,
        vertices: &mut Vec<Vertex>,
    ) {
        let camera = rotation(camera).transpose();
        for drop in &self.drops {
            let style = match drop.kind {
                DropKind::Wound => self.settings.drips,
                DropKind::Weapon => BloodStyle::Modern,
                DropKind::Gush => self.settings.gush_style,
                DropKind::Severed => self.settings.severed_style,
            };
            if style == BloodStyle::Off {
                continue;
            }
            let center = drop.previous.lerp(drop.position, alpha.clamp(0.0, 1.0));
            let projected = drop.velocity - camera.z_axis * drop.velocity.dot(camera.z_axis);
            let modern = style == BloodStyle::Modern;
            let scale = if matches!(drop.kind, DropKind::Gush | DropKind::Severed) {
                2.5
            } else {
                1.0
            };
            let along = if modern {
                projected.try_normalize().unwrap_or(camera.y_axis)
            } else {
                camera.y_axis
            };
            let right = along.cross(camera.z_axis) * if modern { 7.0 } else { 11.0 } * scale;
            let up = along
                * if modern {
                    (9.0 + projected.length().min(150.0) * 0.12) * scale
                } else {
                    11.0 * scale
                };
            let points = [
                center - right - up,
                center + right - up,
                center - right + up,
                center + right + up,
            ];
            if style.uses_original() && matches!(drop.kind, DropKind::Gush | DropKind::Severed) {
                let mut sprite = crate::render_effects::quad(
                    points.map(|p| p.to_array()),
                    [[1.0; 3]; 4],
                    Some(image),
                    0,
                );
                for v in &mut sprite {
                    v.flags = render_mesh::TEXTURED
                        | render_mesh::ALPHA
                        | render_mesh::FOG
                        | render_mesh::DOUBLE_SIDED
                        | render_mesh::STAIN
                        | if style == BloodStyle::OriginalHd {
                            render_mesh::BLOOD_HD
                        } else {
                            0
                        };
                    v.opacity = 0.9;
                }
                vertices.extend(sprite);
                continue;
            }
            vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
                position: points[i].to_array(),
                color: if modern {
                    [0.38, 0.008, 0.016]
                } else {
                    [127.0 / 255.0, 16.0 / 255.0, 23.0 / 255.0]
                },
                uv: [
                    if i & 1 == 0 { -1.0 } else { 1.0 },
                    if i & 2 == 0 { -1.0 } else { 1.0 },
                ],
                opacity: 0.9,
                flags: render_mesh::ALPHA
                    | render_mesh::FOG
                    | render_mesh::DOUBLE_SIDED
                    | if modern { render_mesh::SOFT } else { 0 }
                    | if style == BloodStyle::OriginalHd {
                        render_mesh::STAIN | render_mesh::BLOOD_HD
                    } else {
                        0
                    },
                ..Vertex::zeroed()
            }));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn has_drops(&self) -> bool {
        !self.drops.is_empty()
    }
}
