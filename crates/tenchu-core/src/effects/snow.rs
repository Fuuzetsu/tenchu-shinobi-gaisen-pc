use super::*;

#[derive(Clone, Debug)]
pub struct Snow {
    pub position: [i32; 3],
    pub velocity: [i16; 3],
    pub ground: i32,
    pub sample_y: i32,
    pub size: i32,
}

impl Effects {
    pub fn snow(
        &mut self,
        position: [i32; 3],
        velocity: [i16; 3],
        size: i32,
        map: &tenchu_assets::terrain::AreaMap,
        cursor: &mut Cursor,
    ) -> Result<()> {
        let slot = self.allocate();
        let sample_y = position[1].wrapping_sub(8000);
        let ground = terrain::level(map, cursor, [position[0], sample_y, position[2]], 8)?;
        self.slots[slot] = Some(Effect::Snow(Snow {
            position,
            velocity,
            ground,
            sample_y,
            size,
        }));
        Ok(())
    }

    pub(super) fn update_snow(
        &mut self,
        snow: &mut Snow,
        view: View,
        env: &mut Environment,
    ) -> Result<bool> {
        let mut position: [i32; 3] =
            std::array::from_fn(|i| snow.position[i].wrapping_add(i32::from(snow.velocity[i])));
        if snow.ground < position[1] {
            return Ok(false);
        }
        let mut wrapped = false;
        for axis in [1, 0, 2] {
            let delta = position[axis].wrapping_sub(env.reference[axis]);
            if (axis == 1 && delta > 3000) || (axis != 1 && delta.wrapping_abs() > 3000) {
                position[axis] =
                    env.reference[axis].wrapping_add((delta as u32 % 6000) as i32 - 3000);
                wrapped = true;
            }
        }
        if wrapped {
            let ground = terrain::level(
                env.terrain,
                env.cursor,
                [position[0], snow.sample_y, position[2]],
                8,
            )?;
            if ground < position[1] {
                return Ok(false);
            }
            snow.ground = ground;
        }
        snow.position = position;
        if let Some(drawing) = view.sprite(
            position,
            Graphic::Sprite {
                image: 55,
                blend: 2,
            },
            snow.size,
            128,
        ) {
            self.drawings.push(drawing);
        }
        Ok(true)
    }
}
