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

pub enum Command {
    Bank(Arc<Bank>),
    Effect(Sound),
    Music(Option<Arc<Clip>>, bool),
    Clear,
    Pause(bool),
    Volume(u16),
    Balance(u8, u8, bool),
    Voice(i16),
}

pub struct Output {
    _stream: cpal::Stream,
    commands: Sender<Command>,
}

impl Output {
    pub fn new() -> Result<Self> {
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
        stream.play().map_err(|e| e.to_string())?;
        println!("Audio output: {}", device.id().map_err(|e| e.to_string())?);
        Ok(Self {
            _stream: stream,
            commands,
        })
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    commands: Receiver<Command>,
) -> Result<cpal::Stream> {
    let mut mixer = Mixer::new(config.sample_rate);
    let channels = usize::from(config.channels);
    let mut reported = false;
    let mut volume = 1.0;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                while let Ok(command) = commands.try_recv() {
                    match command {
                        Command::Bank(bank) => mixer.set_bank(bank),
                        Command::Effect(sound) => mixer.play(sound.id, sound.volume, sound.pan),
                        Command::Music(clip, repeat) => mixer.music(clip, repeat),
                        Command::Clear => mixer.clear_effects(),
                        Command::Pause(paused) => mixer.paused = paused,
                        Command::Volume(percent) => volume = f32::from(percent.min(100)) / 100.0,
                        Command::Balance(music, effects, stereo) => {
                            mixer.music_volume = f32::from(music) / 96.0;
                            mixer.effects_volume = f32::from(effects) / 96.0;
                            mixer.stereo = stereo;
                        }
                        Command::Voice(id) => mixer.play_note(id, 127, 64, 48),
                    }
                }
                for frame in data.chunks_exact_mut(channels) {
                    let [left, right] = mixer.frame();
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
                        *out = T::from_sample(value * volume);
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
