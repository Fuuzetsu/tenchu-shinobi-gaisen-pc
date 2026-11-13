use cpal::{
    FromSample, SampleFormat, SizedSample, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::sync::{
    Arc,
    mpsc::{self, Receiver, Sender},
};
use tenchu_assets::{
    Result,
    audio::{bank::Bank, stream::Clip},
};
use tenchu_audio::Mixer;
use tenchu_core::audio::Sound;

#[cfg(target_os = "linux")]
mod pulse;

pub enum Command {
    Bank(Arc<Bank>),
    Effect(tenchu_assets::audio::voice::Sourced<Sound>),
    Tenchu2Effect(tenchu_assets::audio::voice::Sourced<tenchu_core::tenchu2::audio::Sound>),
    UiEffect(i16),
    Music(Option<Arc<Clip>>, bool),
    Preview(Option<Arc<Clip>>),
    PreviewEffect(i16),
    SeekMusic(f64),
    Clear,
    Pause {
        paused: bool,
        music_while_paused: bool,
    },
    Volume(u16),
    Balance(u8, u8, bool),
    Voice(i16),
    Thunder(Option<tenchu_audio::ThunderCue>),
    Ambience(tenchu_audio::Ambience),
    Tenchu2Ambience(tenchu_assets::tenchu2::music::Ambience),
}

pub struct Output {
    _stream: Stream,
    commands: Sender<Command>,
}

enum Stream {
    #[cfg(test)]
    Recorded,
    Device {
        _stream: cpal::Stream,
    },
    #[cfg(target_os = "linux")]
    Pulse {
        _stream: pulseaudio::PlaybackStream,
    },
}

impl Output {
    #[cfg(test)]
    pub(crate) fn recording() -> (Self, Receiver<Command>) {
        let (commands, receiver) = mpsc::channel();
        (
            Self {
                _stream: Stream::Recorded,
                commands,
            },
            receiver,
        )
    }

    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        if pulseaudio::socket_path_from_env().is_some() {
            match pulse::open() {
                Ok(output) => return Ok(output),
                Err(error) => eprintln!("Named PulseAudio output: {error}; trying CPAL"),
            }
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let mut stream_config = config.config();
        if let cpal::SupportedBufferSize::Range { min, max } = *config.buffer_size() {
            stream_config.buffer_size = cpal::BufferSize::Fixed(512u32.clamp(min, max));
        }
        let (commands, receiver) = mpsc::channel();
        let stream = match config.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, stream_config, receiver),
            SampleFormat::F64 => build::<f64>(&device, stream_config, receiver),
            SampleFormat::I16 => build::<i16>(&device, stream_config, receiver),
            SampleFormat::I24 => build::<cpal::I24>(&device, stream_config, receiver),
            SampleFormat::I32 => build::<i32>(&device, stream_config, receiver),
            SampleFormat::U16 => build::<u16>(&device, stream_config, receiver),
            SampleFormat::U32 => build::<u32>(&device, stream_config, receiver),
            format => return Err(format!("Unsupported output sample format {format}")),
        }?;
        #[cfg(all(target_arch = "wasm32", not(target_feature = "atomics")))]
        {
            let cpal::platform::StreamInner::WebAudio(output) = stream.as_inner();
            crate::web::configure_game_audio(output.audio_context());
        }
        stream.play().map_err(|e| e.to_string())?;
        println!("Audio output: {}", device.id().map_err(|e| e.to_string())?);
        Ok(Self {
            _stream: Stream::Device { _stream: stream },
            commands,
        })
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

struct Playback {
    mixer: Mixer,
    commands: Receiver<Command>,
    volume: f32,
}

impl Playback {
    fn new(rate: u32, commands: Receiver<Command>) -> Self {
        Self {
            mixer: Mixer::new(rate),
            commands,
            volume: 1.0,
        }
    }

    fn update(&mut self) {
        while let Ok(command) = self.commands.try_recv() {
            match command {
                Command::Bank(bank) => self.mixer.set_bank(bank),
                Command::Effect(event) => {
                    let sound = event.sound;
                    if let Some(bank) = event.bank {
                        self.mixer
                            .play_from(&bank, sound.id, sound.volume, sound.pan);
                    } else {
                        self.mixer.play(sound.id, sound.volume, sound.pan);
                    }
                }
                Command::Tenchu2Effect(event) => {
                    let sound = event.sound;
                    if let Some(bank) = event.bank {
                        self.mixer.play_at_from(
                            &bank,
                            usize::from(sound.slot),
                            i16::from(sound.id),
                            sound.volume,
                            sound.note,
                        );
                    } else {
                        self.mixer.play_at(
                            usize::from(sound.slot),
                            i16::from(sound.id),
                            sound.volume,
                            sound.note,
                        );
                    }
                }
                Command::UiEffect(id) => self.mixer.play_ui(id, 127, 64),
                Command::Music(clip, repeat) => self.mixer.music(clip, repeat),
                Command::Preview(clip) => self.mixer.preview(clip),
                Command::PreviewEffect(id) => self.mixer.preview_effect(id),
                Command::SeekMusic(seconds) => self.mixer.seek_music(seconds),
                Command::Clear => self.mixer.clear_effects(),
                Command::Pause {
                    paused,
                    music_while_paused,
                } => {
                    self.mixer.paused = paused;
                    self.mixer.music_while_paused = music_while_paused;
                }
                Command::Volume(percent) => self.volume = f32::from(percent.min(100)) / 100.0,
                Command::Balance(music, effects, stereo) => {
                    self.mixer.music_volume = f32::from(music) / 96.0;
                    self.mixer.effects_volume = f32::from(effects) / 96.0;
                    self.mixer.stereo = stereo;
                }
                Command::Voice(id) => self.mixer.play_note(id, 127, 64, 48),
                Command::Thunder(cue) => self.mixer.thunder(cue),
                Command::Ambience(ambience) => self.mixer.ambience(ambience),
                Command::Tenchu2Ambience(ambience) => self.mixer.tenchu2_ambience(ambience),
            }
        }
    }
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    commands: Receiver<Command>,
) -> Result<cpal::Stream> {
    let mut playback = Playback::new(config.sample_rate, commands);
    let channels = usize::from(config.channels);
    let mut reported = false;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                playback.update();
                for frame in data.chunks_exact_mut(channels) {
                    let [left, right] = playback.mixer.frame();
                    for (channel, out) in frame.iter_mut().enumerate() {
                        let value = if channels == 1 {
                            (left + right) * 0.5
                        } else {
                            match channel {
                                0 => left,
                                1 => right,
                                _ => 0.0,
                            }
                        };
                        *out = T::from_sample(value * playback.volume);
                    }
                }
            },
            move |error| {
                if !reported {
                    eprintln!("Audio device: {error}");
                    reported = true;
                }
            },
            None,
        )
        .map_err(|e| e.to_string())
}
