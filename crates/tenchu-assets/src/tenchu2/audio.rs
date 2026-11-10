use super::{Edition, volume::Volume};
use crate::{ReadAt, Result, audio::bank::Bank, disc::Disc, executable::Executable, i16_le};

pub struct Audio {
    pub bank: Bank,
    pub interval: i16,
}

pub fn menu(disc: &Disc<impl ReadAt>) -> Result<Bank> {
    Bank::parse(&Volume::open(disc)?.read(r"o:\cdimage\menu\sound", "menu.vab")?)
}

pub fn interval(disc: &Disc<impl ReadAt>, map: usize) -> Result<i16> {
    if map >= 24 {
        return Err("Invalid Tenchu 2 ambience map".into());
    }
    let (edition, bytes) = super::executable(disc)?;
    let exe = Executable::new(&bytes)?;
    i16_le(
        exe.bytes(edition.addresses().ambience_intervals + map as u32 * 2, 2)?,
        0,
    )
}

impl Audio {
    pub fn load(disc: &Disc<impl ReadAt>, hero: usize, map: usize) -> Result<Self> {
        let (edition, bytes) = super::executable(disc)?;
        if edition != Edition::Japan || hero >= 3 || map >= 24 {
            return Err("Native Tenchu 2 audio requires a Japanese campaign cast".into());
        }
        let exe = Executable::new(&bytes)?;
        let prefix = exe.bytes(0x8008e4d0 + hero as u32, 1)?[0];
        if !prefix.is_ascii_alphabetic() {
            return Err("Invalid Tenchu 2 sound bank prefix".into());
        }
        let name = format!("{}_E_{map:02}.vab", char::from(prefix));
        Ok(Self {
            bank: Bank::parse(&Volume::open(disc)?.read(r"o:\cdimage\game\sound", &name)?)?,
            interval: i16_le(
                exe.bytes(edition.addresses().ambience_intervals + map as u32 * 2, 2)?,
                0,
            )?,
        })
    }
}

pub fn voices(
    disc: &Disc<impl ReadAt>,
    registry: &super::registry::Registry,
) -> Result<std::collections::BTreeMap<i16, std::sync::Arc<crate::audio::voice::VoiceSet>>> {
    voices_for(
        disc,
        registry,
        &registry.actors.iter().map(|a| a.kind).collect::<Vec<_>>(),
    )
}

pub fn voices_for(
    disc: &Disc<impl ReadAt>,
    registry: &super::registry::Registry,
    kinds: &[i16],
) -> Result<std::collections::BTreeMap<i16, std::sync::Arc<crate::audio::voice::VoiceSet>>> {
    use crate::audio::voice::{Dialect, VoiceSet};
    use std::{collections::BTreeMap, sync::Arc};
    let volume = Volume::open(disc)?;
    let mut groups: BTreeMap<(usize, usize), Vec<(i16, usize)>> = BTreeMap::new();
    for actor in registry.actors.iter().filter(|a| kinds.contains(&a.kind)) {
        let source = if actor.kind < 3 {
            Some((actor.kind as usize, 0, 4))
        } else {
            (0..24).find_map(|map| {
                (0..3).find_map(|hero| {
                    registry
                        .cast(hero, map)?
                        .iter()
                        .position(|&kind| kind == actor.kind)
                        .map(|slot| (hero, map, slot + 5))
                })
            })
        };
        if let Some((hero, map, program)) = source {
            groups
                .entry((hero, map))
                .or_default()
                .push((actor.kind, program));
        }
    }
    let mut voices = BTreeMap::new();
    for ((hero, map), actors) in groups {
        let name = format!("{}_E_{map:02}.vab", ['R', 'A', 'T'][hero]);
        let bank = Bank::parse(&volume.read(r"o:\cdimage\game\sound", &name)?)?;
        for (kind, program) in actors {
            voices.insert(
                kind,
                Arc::new(VoiceSet::extract(&bank, program, Dialect::Tenchu2)?),
            );
        }
    }
    Ok(voices)
}
