use super::*;
use crate::math::Position;

#[derive(Clone, Debug)]
pub struct Impact {
    pub parent: Option<ModelId>,
    pub position: Position,
    pub size: [i16; 2],
    pub color: [[u8; 3]; 2],
    pub rotation: i16,
    pub speed: i16,
    pub sprite: u8,
    pub count: u8,
    pub time: u8,
}

impl Effects {
    pub fn impact(&mut self, position: Position, size: i16, sprite: u8, rng: &mut Random) {
        let speed = (rng.next15() % 90 + 90) as i16;
        self.impact_ex(Impact {
            parent: None,
            position,
            size: [size, 0],
            color: [[128; 3]; 2],
            rotation: 0,
            speed,
            sprite,
            count: 0,
            time: 15,
        });
    }

    pub fn impact_ex(&mut self, impact: Impact) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Impact(impact));
    }

    pub(super) fn update_impact(
        &mut self,
        impact: &mut Impact,
        view: View,
        scene: &mut Scene,
    ) -> Result<bool> {
        if impact.time == 0 {
            return Err("Zero impact duration".into());
        }
        let ratio = (i32::from(impact.count) << 12) / i32::from(impact.time);
        let mix =
            |a: i32, b: i32| a.wrapping_mul(4096 - ratio) / 4096 + b.wrapping_mul(ratio) / 4096;
        let size = mix(i32::from(impact.size[0]), i32::from(impact.size[1]));
        let rotation = i32::from(impact.rotation) << 12;
        impact.rotation = impact.rotation.wrapping_add(impact.speed);
        let color = std::array::from_fn(|i| {
            mix(i32::from(impact.color[0][i]), i32::from(impact.color[1][i])) as u8
        });
        let (position, xy, z) = if let Some(parent) = impact.parent {
            let world = scene.world(parent)?;
            let matrix = view.matrix.compose(&world);
            let (xy, z) =
                visibility::project(&matrix, impact.position.map(|v| v as i16).to_array());
            (
                world.transform(impact.position.map(|v| v as i16).to_array()),
                xy,
                (z >> 2) as i16,
            )
        } else {
            let (xy, z) = view.project(impact.position);
            (impact.position, xy, z)
        };
        self.billboards.push(Billboard {
            graphic: Graphic::Impact(impact.sprite),
            position,
            scale: [size; 2],
            color,
            rotation,
        });
        if z > 36 {
            let scale = i32::from((size.wrapping_mul(300) / i32::from(z)) as i16) + 1;
            self.drawings.push(Drawing {
                graphic: Graphic::Impact(impact.sprite),
                xy,
                scale: [scale; 2],
                color,
                rotation,
                depth: (z >> 2).clamp(0, 1249),
            });
        }
        let alive = impact.count < impact.time;
        impact.count = impact.count.wrapping_add(1);
        Ok(alive)
    }
}
