use super::{DEFAULT_TILES, THEMES};
use crate::{Result, i16_le, u16_le, u32_le};

pub const SIZE: usize = 0x1d3c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Objective {
    ReachLord,
    Assassinate,
    EliminateEnemies,
    ProtectPrincess,
    FindHerb,
}
impl Objective {
    pub const ALL: [Self; 5] = [
        Self::ReachLord,
        Self::Assassinate,
        Self::EliminateEnemies,
        Self::ProtectPrincess,
        Self::FindHerb,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::ReachLord => "Reach Lord Gohda",
            Self::Assassinate => "Assassinate the target",
            Self::EliminateEnemies => "Eliminate all enemies",
            Self::ProtectPrincess => "Escort Princess Kiku",
            Self::FindHerb => "Find the medicinal herb",
        }
    }
    pub fn reserved(self) -> usize {
        if self == Self::ProtectPrincess { 3 } else { 2 }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub z: i16,
}
impl Point {
    pub fn valid(self) -> bool {
        (0..32).contains(&self.x) && (0..32).contains(&self.z)
    }
    pub fn world(self) -> [i32; 3] {
        [
            i32::from(self.x) * 2000 + 1000,
            0,
            i32::from(self.z) * -2000 - 1000,
        ]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ground: u8,
    pub ornament: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Waypoint {
    pub point: Point,
    pub facing: i16,
    pub wait: i16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub kind: i16,
    pub point: Point,
    pub route: Vec<Waypoint>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub ticks: i32,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mission {
    pub title: String,
    pub author: String,
    pub password: String,
    pub theme: usize,
    pub objective: Objective,
    pub stealth: bool,
    pub map: Vec<Cell>,
    pub placements: Vec<Placement>,
    pub records: Vec<Record>,
    pub preserved_checksum_bit: bool,
}

fn read_text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    encoding_rs::SHIFT_JIS.decode(&bytes[..end]).0.into_owned()
}
fn write_text(bytes: &mut [u8], value: &str) -> Result<()> {
    let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(value);
    if error || encoded.len() >= bytes.len() {
        return Err(format!(
            "Text must fit {} Shift-JIS bytes: {value}",
            bytes.len() - 1
        ));
    }
    bytes.fill(0);
    bytes[..encoded.len()].copy_from_slice(&encoded);
    Ok(())
}
impl Mission {
    pub fn new(theme: usize, objective: Objective, target: i16) -> Result<Self> {
        let ground = *DEFAULT_TILES.get(theme).ok_or("Unknown trial theme")?;
        let mut mission = Self {
            title: "New mission".into(),
            author: String::new(),
            password: String::new(),
            theme,
            objective,
            stealth: false,
            map: vec![
                Cell {
                    ground,
                    ornament: 255
                };
                1024
            ],
            placements: Vec::new(),
            records: Vec::new(),
            preserved_checksum_bit: false,
        };
        mission.reset_placements(target);
        Ok(mission)
    }
    pub fn reset_placements(&mut self, target: i16) {
        self.placements.clear();
        self.placements.push(Placement {
            kind: 0,
            point: Point { x: 16, z: 18 },
            route: vec![],
        });
        let kind = match self.objective {
            Objective::ReachLord | Objective::ProtectPrincess => 6,
            Objective::Assassinate | Objective::EliminateEnemies => target,
            Objective::FindHerb => 170,
        };
        self.placements.push(Placement {
            kind,
            point: Point { x: 16, z: 14 },
            route: vec![],
        });
        if self.objective == Objective::ProtectPrincess {
            self.placements.push(Placement {
                kind: 5,
                point: Point { x: 16, z: 16 },
                route: vec![],
            });
        }
    }
    pub fn cell(&self, x: i16, z: i16) -> Option<&Cell> {
        Point { x, z }
            .valid()
            .then(|| self.map.get(z as usize * 32 + x as usize))
            .flatten()
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let bytes = if bytes.len() == 8192 {
            if bytes.starts_with(b"SC") {
                &bytes[512..512 + SIZE]
            } else {
                &bytes[..SIZE]
            }
        } else {
            bytes
        };
        if bytes.len() != SIZE {
            return Err(format!("Expected a {SIZE}-byte .msn mission"));
        }
        let flags = u16_le(bytes, 4)?;
        let objective = *Objective::ALL
            .get(usize::from(flags & 7))
            .ok_or("Unknown trial objective")?;
        let theme = u16_le(bytes, 6)? as usize;
        if theme >= THEMES.len() || flags & !15 != 0 {
            return Err("Invalid trial settings".into());
        }
        let mut records = Vec::new();
        for row in bytes[0x4c..0xec].chunks_exact(16) {
            let ticks = u32_le(row, 0)? as i32;
            if ticks >= 0 {
                records.push(Record {
                    ticks,
                    name: read_text(&row[4..14]),
                });
            }
        }
        let map = bytes[0xec..0x8ec]
            .chunks_exact(2)
            .map(|row| Cell {
                ground: row[0],
                ornament: row[1],
            })
            .collect();
        let mut placements = Vec::new();
        for row in bytes[0x8ec..].chunks_exact(520) {
            let kind = i16_le(row, 0)?;
            if kind == -1 {
                continue;
            }
            let point = Point {
                x: i16_le(row, 2)?,
                z: i16_le(row, 4)?,
            };
            let last = i16_le(row, 6)?;
            if !point.valid() || !(0..64).contains(&last) {
                return Err("Invalid trial placement".into());
            }
            let mut route = Vec::new();
            if last > 0 {
                for p in row[8..8 + (last as usize + 1) * 8].chunks_exact(8) {
                    let waypoint = Waypoint {
                        point: Point {
                            x: i16_le(p, 0)?,
                            z: i16_le(p, 2)?,
                        },
                        facing: i16_le(p, 4)?,
                        wait: i16_le(p, 6)?,
                    };
                    if !waypoint.point.valid()
                        || !(0..4).contains(&waypoint.facing)
                        || !(0..=9).contains(&waypoint.wait)
                    {
                        return Err("Invalid trial patrol waypoint".into());
                    }
                    route.push(waypoint);
                }
            }
            placements.push(Placement { kind, point, route });
        }
        let mission = Self {
            title: read_text(&bytes[8..40]),
            author: read_text(&bytes[40..58]),
            password: read_text(&bytes[58..76]),
            objective,
            theme,
            stealth: flags & 8 != 0,
            records,
            map,
            placements,
            preserved_checksum_bit: u32_le(bytes, 0)? & 0x8000000 != 0,
        };
        mission.validate()?;
        Ok(mission)
    }
    pub fn validate(&self) -> Result<()> {
        if self.map.len() != 1024
            || self.theme >= 12
            || !(self.objective.reserved()..=10).contains(&self.placements.len())
            || self.placements[0].kind != 0
            || self.records.len() > 10
        {
            return Err("Invalid trial mission layout".into());
        }
        for placement in &self.placements {
            if !placement.point.valid()
                || placement.route.len() > 64
                || placement.route.len() == 1
                || placement.route.iter().any(|p| {
                    !p.point.valid() || !(0..4).contains(&p.facing) || !(0..=9).contains(&p.wait)
                })
            {
                return Err("Invalid trial placement or patrol".into());
            }
        }
        Ok(())
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = vec![0; SIZE];
        let objective = Objective::ALL
            .iter()
            .position(|v| *v == self.objective)
            .unwrap() as u16;
        bytes[4..6].copy_from_slice(&(objective | (u16::from(self.stealth) * 8)).to_le_bytes());
        bytes[6..8].copy_from_slice(&(self.theme as u16).to_le_bytes());
        write_text(&mut bytes[8..40], &self.title)?;
        write_text(&mut bytes[40..58], &self.author)?;
        write_text(&mut bytes[58..76], &self.password)?;
        bytes[0x4c..0xec].fill(255);
        for (record, row) in self
            .records
            .iter()
            .zip(bytes[0x4c..0xec].chunks_exact_mut(16))
        {
            row[..4].copy_from_slice(&record.ticks.to_le_bytes());
            write_text(&mut row[4..14], &record.name)?;
        }
        for (cell, row) in self.map.iter().zip(bytes[0xec..0x8ec].chunks_exact_mut(2)) {
            row.copy_from_slice(&[cell.ground, cell.ornament]);
        }
        for (i, row) in bytes[0x8ec..].chunks_exact_mut(520).enumerate() {
            let Some(p) = self.placements.get(i) else {
                row[..2].copy_from_slice(&(-1i16).to_le_bytes());
                continue;
            };
            for (at, value) in [
                (0, p.kind),
                (2, p.point.x),
                (4, p.point.z),
                (6, p.route.len().saturating_sub(1) as i16),
            ] {
                row[at..at + 2].copy_from_slice(&value.to_le_bytes());
            }
            for (p, bytes) in p.route.iter().zip(row[8..].chunks_exact_mut(8)) {
                for (value, out) in [p.point.x, p.point.z, p.facing, p.wait]
                    .into_iter()
                    .zip(bytes.chunks_exact_mut(2))
                {
                    out.copy_from_slice(&value.to_le_bytes());
                }
            }
        }
        let checksum = checksum(&bytes) | (u32::from(self.preserved_checksum_bit) << 27);
        bytes[..4].copy_from_slice(&checksum.to_le_bytes());
        Ok(bytes)
    }
    pub fn record(&mut self, ticks: i32, name: String) -> Option<usize> {
        let index = self
            .records
            .iter()
            .position(|r| ticks < r.ticks)
            .unwrap_or(self.records.len());
        if index >= 10 {
            return None;
        }
        self.records.insert(index, Record { ticks, name });
        self.records.truncate(10);
        Some(index)
    }
}
fn checksum(bytes: &[u8]) -> u32 {
    let sum =
        |range: std::ops::Range<usize>| bytes[range].iter().map(|v| u32::from(*v)).sum::<u32>();
    ((sum(4..0x4c) * 2048 + sum(0x8ec..SIZE) * 64 + sum(0xec..0x8ec)) & 0x07ff_ffff).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mission_roundtrip_and_checksum_exclude_records() {
        let mut m = Mission::new(7, Objective::ProtectPrincess, 133).unwrap();
        m.title = "試練".into();
        m.stealth = true;
        m.map[20] = Cell {
            ground: 193,
            ornament: 255,
        };
        m.placements[2].route = vec![
            Waypoint {
                point: m.placements[2].point,
                ..Default::default()
            },
            Waypoint {
                point: Point { x: 17, z: 16 },
                facing: 2,
                wait: 3,
            },
        ];
        let bytes = m.encode().unwrap();
        assert_eq!(Mission::decode(&bytes).unwrap(), m);
        m.record(300, "Player".into());
        assert_eq!(&bytes[..4], &m.encode().unwrap()[..4]);
        m.placements[0].point.x = 32;
        assert!(m.encode().is_err());
        for n in [0, 20, SIZE - 1, SIZE + 1] {
            assert!(Mission::decode(&vec![0; n]).is_err());
        }
    }
}
