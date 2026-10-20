use super::Debug;
use crate::{
    render_effects::{self, Batch, Geometry},
    render_mesh::{DEBUG, DOUBLE_SIDED, SCREEN, Vertex, XRAY},
};
use glam::{Mat4, Vec3, Vec4};
use tenchu_core::{game::Game, ids::ATTR_SUSPEND};

struct Lines {
    vertices: Vec<Vertex>,
    matrix: Mat4,
    aspect: f32,
    xray: bool,
}

fn vertex(position: [f32; 3], color: [f32; 3], flags: u32) -> Vertex {
    Vertex {
        material: 0,
        position,
        color,
        flags,
        uv: [0.0; 2],
        page: 0,
        palette: 0,
    }
}

impl Lines {
    fn line(&mut self, a: Vec3, b: Vec3, color: [f32; 3]) {
        let mut a = self.matrix * a.extend(1.0);
        let mut b = self.matrix * b.extend(1.0);
        for plane in [
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, -1.0, 1.0),
        ] {
            let da = a.dot(plane);
            let db = b.dot(plane);
            if da < 0.0 && db < 0.0 {
                return;
            }
            if da < 0.0 {
                a = a.lerp(b, da / (da - db));
            }
            if db < 0.0 {
                b = b.lerp(a, db / (db - da));
            }
        }
        let to_screen = |p: Vec4| {
            let p = p.truncate() / p.w;
            Vec3::new(
                p.x * self.aspect * 120.0,
                -p.y * 120.0,
                if self.xray {
                    0.0
                } else {
                    (p.z - 0.00001).max(0.0)
                },
            )
        };
        let [a, b] = [a, b].map(to_screen);
        let d = (b - a).truncate().normalize_or_zero() * 0.35;
        let offset = Vec3::new(-d.y, d.x, 0.0);
        let points = [a - offset, b - offset, a + offset, b + offset];
        self.vertices
            .extend([0, 1, 2, 1, 3, 2].map(|i| vertex(points[i].to_array(), color, SCREEN)));
    }

    fn box_at(&mut self, min: Vec3, max: Vec3, color: [f32; 3]) {
        let points: [Vec3; 8] = std::array::from_fn(|i| {
            Vec3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            )
        });
        for i in 0..8 {
            for bit in [1, 2, 4] {
                if i & bit == 0 {
                    self.line(points[i], points[i | bit], color);
                }
            }
        }
    }

    fn label(&self, point: Vec3) -> Option<[i16; 2]> {
        let p = self.matrix * point.extend(1.0);
        let ndc = p.truncate() / p.w;
        (p.w > 0.0 && (0.0..=1.0).contains(&ndc.z) && ndc.x.abs() < 0.9 && ndc.y.abs() < 0.9)
            .then_some([
                (ndc.x * self.aspect * 120.0) as i16,
                (-ndc.y * 120.0) as i16,
            ])
    }
}

