use super::{Edition, executable};
use crate::{
    ReadAt, Result, camera::Pose, disc::Disc, executable::Executable, i16_le, short3, u32_le,
};

pub struct Tables {
    pub poses: [[Pose; 2]; 22],
    pub normal: [i16; 5],
    pub ready: [i16; 32],
    pub stealth: [[Pose; 4]; 7],
    pub origins: [[[i32; 3]; 3]; 7],
    pub smoothing: [[i16; 6]; 2],
    pub wall_probes: [[i16; 3]; 2],
    pub wall_push: [[i16; 2]; 16],
}

impl Tables {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan {
            return Err("Native Tenchu 2 camera requires the Japanese disc".into());
        }
        let exe = Executable::new(&bytes)?;
        let pose = |address| -> Result<Pose> {
            let bytes = exe.bytes(address, 32)?;
            Ok([
                short3(bytes, 0)?,
                short3(bytes, 8)?,
                short3(bytes, 16)?,
                short3(bytes, 24)?,
            ])
        };
        let mut tables = Self {
            poses: [[[[0; 3]; 4]; 2]; 22],
            normal: [0; 5],
            ready: [0; 32],
            stealth: [[[[0; 3]; 4]; 4]; 7],
            origins: [[[0; 3]; 3]; 7],
            smoothing: [[0; 6]; 2],
            wall_probes: [[0; 3]; 2],
            wall_push: [[0; 2]; 16],
        };
        for (i, value) in tables.poses.iter_mut().flatten().enumerate() {
            *value = pose(0x80084028 + i as u32 * 32)?;
        }
        for (i, value) in tables
            .normal
            .iter_mut()
            .chain(&mut tables.ready)
            .enumerate()
        {
            *value = u32_le(exe.bytes(0x800845a8 + i as u32 * 4, 4)?, 0)? as i16;
            if !(0..22).contains(value) {
                return Err("Invalid Tenchu 2 camera mode".into());
            }
        }
        for (i, value) in tables.stealth.iter_mut().flatten().enumerate() {
            *value = pose(0x8008463c + i as u32 * 32)?;
        }
        for (i, value) in tables.origins.iter_mut().flatten().enumerate() {
            let bytes = exe.bytes(0x800849bc + i as u32 * 16, 12)?;
            for (j, axis) in value.iter_mut().enumerate() {
                *axis = u32_le(bytes, j * 4)? as i32;
            }
        }
        for i in 0..2 {
            let bytes = exe.bytes(0x80084b0c + i as u32 * 16, 12)?;
            for (j, value) in tables.smoothing[i].iter_mut().enumerate() {
                *value = i16_le(bytes, j * 2)?;
            }
            tables.wall_probes[i] = short3(exe.bytes(0x8008e9a8 + i as u32 * 8, 6)?, 0)?;
        }
        let bytes = exe.bytes(0x80013060, 64)?;
        for (i, value) in tables.wall_push.iter_mut().flatten().enumerate() {
            *value = i16_le(bytes, i * 2)?;
        }
        Ok(tables)
    }
}
