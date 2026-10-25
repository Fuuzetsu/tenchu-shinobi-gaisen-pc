use super::*;

pub(super) struct Eave {
    position: Vec3,
    velocity: Vec3,
    cooldown: u16,
}

impl Weather {
    pub(super) fn update_runoff(
        &mut self,
        center: Vec3,
        surfaces: &Surfaces,
        settings: &Settings,
        landings: &mut Vec<Hit>,
    ) {
        if self.mode != WeatherMode::Rain || !settings.runoff || settings.runoff_amount == 0 {
            self.runoff.clear();
            self.eaves.clear();
            return;
        }
        let wind = settings.velocity(380.0) * Vec3::new(0.06, 0.0, 0.06);
        let mut edges = Vec::new();
        self.runoff.retain_mut(|p| {
            p.previous = p.position;
            p.life = p.life.saturating_sub(1);
            if let Some(surface) = p.hit {
                let downhill = Vec3::Y - surface.normal * surface.normal.y;
                p.velocity = (p.velocity + downhill * 14.0).clamp_length_max(140.0);
                let next = p.position + p.velocity;
                if let Some(hit) = surfaces.cast(next - Vec3::Y * 100.0, next + Vec3::Y * 160.0) {
                    p.position = hit.position + hit.normal * 5.0;
                    p.hit = Some(hit);
                    if hit.normal.y < -0.98 || hit.normal.y > -0.1 {
                        return false;
                    }
                } else {
                    p.position = next;
                    p.hit = None;
                    edges.push((next, p.velocity * 0.2));
                }
            } else {
                p.velocity.x += (wind.x - p.velocity.x) * 0.04;
                p.velocity.z += (wind.z - p.velocity.z) * 0.04;
                p.velocity.y = (p.velocity.y + 12.0).min(300.0);
                p.position += p.velocity;
                if let Some(hit) = surfaces.cast(p.previous, p.position) {
                    landings.push(hit);
                    return false;
                }
            }
            p.life != 0 && p.position.distance_squared(center) < (RADIUS * 1.5).powi(2)
        });
        self.eaves
            .retain(|e| e.position.distance_squared(center) < RADIUS * RADIUS);
        for (position, velocity) in edges {
            if self.eaves.len() >= EAVE_LIMIT {
                break;
            }
            if self
                .eaves
                .iter()
                .any(|e| e.position.distance_squared(position) < 250.0 * 250.0)
            {
                continue;
            }
            // Only persist a drip if there is a real fall beneath the roof edge.
            if surfaces
                .cast(position, position + Vec3::Y * 350.0)
                .is_none()
            {
                self.eaves.push(Eave {
                    position,
                    velocity,
                    cooldown: 0,
                });
            }
        }
        let flow = f32::from(settings.amount) * f32::from(settings.runoff_amount) / 10000.0;
        let interval = (8.0 / (flow * self.wet).max(0.03)).clamp(4.0, 300.0) as u16;
        for eave in &mut self.eaves {
            eave.cooldown = eave.cooldown.saturating_sub(1);
            if eave.cooldown != 0 || self.runoff.len() >= RUNOFF_LIMIT || flow == 0.0 {
                continue;
            }
            eave.cooldown = interval;
            self.runoff.push(Particle {
                previous: eave.position,
                position: eave.position,
                velocity: eave.velocity + settings.velocity(45.0) * Vec3::new(0.1, 1.0, 0.1),
                hit: None,
                life: 90,
                size: 5.0,
            });
        }
        // Sample catchment independently of visible particles so eaves still drip
        // at low particle detail and when rain streaks are hidden.
        for _ in 0..(4 + usize::from(settings.quality.min(2)) * 4) {
            if self.runoff.len() >= RUNOFF_LIMIT * 3 / 4 || self.random() > flow * self.wet {
                continue;
            }
            let start = Vec3::new(
                center.x + (self.random() * 2.0 - 1.0) * RADIUS,
                surfaces.top() - 100.0,
                center.z + (self.random() * 2.0 - 1.0) * RADIUS,
            );
            let end = Vec3::new(start.x, center.y + HEIGHT, start.z);
            let Some(hit) = surfaces.cast(start, end) else {
                continue;
            };
            if hit.normal.y >= -0.2
                || hit.normal.y <= -0.98
                || hit.normal.dot(settings.direction()) >= 0.0
                || hit.position.distance_squared(center) > RADIUS * RADIUS
                || !surfaces.exposed(hit.position + hit.normal * 5.0, settings.direction())
            {
                continue;
            }
            let downhill = Vec3::Y - hit.normal * hit.normal.y;
            let size = 4.0 + self.random() * 3.0;
            self.runoff.push(Particle {
                previous: hit.position + hit.normal * 5.0,
                position: hit.position + hit.normal * 5.0,
                velocity: downhill * 65.0,
                hit: Some(hit),
                life: 180,
                size,
            });
        }
    }
}
