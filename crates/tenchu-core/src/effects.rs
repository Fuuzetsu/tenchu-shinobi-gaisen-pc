use crate::random::Random;

#[derive(Clone, Debug)]
pub struct Smoke {
    pub position: [i32; 3],
    pub velocity: [i16; 3],
    pub scale: i32,
    pub time: u8,
    pub change_at: u8,
}

#[derive(Clone, Debug)]
pub struct Effects {
    pub smoke: Vec<Option<Smoke>>,
    cursor: usize,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            smoke: vec![None; 201],
            cursor: 0,
        }
    }
}

impl Effects {
    fn allocate(&mut self) -> usize {
        let mut i = self.cursor;
        for _ in 0..200 {
            i = (i + 1) % 200;
            if self.smoke[i].is_none() {
                self.cursor = (i + 1) % 200;
                return i;
            }
        }
        200
    }

    pub fn smoke_burst(
        &mut self,
        position: [i32; 3],
        spread: u16,
        divisor: i16,
        count: i16,
        rng: &mut Random,
    ) {
        for _ in 0..count {
            let slot = self.allocate();
            let width = i32::from(spread as i16) * 2;
            let x = (if width > 0 { rng.next15() % width } else { 0 } - i32::from(spread)) as i16;
            let z = (if width > 0 { rng.next15() % width } else { 0 } - i32::from(spread)) as i16;
            let velocity = [x, -5, z];
            self.smoke[slot] = Some(Smoke {
                position: std::array::from_fn(|i| position[i].wrapping_add(i32::from(velocity[i]))),
                velocity: velocity.map(|v| (i32::from(v) / i32::from(divisor)) as i16),
                scale: rng.next15() % 0x2000 + 0x1000,
                time: 15,
                change_at: (7 - rng.next15() % 15) as u8,
            });
        }
    }

    pub fn step(&mut self, rng: &mut Random) {
        for slot in &mut self.smoke[..200] {
            let Some(smoke) = slot else {
                continue;
            };
            if smoke.time == smoke.change_at {
                if smoke.velocity[1] < -20 {
                    smoke.velocity[0] = (i32::from(smoke.velocity[0]) * 80 / 100) as i16;
                    smoke.velocity[1] /= 2;
                    smoke.velocity[2] = (i32::from(smoke.velocity[2]) * 80 / 100) as i16;
                }
                smoke.scale = smoke.scale.wrapping_add(1024);
                smoke.change_at = (i32::from(smoke.time) - 1 - rng.next15() % 5) as u8;
            }
            for i in 0..3 {
                smoke.position[i] = smoke.position[i].wrapping_add(i32::from(smoke.velocity[i]));
            }
            if smoke.time == 0 {
                *slot = None;
            } else {
                smoke.time -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pool_keeps_rng_cost_but_does_not_tick_dummy_smoke() {
        let mut effects = Effects::default();
        let mut rng = Random::new(1);
        effects.smoke_burst([0; 3], 0, 12, 201, &mut rng);
        assert_eq!(rng.calls, 402);
        assert!(effects.smoke.iter().all(Option::is_some));
        // Avoid event rolls: this test isolates the inclusive zero lifetime and dummy slot.
        for smoke in effects.smoke.iter_mut().flatten() {
            smoke.change_at = 255;
        }
        for _ in 0..15 {
            effects.step(&mut rng);
        }
        assert!(
            effects.smoke[..200]
                .iter()
                .all(|s| s.as_ref().unwrap().time == 0)
        );
        effects.step(&mut rng);
        assert!(effects.smoke[..200].iter().all(Option::is_none));
        assert_eq!(effects.smoke[200].as_ref().unwrap().time, 15);
        assert_eq!(rng.calls, 402);
        effects.smoke_burst([0; 3], 150, 12, 1, &mut rng);
        assert_eq!(rng.calls, 406);
    }
}