pub fn build(debug: &Debug, game: &Game, aspect: f32) -> Geometry {
    let mut lines = Lines {
        vertices: Vec::new(),
        matrix: debug.matrix(game, aspect),
        aspect,
        xray: debug.xray,
    };
    let mut labels = Vec::new();
    if debug.collision || debug.collision_only {
        for surface in &debug.surfaces {
            let flags = DOUBLE_SIDED | DEBUG | if debug.xray { XRAY } else { 0 };
            lines.vertices.extend(
                [0, 1, 2, 1, 3, 2]
                    .map(|i| vertex(surface.corners[i].to_array(), surface.color, flags)),
            );
        }
    }
    if debug.hitboxes {
        for slot in &game.collision.slots[..game.collision.len] {
            let Some(model) = slot.model else {
                continue;
            };
            if game.scene.models[model].attribute & tenchu_core::scene::COLLIDE == 0 {
                continue;
            }
            if slot
                .owner
                .and_then(|id| game.roster.actors.get(id))
                .and_then(Option::as_ref)
                .is_some_and(|a| a.attribute & ATTR_SUSPEND != 0)
            {
                continue;
            }
            let center = Vec3::from(slot.position.map(|v| v as f32));
            let size = Vec3::from(slot.size.map(f32::from));
            let color = if slot.class & tenchu_core::collision::HIT != 0 {
                [1.0, 0.7, 0.15]
            } else {
                [0.25, 0.9, 1.0]
            };
            lines.box_at(center - size, center + size, color);
        }
    }
    for &id in &game.roster.order {
        let actor = game.roster.get(id);
        let position = Vec3::from(actor.position.map(|v| v as f32));
        let suspended = actor.attribute & ATTR_SUSPEND != 0;
        if debug.labels {
            let top = position
                - Vec3::Y * (f32::from(game.mission.characters[actor.definition].height) + 300.0);
            if let Some([x, y]) = lines.label(top) {
                labels.push((
                    x,
                    y,
                    format!("#{id} {:02x} HP {}", actor.kind, actor.health),
                    if suspended {
                        [0.5; 3]
                    } else if id == debug.selected_actor {
                        [1.0, 0.85, 0.2]
                    } else {
                        [0.7, 0.95, 1.0]
                    },
                ));
            }
        }
        if debug.patrols
            && let Some(patrol) = &actor.patrol
        {
            let points: Vec<_> = patrol
                .points
                .iter()
                .filter(|p| p.pad != 0xffff)
                .map(|p| {
                    let mut cursor = tenchu_core::terrain::Cursor::new(&game.mission.terrain);
                    let p = [p.position[0], actor.position[1] - 1500, p.position[1]];
                    let floor =
                        tenchu_core::terrain::level(&game.mission.terrain, &mut cursor, p, 0)
                            .ok()
                            .filter(|&v| v != tenchu_core::terrain::NONE)
                            .unwrap_or(actor.position[1]);
                    Vec3::new(p[0] as f32, floor as f32 - 30.0, p[2] as f32)
                })
                .collect();
            if points.len() > 1 {
                for i in 0..points.len() {
                    lines.line(points[i], points[(i + 1) % points.len()], [0.95, 0.55, 0.9]);
                }
            }
        }
        if debug.sight
            && id != game.player_id
            && !suspended
            && actor.health > 0
            && actor.attribute & tenchu_core::ids::ATTR_CUSTOMAI != 0
        {
            let sneaking = match debug.sight_stance {
                1 => false,
                2 => true,
                _ => matches!(game.player().status, 11 | 12),
            };
            let profile = tenchu_core::actor::sight::profile(sneaking, debug.sight_percent);
            for (range, color) in [
                (profile.clear, [0.35, 1.0, 0.3]),
                (profile.sight, [1.0, 0.75, 0.2]),
            ] {
                for dy in [-2999.0f32, 0.0, 2999.0] {
                    if range as f32 <= dy.abs() {
                        continue;
                    }
                    let radius = ((range as f32).powi(2) - dy * dy).sqrt();
                    let points: Vec<_> = (0..=32)
                        .map(|i| {
                            let angle = (f32::from(actor.rotation[1])
                                - f32::from(profile.half_angle)
                                + 2.0 * f32::from(profile.half_angle) * i as f32 / 32.0)
                                * std::f32::consts::TAU
                                / 4096.0;
                            position + Vec3::new(-angle.sin() * radius, dy, -angle.cos() * radius)
                        })
                        .collect();
                    for pair in points.windows(2) {
                        lines.line(pair[0], pair[1], color);
                    }
                    lines.line(position + Vec3::Y * dy, points[0], color);
                    lines.line(position + Vec3::Y * dy, points[32], color);
                }
            }
        }
    }
    if debug.events {
        for event in &game.mission.scripts.events {
            let bounds = match event.mode {
                1 => Some(event.bounds.map(|[lo, hi]| {
                    [
                        i32::from(lo) * 1000 - if lo <= 0 { 999 } else { 0 },
                        i32::from(hi) * 1000 + if hi >= 0 { 999 } else { 0 },
                    ]
                })),
                6 => {
                    let id = if event.target == 255 {
                        Some(game.player_id)
                    } else {
                        game.roster.find(i16::from(event.target))
                    };
                    id.map(|id| game.roster.get(id).position.map(|v| [v - 2000, v + 2000]))
                }
                _ => None,
            };
            if let Some(b) = bounds {
                let min = Vec3::new(b[0][0] as f32, b[1][0] as f32, b[2][0] as f32);
                let max = Vec3::new(b[0][1] as f32, b[1][1] as f32, b[2][1] as f32);
                lines.box_at(min, max, [0.9, 0.35, 0.95]);
                if let Some([x, y]) = lines.label((min + max) * 0.5) {
                    labels.push((
                        x,
                        y,
                        format!("Event {} CVA {}", event.id, event.sequence),
                        [0.9, 0.35, 0.95],
                    ));
                }
            }
        }
    }
    if debug.show_trail {
        for (a, b) in debug.trail.iter().zip(debug.trail.iter().skip(1)) {
            if a.distance_squared(*b) < 4000.0f32.powi(2) {
                lines.line(*a - Vec3::Y * 20.0, *b - Vec3::Y * 20.0, [1.0, 0.85, 0.2]);
            }
        }
    }
    if debug.inspect_actor
        && let Some(actor) = game
            .roster
            .actors
            .get(debug.selected_actor)
            .and_then(Option::as_ref)
    {
        for (i, text) in [
            format!(
                "#{} kind {:02x} HP {}/{}",
                actor.id, actor.kind, actor.health, actor.max_health
            ),
            format!(
                "XYZ {} {} {}",
                actor.position[0], actor.position[1], actor.position[2]
            ),
            format!(
                "Yaw {} motion {:04x}:{}",
                actor.rotation[1], actor.motion.action, actor.motion.count
            ),
            format!(
                "State {} flags {:04x} AI {:?}",
                actor.status, actor.attribute, actor.think
            ),
        ]
        .into_iter()
        .enumerate()
        {
            labels.push((-154, -94 + i as i16 * 16, text, [0.7, 0.95, 1.0]));
        }
    }
    if debug.camera.is_some() {
        labels.push((
            -154,
            83,
            "FREE CAMERA  WASD/QE  Shift: boost".into(),
            [0.7, 0.95, 1.0],
        ));
        labels.push((
            -154,
            101,
            if debug.teleport_armed {
                "Click collision floor to teleport"
            } else {
                "RMB: look  Click: actor  F2: return"
            }
            .into(),
            [0.7, 0.95, 1.0],
        ));
    }
    let count = lines.vertices.len() as u32;
    let mut geometry = Geometry {
        vertices: lines.vertices,
        batches: vec![Batch {
            blend: 0,
            range: 0..count,
        }],
    };
    geometry.append(render_effects::text(&game.mission.font, &labels));
    geometry
}
