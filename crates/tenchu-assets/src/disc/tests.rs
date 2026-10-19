use super::*;

fn both(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    bytes[at + 4..at + 8].copy_from_slice(&value.to_be_bytes());
}

fn record(name: &[u8], sector: u32, size: u32, directory: bool) -> Vec<u8> {
    let length = (33 + name.len()).next_multiple_of(2);
    let mut row = vec![0; length];
    row[0] = length as u8;
    both(&mut row, 2, sector);
    both(&mut row, 10, size);
    row[25] = if directory { 2 } else { 0 };
    row[28] = 1;
    row[31] = 1;
    row[32] = name.len() as u8;
    row[33..33 + name.len()].copy_from_slice(name);
    row
}

fn image() -> Vec<u8> {
    let mut data = vec![0; 32 * 2048];
    let pvd = &mut data[16 * 2048..17 * 2048];
    pvd[..7].copy_from_slice(b"\x01CD001\x01");
    pvd[128..132].copy_from_slice(&[0, 8, 8, 0]);
    both(pvd, 80, 40);
    pvd[156..190].copy_from_slice(&record(&[0], 20, 2048, true));
    let rows = [
        record(&[0], 20, 2048, true),
        record(&[1], 20, 2048, true),
        record(b"DATA.VOL;1", 21, 5000, false),
        record(b"CD.CA;1", 32, 8 * 2048, false),
    ]
    .concat();
    data[20 * 2048..20 * 2048 + rows.len()].copy_from_slice(&rows);
    for (i, byte) in data[21 * 2048..21 * 2048 + 5000].iter_mut().enumerate() {
        *byte = (i * 37) as u8;
    }
    data
}

#[test]
fn cooked_and_raw_images_read_identically_across_sector_boundaries() {
    let cooked = image();
    let expected = &cooked[21 * 2048 + 2000..21 * 2048 + 4700];
    for format in [
        SectorFormat::Iso,
        SectorFormat::Mode1,
        SectorFormat::Mode2,
        SectorFormat::Mode2_2336,
    ] {
        let (stride, header) = format.layout();
        for pregap in [0, 150] {
            let mut raw = vec![0; (32 + pregap) * stride as usize];
            for (sector, bytes) in cooked.chunks_exact(2048).enumerate() {
                let at = (pregap + sector) * stride as usize + header as usize;
                raw[at..at + 2048].copy_from_slice(bytes);
            }
            let disc = if pregap == 0 {
                Disc::detect(raw).unwrap()
            } else {
                Disc::open(raw, format, pregap as u64).unwrap()
            };
            let entry = disc.find("/data.vol").unwrap();
            assert_eq!(disc.file(&entry).read(2000, 2700).unwrap(), expected);
            assert!(disc.file(&entry).read(4990, 11).is_err());
            let audio = disc.find("CD.CA").unwrap();
            assert!(disc.file(&audio).read(0, 1).is_err());
        }
    }
}

#[test]
fn corrupt_extents_and_truncated_records_are_rejected() {
    let mut data = image();
    both(&mut data, 20 * 2048 + 68 + 2, 31);
    let disc = Disc::detect(data).unwrap();
    let entry = disc.find("DATA.VOL").unwrap();
    assert!(disc.file(&entry).read(0, entry.size as usize).is_err());
    let mut data = image();
    data[20 * 2048 + 68] = 12;
    assert!(Disc::detect(data).unwrap().find("DATA.VOL").is_err());
    let mut data = image();
    data[16 * 2048 + 84] = 1;
    assert!(Disc::detect(data).is_err());
    let mut data = image();
    data.truncate(22 * 2048);
    let disc = Disc::detect(data).unwrap();
    let entry = disc.find("DATA.VOL").unwrap();
    assert!(disc.file(&entry).read(0, entry.size as usize).is_err());
}

#[test]
fn xa_reads_full_form_two_sectors_without_exposing_them_as_files() {
    let cooked = image();
    assert!(Disc::detect(&cooked).unwrap().xa_sector(21).is_err());
    for format in [SectorFormat::Mode2, SectorFormat::Mode2_2336] {
        let (stride, header) = format.layout();
        let mut raw = vec![0; 182 * stride as usize];
        for (sector, bytes) in cooked.chunks_exact(2048).enumerate() {
            let at = (150 + sector) * stride as usize + header as usize;
            raw[at..at + 2048].copy_from_slice(bytes);
        }
        let at = 171 * stride as usize + header as usize - 8;
        raw[at..at + 8].copy_from_slice(&[1, 3, 0x64, 1, 1, 3, 0x64, 1]);
        raw[at + 8..at + 2336].fill(0xa5);
        let disc = Disc::open(&raw, format, 150).unwrap();
        let sector = disc.xa_sector(21).unwrap();
        assert_eq!(&sector[..8], &[1, 3, 0x64, 1, 1, 3, 0x64, 1]);
        assert!(sector[8..].iter().all(|&byte| byte == 0xa5));
        assert!(disc.read(21 * 2048, 1).is_err());
        assert!(disc.xa_sector(32).is_err());
        raw[at + 4] ^= 1;
        assert!(Disc::open(raw, format, 150).unwrap().xa_sector(21).is_err());
    }
}
