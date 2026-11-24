use super::*;
use std::sync::Arc;
use tenchu_assets::audio::stream::Clip;
use tenchu_assets::tenchu2::Edition;

fn european_disc() -> Option<Disc<crate::source::DiscSource>> {
    let (_, path) = crate::tenchu2::menu::known_discs()
        .into_iter()
        .find(|(edition, _)| *edition == Edition::Europe)?;
    let disc = crate::source::open_disc(&path).ok()?;
    (Edition::detect(&disc).ok()? == Edition::Europe).then_some(disc)
}

impl Story {
    pub(in crate::tenchu2::native) fn load_optional_execution(
        disc: &Disc<impl ReadAt>,
        sequence: usize,
        event: u16,
        seed: u32,
        enabled: bool,
    ) -> Result<Self> {
        if enabled
            && event == 113
            && let Some(europe) = european_disc()
        {
            return Self::european_execution(disc, &europe, sequence, seed);
        }
        Self::load(disc, sequence, event, seed)
    }

    fn european_execution(
        japan: &Disc<impl ReadAt>,
        europe: &Disc<impl ReadAt>,
        sequence: usize,
        seed: u32,
    ) -> Result<Self> {
        let resources = Resources::load(europe, 113)?;
        if resources.edition != Edition::Europe || resources.area != 8 {
            return Err("Invalid European Toda execution scene".into());
        }
        Self::new(
            europe,
            sequence,
            113,
            seed,
            resources,
            Level::european_execution(europe)?,
            // These particle tables are byte-identical in both retail executables.
            Rules::load(japan)?,
        )
    }

    pub(in crate::tenchu2::native) fn load_edition(
        disc: &Disc<impl ReadAt>,
        sequence: usize,
        event: u16,
        seed: u32,
        edition: Edition,
    ) -> Result<Self> {
        match edition {
            Edition::Japan => Self::load(disc, sequence, event, seed),
            Edition::Europe if event == 113 => Self::european_execution(
                disc,
                &european_disc().ok_or(
                    "Reconnect the European Tenchu 2 disc to restore this execution scene",
                )?,
                sequence,
                seed,
            ),
            _ => Err("Invalid saved European story identity".into()),
        }
    }

    pub(in crate::tenchu2::native) fn audio(
        &self,
        disc: &Disc<impl ReadAt>,
    ) -> Result<Option<Arc<Clip>>> {
        let Some(track) = &self.resources.audio else {
            return Ok(None);
        };
        let clip = match self.resources.edition {
            Edition::Japan => Clip::load(disc, track)?,
            Edition::Europe => Clip::load(
                &european_disc()
                    .ok_or("Reconnect the European Tenchu 2 disc for the execution scene audio")?,
                track,
            )?,
        };
        Ok(Some(Arc::new(clip)))
    }
}

#[cfg(test)]
mod tests;
