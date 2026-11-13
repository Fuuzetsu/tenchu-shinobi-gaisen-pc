use std::{io::Cursor, sync::Arc};
use symphonia::core::{
    codecs::audio::AudioDecoderOptions,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
};
use tenchu_assets::{Result, audio::stream::Clip};

pub(super) fn decode(bytes: Arc<[u8]>) -> Result<Clip> {
    let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            stream,
            FormatOptions::default(),
            Default::default(),
        )
        .map_err(|e| format!("Open audio (WAV, Ogg/Vorbis, MP3 or FLAC): {e}"))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("No audio track")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("No audio codec")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| e.to_string())?;
    let track_id = track.id;
    let mut clip = Clip {
        rate: 0,
        frames: vec![],
    };
    let mut samples = Vec::<i16>::new();
    while let Some(packet) = format
        .next_packet()
        .map_err(|e| format!("Read audio: {e}"))?
    {
        if packet.track_id != track_id {
            continue;
        }
        let buffer = decoder
            .decode(&packet)
            .map_err(|e| format!("Decode audio: {e}"))?;
        let rate = buffer.spec().rate();
        let channels = buffer.spec().channels().count();
        if !(8000..=96000).contains(&rate) || !(1..=2).contains(&channels) {
            return Err("Use mono or stereo audio at 8–96 kHz".into());
        }
        if clip.rate != 0 && clip.rate != rate {
            return Err("Audio sample rate changes within the file".into());
        }
        clip.rate = rate;
        let count = buffer.samples_interleaved();
        if count > 2 * 1024 * 1024 || clip.frames.len() + count / channels > 16 * 1024 * 1024 {
            return Err("Decoded audio exceeds 64 MiB; shorten or resample the track".into());
        }
        samples.resize(count, 0);
        buffer.copy_to_slice_interleaved(&mut samples);
        clip.frames.extend(
            samples
                .chunks_exact(channels)
                .map(|s| [s[0], s[channels - 1]]),
        );
    }
    if clip.frames.is_empty() {
        return Err("Audio contains no samples".into());
    }
    Ok(clip)
}
