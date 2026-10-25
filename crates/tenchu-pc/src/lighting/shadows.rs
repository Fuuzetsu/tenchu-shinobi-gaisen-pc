use super::Light;

#[derive(Clone, Copy)]
struct Slot {
    position: glam::Vec3,
    strength: f32,
}

#[derive(Default)]
pub struct Selection {
    slots: Vec<Option<Slot>>,
}

impl Selection {
    pub fn update(
        &mut self,
        lights: &[Light],
        budget: usize,
        elapsed: Option<f32>,
    ) -> Vec<Option<(usize, f32)>> {
        let mut selected = vec![None; lights.len()];
        let first = self.slots.is_empty();
        self.slots.resize(budget, None);
        let step = elapsed.map_or(1.0, |dt| dt.clamp(0.0, 0.1) / 0.3);
        for (slot, entry) in self.slots.iter_mut().enumerate() {
            let Some(previous) = entry else {
                continue;
            };
            let matched = lights
                .iter()
                .enumerate()
                .filter(|(i, light)| {
                    selected[*i].is_none()
                        && light.position.distance_squared(previous.position) < 2000.0_f32.powi(2)
                })
                .min_by(|(_, a), (_, b)| {
                    a.position
                        .distance_squared(previous.position)
                        .total_cmp(&b.position.distance_squared(previous.position))
                });
            let Some((index, light)) = matched else {
                *entry = None;
                continue;
            };
            previous.position = light.position;
            previous.strength =
                (previous.strength + if index < budget { step } else { -step }).clamp(0.0, 1.0);
            if previous.strength == 0.0 {
                *entry = None;
            } else {
                selected[index] = Some((slot, previous.strength));
            }
        }
        for index in 0..budget.min(lights.len()) {
            if selected[index].is_some() {
                continue;
            }
            let Some(slot) = self.slots.iter().position(Option::is_none) else {
                break;
            };
            let strength = if first || elapsed.is_none() {
                1.0
            } else {
                step
            };
            self.slots[slot] = Some(Slot {
                position: lights[index].position,
                strength,
            });
            selected[index] = Some((slot, strength));
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_budget_handover_fades_without_changing_on_list_reordering() {
        let a = Light {
            position: glam::Vec3::ZERO,
            color: [1.0; 3],
            radius: 12000.0,
        };
        let b = Light {
            position: glam::Vec3::X * 3000.0,
            ..a
        };
        let mut selection = Selection::default();
        assert_eq!(
            selection.update(&[a, b], 1, Some(1.0 / 60.0)),
            [Some((0, 1.0)), None]
        );
        let changed = selection.update(&[b, a], 1, Some(1.0 / 60.0));
        assert!(changed[0].is_none());
        assert!((changed[1].unwrap().1 - 0.94444).abs() < 0.001);
        for _ in 0..40 {
            selection.update(&[b, a], 1, Some(1.0 / 60.0));
        }
        assert_eq!(
            selection.update(&[b, a], 1, Some(0.0)),
            [Some((0, 1.0)), None]
        );
        let mut both = Selection::default();
        both.update(&[a, b], 2, None);
        assert_eq!(
            both.update(&[b, a], 2, Some(0.0)),
            [Some((1, 1.0)), Some((0, 1.0))]
        );
    }
}
