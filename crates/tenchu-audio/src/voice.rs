use super::envelope::Envelope;
use tenchu_assets::audio::bank::Bank;

pub(super) struct Voice {
    sample: usize,
    pub position: f64,
    step: f64,
    volume: [f32; 2],
    envelope: Envelope,
    pub during_pause: bool,
}

impl Voice {
    pub fn new(
        bank: &Bank,
        id: i16,
        volume: u8,
        pan: u8,
        note: u8,
        rate: u32,
        during_pause: bool,
    ) -> Option<Self> {
        let Some(Some(program)) = bank.programs.get((id as u16 >> 4) as usize) else {
            return None;
        };
        let tone = program.tones.get((id & 15) as usize)?;
        let sample = bank.samples.get(tone.sample)?;
        if sample.frames.is_empty() {
            return None;
        }
        let volume = f32::from(volume) / 127.0 * f32::from(bank.volume) / 127.0
            * f32::from(program.volume)
            / 127.0
            * f32::from(tone.volume)
            / 127.0;
        let pan = f32::from(pan.min(127));
        let pitch =
            (f64::from(note) - f64::from(tone.center) - f64::from(tone.fine) / 128.0) / 12.0;
        Some(Self {
            sample: tone.sample,
            position: 0.0,
            step: 44100.0 * pitch.exp2() / f64::from(rate),
            volume: [
                volume * ((127.0 - pan) / 63.0).min(1.0),
                volume * (pan / 64.0).min(1.0),
            ],
            envelope: Envelope::new(tone.adsr),
            during_pause,
        })
    }

    pub fn frame(&mut self, bank: &Bank, ticks: u64) -> Option<[f32; 2]> {
        let sample = &bank.samples[self.sample];
        let length = sample.frames.len();
        if self.position >= length as f64 {
            let start = sample.loop_start.filter(|&start| start < length)?;
            self.position =
                start as f64 + (self.position - length as f64) % (length - start) as f64;
        }
        for _ in 0..ticks {
            self.envelope.tick();
        }
        if self.envelope.finished() {
            return None;
        }
        let index = self.position as usize;
        let next = if index + 1 < length {
            index + 1
        } else {
            sample.loop_start.unwrap_or(index).min(length - 1)
        };
        let fraction = self.position.fract() as f32;
        let a = f32::from(sample.frames[index]);
        let b = f32::from(sample.frames[next]);
        let value = (a + (b - a) * fraction) * self.envelope.volume() / 32768.0;
        self.position += self.step;
        Some(self.volume.map(|v| value * v))
    }
}
