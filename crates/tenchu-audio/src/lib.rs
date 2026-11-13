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

impl Music {
    fn new(clip: Arc<Clip>, rate: u32, repeat: bool) -> Option<Self> {
        (!clip.frames.is_empty()).then(|| Self {
            step: f64::from(clip.rate) / f64::from(rate),
            clip,
            position: 0.0,
            repeat,
        })
    }

    fn frame(&mut self) -> Option<[f32; 2]> {
        let length = self.clip.frames.len();
        if self.position >= length as f64 && !self.repeat {
            return None;
        }
        self.position %= length as f64;
        let index = self.position as usize;
        let next = if index + 1 < length {
            index + 1
        } else if self.repeat {
            0
        } else {
            index
        };
        let fraction = self.position.fract() as f32;
        let frame = std::array::from_fn(|channel| {
            let a = f32::from(self.clip.frames[index][channel]);
            let b = f32::from(self.clip.frames[next][channel]);
            (a + (b - a) * fraction) / 32768.0
        });
        self.position += self.step;
        Some(frame)
    }
}

enum Preview {
    Music(Music),
    Effect(Voice),
}

pub struct Mixer {
    bank: Option<Arc<Bank>>,
    voices: [Option<Voice>; 24],
    next_voice: usize,
    music: Option<Music>,
    preview: Option<Preview>,
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
            preview: None,
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
        self.preview = None;
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

    pub fn play_from(&mut self, bank: &Arc<Bank>, id: i16, volume: u8, pan: u8) {
        self.next_voice = (self.next_voice + 1) % self.voices.len();
        if let Some(voice) = Voice::new(bank, id, volume, pan, 36, self.rate, false) {
            self.voices[self.next_voice] = Some(voice);
        }
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

    pub fn play_at_from(
        &mut self,
        bank: &Arc<Bank>,
        slot: usize,
        id: i16,
        volume: [u8; 2],
        note: u8,
    ) {
        if let Some(slot) = self.voices.get_mut(slot)
            && let Some(voice) = Voice::stereo(bank, id, volume, note, self.rate)
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
        self.music = clip.and_then(|clip| Music::new(clip, self.rate, repeat));
    }

    pub fn preview(&mut self, clip: Option<Arc<Clip>>) {
        self.preview = clip
            .and_then(|clip| Music::new(clip, self.rate, false))
            .map(Preview::Music);
    }

    pub fn preview_effect(&mut self, id: i16) {
        self.preview = self
            .bank
            .as_ref()
            .and_then(|bank| Voice::new(bank, id, 127, 64, 36, self.rate, true))
            .map(Preview::Effect);
    }

    pub fn preview_from(&mut self, bank: &Arc<Bank>, id: i16, volume: u8, pan: u8) {
        self.preview = Voice::new(bank, id, volume, pan, 36, self.rate, true).map(Preview::Effect);
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
        for slot in &mut self.voices {
            let Some(voice) = slot else {
                continue;
            };
            if self.paused && !voice.during_pause {
                continue;
            }
            if let Some(frame) = voice.frame(ticks) {
                for (out, sample) in output.iter_mut().zip(frame) {
                    *out += sample;
                }
            } else {
                *slot = None;
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
        let (preview, mute_music) = match &mut self.preview {
            Some(Preview::Music(music)) => (
                music.frame().map(|f| f.map(|s| s * self.music_volume)),
                true,
            ),
            Some(Preview::Effect(voice)) => (
                voice
                    .frame(ticks)
                    .map(|f| f.map(|s| s * self.effects_volume)),
                false,
            ),
            None => (None, false),
        };
        if preview.is_none() {
            self.preview = None;
        }
        if (!self.paused || self.music_while_paused)
            && let Some(music) = &mut self.music
        {
            if let Some(frame) = music.frame() {
                if !mute_music || preview.is_none() {
                    for (out, sample) in output.iter_mut().zip(frame) {
                        *out += sample * self.music_volume;
                    }
                }
            } else {
                self.music = None;
            }
        }
        if let Some(frame) = preview {
            for (out, sample) in output.iter_mut().zip(frame) {
                *out += sample;
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
