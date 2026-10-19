pub mod afs;
pub mod camera;
pub mod cue;
pub mod disc;
pub mod executable;
pub mod game;
pub mod model;
pub mod motion;
pub mod stage;
pub mod terrain;
pub mod texture;

pub type Result<T> = std::result::Result<T, String>;

pub trait ReadAt {
    fn byte_len(&self) -> u64;
    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()>;

    fn read(&self, offset: u64, size: usize) -> Result<Vec<u8>> {
        check_span(offset, size as u64, self.byte_len())?;
        let mut output = vec![0; size];
        self.read_exact_at(offset, &mut output)?;
        Ok(output)
    }
}

impl<T: AsRef<[u8]>> ReadAt for T {
    fn byte_len(&self) -> u64 {
        self.as_ref().len() as u64
    }

    fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        let bytes = self.as_ref();
        check_span(offset, output.len() as u64, bytes.len() as u64)?;
        output.copy_from_slice(&bytes[offset as usize..offset as usize + output.len()]);
        Ok(())
    }
}

pub fn check_span(offset: u64, size: u64, length: u64) -> Result<()> {
    match offset.checked_add(size) {
        Some(end) if end <= length => Ok(()),
        _ => Err(format!(
            "Read outside data: {offset}+{size}, length {length}"
        )),
    }
}

fn u32_le(bytes: &[u8], at: usize) -> Result<u32> {
    let data = slice(bytes, at, 4)?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}

fn u32_be(bytes: &[u8], at: usize) -> Result<u32> {
    let data = slice(bytes, at, 4)?;
    Ok(u32::from_be_bytes(data.try_into().unwrap()))
}

fn slice(bytes: &[u8], at: usize, size: usize) -> Result<&[u8]> {
    at.checked_add(size)
        .and_then(|end| bytes.get(at..end))
        .ok_or_else(|| format!("Missing asset bytes {at}+{size}, length {}", bytes.len()))
}

fn u16_le(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(slice(bytes, at, 2)?.try_into().unwrap()))
}

fn i16_le(bytes: &[u8], at: usize) -> Result<i16> {
    Ok(u16_le(bytes, at)? as i16)
}

fn short3(bytes: &[u8], at: usize) -> Result<[i16; 3]> {
    let row = slice(bytes, at, 6)?;
    Ok([i16_le(row, 0)?, i16_le(row, 2)?, i16_le(row, 4)?])
}

fn relative(bytes: &[u8], base: usize, field: usize) -> Result<usize> {
    let at = base
        .checked_add(u32_le(bytes, field)? as usize)
        .ok_or("Asset offset overflow")?;
    slice(bytes, at, 1)?;
    Ok(at)
}

fn text(bytes: &[u8]) -> Result<String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end])
        .map(str::to_owned)
        .map_err(|e| format!("Invalid asset name: {e}"))
}
