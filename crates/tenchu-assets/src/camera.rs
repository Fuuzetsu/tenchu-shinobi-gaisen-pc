use crate::{Result, executable::Executable, i16_le, short3};

pub type Pose = [[i16; 3]; 4];

#[derive(Clone, Debug)]
pub struct CameraTables {
    pub ordinary: Pose,
    pub critical: [Pose; 4],
    pub poses: [Pose; 10],
    pub wall: [[i16; 3]; 2],
    pub stick: [[i16; 3]; 2],
    pub smoothing: [[i16; 6]; 2],
}

impl CameraTables {
    pub fn load(exe: &Executable) -> Result<Self> {
        let pose = |address| -> Result<Pose> {
            let data = exe.bytes(address, 32)?;
            Ok([
                short3(data, 0)?,
                short3(data, 8)?,
                short3(data, 16)?,
                short3(data, 24)?,
            ])
        };
        let mut critical = [[[0; 3]; 4]; 4];
        let mut poses = [[[0; 3]; 4]; 10];
        for (i, row) in critical.iter_mut().enumerate() {
            *row = pose(0x80089f50 + i as u32 * 32)?;
        }
        for (i, row) in poses.iter_mut().enumerate() {
            *row = pose(0x80011a64 + i as u32 * 32)?;
        }
        let mut smoothing = [[0; 6]; 2];
        for (i, row) in smoothing.iter_mut().enumerate() {
            let data = exe.bytes(0x80089f10 + i as u32 * 16, 12)?;
            for (j, value) in row.iter_mut().enumerate() {
                *value = i16_le(data, j * 2)?;
            }
        }
        Ok(Self {
            ordinary: pose(0x80011bc0)?,
            critical,
            poses,
            smoothing,
            wall: [
                short3(exe.bytes(0x800979f4, 6)?, 0)?,
                short3(exe.bytes(0x800979fc, 6)?, 0)?,
            ],
            stick: [
                short3(exe.bytes(0x80097a28, 6)?, 0)?,
                short3(exe.bytes(0x80097a30, 6)?, 0)?,
            ],
        })
    }
}
