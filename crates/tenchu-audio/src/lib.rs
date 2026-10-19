mod envelope;

use envelope::Envelope;
use std::sync::Arc;
use tenchu_assets::audio::{bank::Bank, stream::Clip};

struct Voice {
    sample: usize,
    position: f64,
    step: f64,
    volume: [f32; 2],
    envelope: Envelope,
}

struct Music {
    clip: Arc<Clip>,
    position: f64,
    step: f64,
    repeat: bool,
}

pub struct Mixer {
    bank: Option<Arc<Bank>>,
    voices: [Option<Voice>; 24],
    next_voice: usize,
    music: Option<Music>,
    rate: u32,
    envelope_clock: u64,
    pub paused: bool,
    pub volume: f32,
}

impl Mixer {
    pub fn new(rate: u32) -> Self {
        assert!(rate > 0);
        Self {
            bank: None,
            voices: std::array::from_fn(|_| None),
            next_voice: 0,
            music: None,
            rate,
            envelope_clock: 0,
            paused: false,
            volume: 0.5,
        }
    }

    pub fn set_bank(&mut self, bank: Arc<Bank>) {
        self.bank = Some(bank);
        self.clear_effects();
    }

    pub fn clear_effects(&mut self) {
        self.voices = std::array::from_fn(|_| None);
    }

    pub fn play(&mut self, id: i16, volume: u8, pan: u8) {
        self.next_voice = (self.next_voice + 1) % self.voices.len();
        let Some(bank) = &self.bank else {
            return;
        };
        let Some(Some(program)) = bank.programs.get((id as u16 >> 4) as usize) else {
            return;
        };
        let Some(tone) = program.tones.get((id & 15) as usize) else {
            return;
        };
        let Some(sample) = bank.samples.get(tone.sample) else {
            return;
        };
        if sample.frames.is_empty() {
            return;
        }
        let volume = f32::from(volume) / 127.0 * f32::from(bank.volume) / 127.0
            * f32::from(program.volume)
            / 127.0
            * f32::from(tone.volume)
            / 127.0;
        let pan = f32::from(pan.min(127));
        let pitch = (36.0 - f64::from(tone.center) - f64::from(tone.fine) / 128.0) / 12.0;
        self.voices[self.next_voice] = Some(Voice {
            sample: tone.sample,
            position: 0.0,
            step: 44100.0 * pitch.exp2() / f64::from(self.rate),
            volume: [
                volume * ((127.0 - pan) / 63.0).min(1.0),
                volume * (pan / 64.0).min(1.0),
            ],
            envelope: Envelope::new(tone.adsr),
        });
    }

    pub fn music(&mut self, clip: Option<Arc<Clip>>, repeat: bool) {
        self.music = clip
            .filter(|clip| !clip.frames.is_empty())
            .map(|clip| Music {
                step: f64::from(clip.rate) / f64::from(self.rate),
                clip,
                position: 0.0,
                repeat,
            });
    }

    pub fn frame(&mut self) -> [f32; 2] {
        if self.paused {
            return [0.0; 2];
        }
        self.envelope_clock += 44100;
        let ticks = self.envelope_clock / u64::from(self.rate);
        self.envelope_clock %= u64::from(self.rate);
        let mut output = [0.0; 2];
        if let Some(bank) = &self.bank {
            for slot in &mut self.voices {
                let Some(voice) = slot else {
                    continue;
                };
                let sample = &bank.samples[voice.sample];
                let length = sample.frames.len();
                if voice.position >= length as f64 {
                    if let Some(start) = sample.loop_start.filter(|&start| start < length) {
                        voice.position = start as f64
                            + (voice.position - length as f64) % (length - start) as f64;
                    } else {
                        *slot = None;
                        continue;
                    }
                }
                for _ in 0..ticks {
                    voice.envelope.tick();
                }
                if voice.envelope.finished() {
                    *slot = None;
                    continue;
                }
                let index = voice.position as usize;
                let next = if index + 1 < length {
                    index + 1
                } else {
                    sample.loop_start.unwrap_or(index).min(length - 1)
                };
                let fraction = voice.position.fract() as f32;
                let a = f32::from(sample.frames[index]);
                let b = f32::from(sample.frames[next]);
                let value = (a + (b - a) * fraction) * voice.envelope.volume() / 32768.0;
                for (out, volume) in output.iter_mut().zip(voice.volume) {
                    *out += value * volume;
                }
                voice.position += voice.step;
            }
        }
        if let Some(music) = &mut self.music {
            let length = music.clip.frames.len();
            if music.position < length as f64 || music.repeat {
                music.position %= length as f64;
                let index = music.position as usize;
                let next = if index + 1 < length {
                    index + 1
                } else if music.repeat {
                    0
                } else {
                    index
                };
                let fraction = music.position.fract() as f32;
                for (channel, out) in output.iter_mut().enumerate() {
                    let a = f32::from(music.clip.frames[index][channel]);
                    let b = f32::from(music.clip.frames[next][channel]);
                    *out += (a + (b - a) * fraction) / 32768.0;
                }
                music.position += music.step;
            } else {
                self.music = None;
            }
        }
        output.map(|v| (v * self.volume).clamp(-1.0, 1.0))
    }

    pub fn render(&mut self, output: &mut [[f32; 2]]) {
        for frame in output {
            *frame = self.frame();
        }
    }
}

#[cfg(test)]
mod tests;
