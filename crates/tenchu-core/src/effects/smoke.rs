use super::*;
use crate::math::Position;

impl Effects {
    pub fn smoke(
        &mut self,
        position: Position,
        velocity: [i16; 3],
        count: i16,
        time: i16,
        rng: &mut Random,
    ) -> Result<()> {
        for _ in 0..count {
            self.new_smoke(position, velocity, time, true, rng)?;
        }
        Ok(())
    }

    pub fn smoke_single(
        &mut self,
        position: Position,
        velocity: [i16; 3],
        time: i16,
        rng: &mut Random,
    ) -> Result<()> {
        self.new_smoke(position, velocity, time, false, rng)
    }

    fn new_smoke(
        &mut self,
        position: Position,
        mut velocity: [i16; 3],
        duration: i16,
        jitter: bool,
        rng: &mut Random,
    ) -> Result<()> {
        if duration == 0 {
            return Err("Smoke has a zero random-event interval".into());
        }
        let slot = self.allocate();
        let scale = rng.next15() % 8192 + 4096;
        let rotation = (rng.next15() % 360) * 4096;
        if jitter {
            velocity = velocity.map(|v| v.wrapping_add((rng.next15() % 100 - 50) as i16));
        }
        let time = (i32::from(duration) + if jitter { rng.next15() % 160 } else { 0 }) as u8;
        let change_at =
            (i32::from(time) - 1 - (i32::from(duration) / 2 + rng.next15() % i32::from(duration)))
                as u8;
        self.slots[slot] = Some(Effect::Smoke(Smoke {
            position,
            velocity,
            scale,
            time,
            change_at,
            sprite: 0,
            rotation,
        }));
        Ok(())
    }

    pub(super) fn update_smoke(&mut self, smoke: &mut Smoke, rng: &mut Random, view: View) -> bool {
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
        let color = if smoke.time < 26 { smoke.time * 5 } else { 128 };
        self.sprite(
            Graphic::Smoke(smoke.sprite),
            smoke.position,
            smoke.scale,
            color,
            smoke.rotation,
            view,
        );
        let alive = smoke.time != 0;
        smoke.time = smoke.time.wrapping_sub(1);
        alive
    }
}
