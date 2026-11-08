use crate::{
    ReadAt, Result,
    disc::Disc,
    executable::{Executable, MathTables},
};

pub fn math_tables(disc: &Disc<impl ReadAt>) -> Result<MathTables> {
    let (edition, data) = super::executable(disc)?;
    let executable = Executable::new(&data)?;
    let addresses = edition.addresses();
    let shorts = |address, count: usize| -> Result<Vec<i16>> {
        Ok(executable
            .bytes(address, count * 2)?
            .chunks_exact(2)
            .map(|v| i16::from_le_bytes([v[0], v[1]]))
            .collect())
    };
    let quadrant = shorts(addresses.sine, 1025)?;
    let sine = |angle: usize| match angle & 4095 {
        a @ 0..=1024 => quadrant[a],
        a @ 1025..=2048 => quadrant[2048 - a],
        a @ 2049..=3072 => -quadrant[a - 2048],
        a => -quadrant[4096 - a],
    };
    Ok(MathTables {
        trig: (0..4096).map(|a| [sine(a), sine(a + 1024)]).collect(),
        atan: shorts(addresses.atan, 1025)?,
        sqrt: shorts(addresses.sqrt - 640, 512)?,
        normal: shorts(addresses.normal - 640, 512)?,
        hermite: super::motions::spline_basis(disc)?,
    })
}
