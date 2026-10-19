pub mod afs;
pub mod cue;
pub mod disc;
pub mod game;

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
    let data = bytes.get(at..at + 4).ok_or("Missing 32-bit field")?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()))
}

fn u32_be(bytes: &[u8], at: usize) -> Result<u32> {
    let data = bytes.get(at..at + 4).ok_or("Missing 32-bit field")?;
    Ok(u32::from_be_bytes(data.try_into().unwrap()))
}

fn text(bytes: &[u8]) -> Result<String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end])
        .map(str::to_owned)
        .map_err(|e| format!("Invalid asset name: {e}"))
}
