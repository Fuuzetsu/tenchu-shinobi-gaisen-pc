mod ambience;
mod envelope;
mod thunder;
pub use ambience::Ambience;
pub use thunder::{ThunderCue, ThunderStyle};

mod tenchu2;
mod voice;
use std::sync::Arc;
use tenchu_assets::audio::{bank::Bank, stream::Clip};
use voice::Voice;

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
    thunder: [Option<thunder::Thunder>; 2],
    ambience: Option<ambience::Bed>,
    soundscape: Option<tenchu2::Soundscape>,
    rate: u32,
    envelope_clock: u64,
    pause_envelope_clock: u64,
    pub paused: bool,
    pub music_while_paused: bool,
    pub volume: f32,
    pub music_volume: f32,
    pub effects_volume: f32,
    pub stereo: bool,
}

impl Mixer {
    pub fn new(rate: u32) -> Self {
        assert!(rate > 0);
        Self {
            bank: None,
            voices: std::array::from_fn(|_| None),
            next_voice: 0,
            music: None,
            thunder: [None, None],
            ambience: None,
            soundscape: None,
            rate,
            envelope_clock: 0,
            pause_envelope_clock: 0,
            paused: false,
            music_while_paused: false,
            volume: 0.5,
            music_volume: 1.0,
            effects_volume: 1.0,
            stereo: true,
        }
    }

    pub fn set_bank(&mut self, bank: Arc<Bank>) {
        self.bank = Some(bank);
        self.clear_effects();
    }

    pub fn clear_effects(&mut self) {
        self.voices = std::array::from_fn(|_| None);
        self.thunder = [None, None];
        self.ambience = None;
        self.soundscape = None;
    }

    pub fn tenchu2_ambience(&mut self, ambience: tenchu_assets::tenchu2::music::Ambience) {
        self.soundscape = Some(tenchu2::Soundscape::new(ambience, self.rate));
    }

    pub fn ambience(&mut self, ambience: Ambience) {
        if ambience.rain > 0.0 || ambience.wind > 0.0 || self.ambience.is_some() {
            self.ambience
                .get_or_insert_with(|| ambience::Bed::new(self.rate))
                .set(ambience);
        }
    }

    pub fn thunder(&mut self, cue: Option<ThunderCue>) {
        if let Some(cue) = cue.filter(|c| c.strength > 0.0) {
            self.thunder[1] = if cue.style == ThunderStyle::Simple {
                None
            } else {
                self.thunder[0].take()
            };
            self.thunder[0] = Some(thunder::Thunder::new(cue, self.rate));
        } else {
            self.thunder = [None, None];
        }
    }

    pub fn play(&mut self, id: i16, volume: u8, pan: u8) {
        self.play_note(id, volume, pan, 36);
    }

    pub fn play_ui(&mut self, id: i16, volume: u8, pan: u8) {
        self.play_note_inner(id, volume, pan, 36, true);
    }

    pub fn play_note(&mut self, id: i16, volume: u8, pan: u8, note: u8) {
        self.play_note_inner(id, volume, pan, note, false);
    }

    pub fn play_at(&mut self, slot: usize, id: i16, volume: [u8; 2], note: u8) {
        if let Some(slot) = self.voices.get_mut(slot)
            && let Some(voice) = self
                .bank
                .as_ref()
                .and_then(|bank| Voice::stereo(bank, id, volume, note, self.rate))
        {
            *slot = Some(voice);
        }
    }

    fn play_note_inner(&mut self, id: i16, volume: u8, pan: u8, note: u8, during_pause: bool) {
        self.next_voice = (self.next_voice + 1) % self.voices.len();
        if let Some(voice) = self
            .bank
            .as_ref()
            .and_then(|bank| Voice::new(bank, id, volume, pan, note, self.rate, during_pause))
        {
            self.voices[self.next_voice] = Some(voice);
        }
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

    pub fn seek_music(&mut self, seconds: f64) {
        if let Some(music) = &mut self.music {
            let position = seconds.max(0.0) * f64::from(music.clip.rate);
            music.position = if position.is_finite() { position } else { 0.0 };
        }
    }

    pub fn frame(&mut self) -> [f32; 2] {
        let clock = if self.paused {
            &mut self.pause_envelope_clock
        } else {
            &mut self.envelope_clock
        };
        *clock += 44100;
        let ticks = *clock / u64::from(self.rate);
        *clock %= u64::from(self.rate);
        let mut output = [0.0; 2];
        if let Some(bank) = &self.bank {
            for slot in &mut self.voices {
                let Some(voice) = slot else {
                    continue;
                };
                if self.paused && !voice.during_pause {
                    continue;
                }
                if let Some(frame) = voice.frame(bank, ticks) {
                    for (out, sample) in output.iter_mut().zip(frame) {
                        *out += sample;
                    }
                } else {
                    *slot = None;
                }
            }
        }
        if !self.paused {
            for slot in &mut self.thunder {
                if let Some(thunder) = slot {
                    if thunder.finished() {
                        *slot = None;
                    } else {
                        for (out, sample) in output.iter_mut().zip(thunder.frame()) {
                            *out += sample;
                        }
                    }
                }
            }
        }
        if !self.paused
            && let Some(rain) = &mut self.ambience
        {
            if rain.finished() {
                self.ambience = None;
            } else {
                for (out, sample) in output.iter_mut().zip(rain.frame()) {
                    *out += sample;
                }
            }
        }
        if !self.paused
            && let Some(soundscape) = &mut self.soundscape
        {
            for (out, sample) in output.iter_mut().zip(soundscape.frame(ticks)) {
                *out += sample;
            }
        }
        output = output.map(|v| v * self.effects_volume);
        if (!self.paused || self.music_while_paused)
            && let Some(music) = &mut self.music
        {
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
                    *out += (a + (b - a) * fraction) / 32768.0 * self.music_volume;
                }
                music.position += music.step;
            } else {
                self.music = None;
            }
        }
        if !self.stereo {
            output = [(output[0] + output[1]) * 0.5; 2];
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
