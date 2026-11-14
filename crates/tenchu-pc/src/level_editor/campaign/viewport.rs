use super::*;
use egui::{Color32, Pos2, Sense, Stroke, Vec2};
use glam::Vec3;

pub(super) fn corners(node: Node, t2: bool) -> [Vec3; 4] {
    let [x1, z1, x2, z2] = node.bounds.map(|v| f32::from(v) * 10.0);
    let pad = if t2 { 0.0 } else { 10.0 };
    [
        (x1, z1),
        (x2 + pad, z1),
        (x2 + pad, z2 + pad),
        (x1, z2 + pad),
    ]
    .map(|(x, z)| {
        let mut p = glam::vec3(x, 0.0, z);
        p.y = super::mesh::height(node, p, t2);
        p
    })
}

impl Editor {
    pub(in crate::level_editor) fn campaign_viewport(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        if !self.grid {
            self.navigate(ui, &response);
        }
        let painter = ui.painter_at(rect);
        let pointer = ui
            .input(|i| i.pointer.hover_pos())
            .filter(|p| rect.contains(*p));
        let campaign = self.project.campaign.as_ref().unwrap();
        let t2 = campaign.source.t2();
        let Some(cache) = &self.campaign_cache else {
            return;
        };
        let mut minimum = Vec3::splat(f32::INFINITY);
        let mut maximum = Vec3::splat(f32::NEG_INFINITY);
        for &node in cache.nodes.values() {
            for point in corners(node, t2) {
                minimum = minimum.min(point);
                maximum = maximum.max(point);
            }
        }
        let center = (minimum + maximum) * 0.5;
        let extent = (maximum - minimum).max(Vec3::splat(1000.0));
        if self.grid {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(15, 20, 26));
            if response.hovered() {
                self.zoom = (self.zoom * (ui.input(|i| i.smooth_scroll_delta.y) * 0.002).exp())
                    .clamp(0.5, 30.0);
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                self.pan += ui.input(|i| i.pointer.delta());
            }
        }
        let scale = (rect.width() / extent.x).min(rect.height() / extent.z) * 0.9 * self.zoom;
        let screen = ui.ctx().screen_rect();
        let matrix = self.camera.matrix(screen.width() / screen.height());
        let project = |p: Vec3| -> Option<Pos2> {
            if self.grid {
                return Some(
                    rect.center() + self.pan + Vec2::new(p.x - center.x, p.z - center.z) * scale,
                );
            }
            let clip = matrix * p.extend(1.0);
            if clip.w <= 0.0 {
                return None;
            }
            let p = clip.truncate() / clip.w;
            Some(
                screen.min
                    + Vec2::new(
                        (p.x + 1.0) * screen.width() * 0.5,
                        (1.0 - p.y) * screen.height() * 0.5,
                    ),
            )
        };
        let ray = pointer.map(|p| {
            if self.grid {
                let q = (p - rect.center() - self.pan) / scale;
                (
                    glam::vec3(center.x + q.x, minimum.y - 10000.0, center.z + q.y),
                    Vec3::Y,
                )
            } else {
                crate::debug::camera::ray(
                    matrix,
                    [
                        (p.x - screen.min.x) / screen.width() * 2.0 - 1.0,
                        1.0 - (p.y - screen.min.y) / screen.height() * 2.0,
                    ],
                )
            }
        });
        let mut nearest: Option<(f32, [usize; 2], Vec3)> = None;
        for (&key, &source) in &cache.nodes {
            let node = campaign
                .terrain
                .iter()
                .find(|e| e.key == key)
                .map_or(source, |e| e.node);
            let quad = corners(node, t2);
            if let Some((origin, direction)) =
                ray.filter(|_| self.grid || self.campaign_pick_collision)
            {
                for triangle in [[0, 1, 2], [0, 2, 3]] {
                    if let Some(distance) = crate::debug::terrain::triangle(
                        origin,
                        direction,
                        triangle.map(|i| quad[i]),
                    ) && nearest.is_none_or(|(best, _, _)| distance < best)
                    {
                        nearest = Some((distance, key, origin + direction * distance));
                    }
                }
            }
            let selected = self.panel == Panel::Terrain && self.campaign_surface == Some(key);
            if self.grid || selected && self.guides {
                let points: Vec<_> = quad.into_iter().filter_map(project).collect();
                if points.len() == 4 {
                    if self.grid {
                        painter.add(egui::Shape::convex_polygon(
                            points.clone(),
                            if selected {
                                Color32::from_rgba_unmultiplied(190, 150, 65, 150)
                            } else if node.division == 0 {
                                Color32::from_rgba_unmultiplied(150, 65, 65, 30)
                            } else {
                                Color32::from_rgba_unmultiplied(70, 110, 110, 35)
                            },
                            Stroke::NONE,
                        ));
                    }
                    painter.add(egui::Shape::closed_line(
                        points,
                        Stroke::new(
                            if selected { 2.0_f32 } else { 0.5_f32 },
                            if selected {
                                Color32::GOLD
                            } else {
                                Color32::from_gray(80)
                            },
                        ),
                    ));
                }
            }
        }
        let visible_hit =
            if self.panel == Panel::Terrain && !self.grid && !self.campaign_pick_collision {
                ray.and_then(|(origin, direction)| self.campaign_mesh.pick(origin, direction))
            } else {
                None
            };
        if let Some(hit) = &visible_hit {
            let nodes = cache.nodes.iter().map(|(&key, &node)| {
                (
                    key,
                    campaign
                        .terrain
                        .iter()
                        .find(|e| e.key == key)
                        .map_or(node, |e| e.node),
                )
            });
            nearest =
                super::picking::surface(hit, nodes, t2).map(|key| (hit.distance, key, hit.point));
            if self.guides {
                let points: Vec<_> = hit.triangle.into_iter().filter_map(project).collect();
                if points.len() == 3 {
                    painter.add(egui::Shape::closed_line(
                        points,
                        Stroke::new(
                            2.0_f32,
                            if nearest.is_some() {
                                Color32::GOLD
                            } else {
                                Color32::LIGHT_RED
                            },
                        ),
                    ));
                }
            }
        }
        if response.clicked() && visible_hit.is_some() && nearest.is_none() {
            self.notice = "This visible face has no matching collision column. Use Collision surfaces to inspect invisible terrain, or Scenery for placed props.".into();
        }
        if self.panel != Panel::Terrain {
            nearest = ray.and_then(|(origin, direction)| {
                crate::debug::terrain::pick(&self.surfaces, origin, direction).map(|p| {
                    let point = Vec3::from_array(p.to_array().map(|v| v as f32));
                    ((point - origin).length(), [0, 0], point)
                })
            });
        }
        if let Some((_, _, p)) = nearest {
            let points: Vec<_> = [
                glam::vec3(-100., 0., 0.),
                glam::vec3(100., 0., 0.),
                glam::vec3(0., 0., -100.),
                glam::vec3(0., 0., 100.),
            ]
            .into_iter()
            .filter_map(|d| project(p + d))
            .collect();
            if points.len() == 4 && self.guides {
                painter.line_segment([points[0], points[1]], Stroke::new(2.0_f32, Color32::WHITE));
                painter.line_segment([points[2], points[3]], Stroke::new(2.0_f32, Color32::WHITE));
            }
        }
        // Marker categories are local to their inspector so dense maps remain selectable.
        let mut targets = Vec::new();
        if self.panel == Panel::Actors || self.tool == Tool::Patrol {
            targets.push((0, 0, campaign.start));
            targets.extend(
                campaign
                    .actors
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| a.enabled)
                    .map(|(i, a)| (0, i + 1, a.spawn.position)),
            );
            if let Some(actor) = self
                .actor
                .checked_sub(1)
                .and_then(|i| campaign.actors.get(i))
            {
                for pair in std::iter::once(actor.spawn.position)
                    .chain(actor.spawn.patrol.iter().map(|(p, _)| *p))
                    .collect::<Vec<_>>()
                    .windows(2)
                {
                    if self.guides
                        && let (Some(a), Some(b)) = (
                            project(Vec3::from_array(pair[0].map(|v| v as f32))),
                            project(Vec3::from_array(pair[1].map(|v| v as f32))),
                        )
                    {
                        painter.line_segment([a, b], Stroke::new(2.0_f32, Color32::GOLD));
                    }
                }
            }
        } else if self.panel == Panel::Scenery {
            targets.extend(
                campaign
                    .props
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (1, i, p.position)),
            );
            targets.extend(
                self.project
                    .scenery
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (2, i, p.position)),
            );
        } else if self.panel == Panel::Mission {
            targets.extend(
                campaign
                    .pickups
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (3, i, p.position)),
            );
        }
        let mut hit = None;
        let mut best = 14.0;
        for (kind, i, p) in targets {
            let Some(pos) = project(Vec3::from_array(p.map(|v| v as f32))) else {
                continue;
            };
            if !rect.contains(pos) {
                continue;
            }
            let selected = match kind {
                0 => self.actor == i,
                1 => self.campaign_prop == Some(i),
                2 => self.scenery_selection == Some(i),
                _ => self.campaign_pickup == Some(i),
            };
            if self.guides {
                painter.circle_filled(pos, 10., Color32::from_black_alpha(220));
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    if kind == 0 {
                        i.to_string()
                    } else if kind == 1 {
                        "P".into()
                    } else if kind == 2 {
                        "S".into()
                    } else {
                        "I".into()
                    },
                    egui::FontId::proportional(12.),
                    if selected {
                        Color32::GOLD
                    } else {
                        Color32::LIGHT_GREEN
                    },
                );
            }
            if let Some(pointer) = pointer {
                let distance = pointer.distance(pos);
                if distance < best {
                    best = distance;
                    hit = Some((kind, i));
                }
            }
        }
        if self.panel == Panel::Events && self.guides {
            for event in &self.project.events {
                if let project::Trigger::Enter { min, max } = event.trigger {
                    for axis in 0..3 {
                        for bits in 0..4 {
                            let mut a = min;
                            for n in 0..2 {
                                if bits & (1 << n) != 0 {
                                    let j = (axis + 1 + n) % 3;
                                    a[j] = max[j];
                                }
                            }
                            let mut b = a;
                            b[axis] = max[axis];
                            if let (Some(a), Some(b)) = (
                                project(Vec3::from_array(a.map(|v| v as f32))),
                                project(Vec3::from_array(b.map(|v| v as f32))),
                            ) {
                                painter.line_segment(
                                    [a, b],
                                    Stroke::new(1.0_f32, Color32::LIGHT_BLUE),
                                );
                            }
                        }
                    }
                }
            }
        }
        if let Some((_, _, p)) = nearest {
            self.campaign_cursor = p.to_array().map(|v| v.round() as i32);
        }
        if !self.unlocked {
            return;
        }
        let clicked = response.clicked_by(egui::PointerButton::Primary);
        let started = response.drag_started_by(egui::PointerButton::Primary);
        if (clicked || started)
            && let Some((kind, i)) = hit
        {
            match kind {
                0 => {
                    self.actor = i;
                    self.tool = Tool::Select;
                }
                1 => {
                    self.campaign_prop = Some(i);
                    self.scenery_selection = None;
                }
                2 => {
                    self.scenery_selection = Some(i);
                    self.campaign_prop = None;
                }
                _ => self.campaign_pickup = Some(i),
            }
            return;
        }
        if let Some((_, key, _)) = nearest
            && (clicked
                || response.dragged_by(egui::PointerButton::Primary) && self.tool != Tool::Patrol)
        {
            self.history.begin(&self.project);
            let point = std::array::from_fn(|axis| {
                let v = self.campaign_cursor[axis];
                if self.snap > 0 && axis != 1 {
                    (v as f32 / self.snap as f32).round() as i32 * self.snap
                } else {
                    v
                }
            });
            let campaign = self.project.campaign.as_mut().unwrap();
            match self.panel {
                Panel::Terrain => self.campaign_surface = Some(key),
                Panel::Actors => {
                    if self.actor == 0 {
                        campaign.start = point;
                    } else if let Some(actor) = campaign.actors.get_mut(self.actor - 1) {
                        if self.tool == Tool::Patrol {
                            if actor.spawn.patrol.len() < self.project.mission.limits.patrol {
                                actor
                                    .spawn
                                    .patrol
                                    .push((point, i32::from(self.patrol_wait)));
                                actor.spawn.think = 0x3453;
                            }
                        } else {
                            actor.spawn.position = point;
                        }
                    }
                }
                Panel::Scenery => {
                    if let Some(prop) = self.campaign_prop.and_then(|i| campaign.props.get_mut(i)) {
                        prop.position = point;
                    } else if let Some(prop) = self
                        .scenery_selection
                        .and_then(|i| self.project.scenery.get_mut(i))
                    {
                        prop.position = point;
                    }
                }
                Panel::Mission => {
                    if let Some(p) = self
                        .campaign_pickup
                        .and_then(|i| campaign.pickups.get_mut(i))
                    {
                        p.position = point;
                    }
                }
                _ => (),
            }
        }
    }
}
