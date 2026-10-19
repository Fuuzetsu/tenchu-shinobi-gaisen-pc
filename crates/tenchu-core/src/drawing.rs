use crate::{
    Result,
    effects::Effects,
    math::{Math, Matrix},
    packets, projection,
};
use tenchu_assets::{executable::Executable, stage::MissionAssets};

#[derive(Debug)]
pub struct Drawing {
    pub fog: bool,
    pub bytes: u32,
    pub sampled_bytes: u32,
    pub subdivision: packets::Subdivision,
}

impl Drawing {
    pub fn new(exe: &Executable) -> Result<Self> {
        Ok(Self {
            fog: false,
            bytes: 0,
            sampled_bytes: 0,
            subdivision: packets::Subdivision::load(exe)?,
        })
    }

    pub fn finish(&mut self, clock: i32) {
        if clock % 30 == 0 {
            self.sampled_bytes = self.bytes.min(65536);
        }
    }

    pub fn effects(
        &mut self,
        effects: &Effects,
        mission: &MissionAssets,
        math: &Math,
        view: Matrix,
    ) -> Result<()> {
        let shadow = mission.common_models[&25]
            .first()
            .ok_or("Missing shadow geometry")?;
        for instance in &effects.shadows {
            let mut rotation = math.rotate_yxz(instance.rotation);
            for row in &mut rotation {
                for coefficient in row {
                    *coefficient =
                        (i32::from(*coefficient).wrapping_mul(instance.scale) >> 12) as i16;
                }
            }
            let matrix = view.compose(&Matrix {
                rotation,
                translation: instance.position,
            });
            if projection::point(&matrix, [0; 3]).z >> 4 < 1250 {
                self.bytes += packets::sdk_plain(shadow, &matrix)?;
            }
        }
        for wire in &effects.wires {
            self.bytes += wire.lines.len() as u32 * 20;
            if let Some((matrix, fog)) = wire.hook {
                self.fog = fog;
                if let Some(mesh) = mission.common_models[&26].first() {
                    self.bytes += packets::draw_tmd(mesh, &view.compose(&matrix), fog);
                }
            }
        }
        self.bytes += effects.fades.len() as u32 * 32;
        self.bytes += effects.texture_moves.len() as u32 * 24;
        self.bytes += effects
            .drawings
            .iter()
            .map(crate::effects::Drawing::bytes)
            .sum::<u32>();
        Ok(())
    }
}
