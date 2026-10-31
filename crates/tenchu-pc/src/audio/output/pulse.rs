use super::{Command, Output, Playback, Stream};
use pulseaudio::{AsPlaybackSource, protocol};
use std::{sync::mpsc, time::Duration};
use tenchu_assets::Result;

pub(super) fn open() -> Result<Output> {
    // CPAL 0.18 names PulseAudio clients after their PID and pins the default sink.
    // Its underlying client lets the server restore per-game volume and routing.
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(connect());
    });
    let (stream, commands) = receiver
        .recv_timeout(Duration::from_secs(2))
        .map_err(|e| format!("initialization failed: {e}"))??;
    println!("Audio output: Tenchu: Shinobi Gaisen (PulseAudio)");
    Ok(Output {
        _stream: Stream::Pulse { _stream: stream },
        commands,
    })
}

fn connect() -> Result<(pulseaudio::PlaybackStream, mpsc::Sender<Command>)> {
    let client =
        pulseaudio::Client::from_env(c"Tenchu: Shinobi Gaisen").map_err(|e| e.to_string())?;
    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, c"Tenchu: Shinobi Gaisen");
    props.set(protocol::Prop::ApplicationId, c"tenchu-shinobi-gaisen-pc");
    props.set(protocol::Prop::ApplicationIconName, c"applications-games");
    props.set(protocol::Prop::MediaName, c"Game audio");
    let sample_rate = pollster::block_on(client.server_info())
        .map_err(|e| e.to_string())?
        .sample_spec
        .sample_rate;
    let params = protocol::PlaybackStreamParams {
        sample_spec: protocol::SampleSpec {
            format: protocol::SampleFormat::Float32Le,
            channels: 2,
            sample_rate,
        },
        channel_map: protocol::ChannelMap::stereo(),
        buffer_attr: protocol::stream::BufferAttr {
            target_length: 1024 * 8,
            max_length: 1024 * 8,
            minimum_request_length: 512 * 8,
            ..Default::default()
        },
        flags: protocol::stream::StreamFlags {
            adjust_latency: true,
            ..Default::default()
        },
        props,
        ..Default::default()
    };
    let (commands, receiver) = mpsc::channel();
    let mut playback = Playback::new(sample_rate, receiver);
    let source = move |data: &mut [u8]| {
        playback.update();
        for frame in data.chunks_exact_mut(8) {
            let [left, right] = playback.mixer.frame();
            frame[..4].copy_from_slice(&(left * playback.volume).to_le_bytes());
            frame[4..].copy_from_slice(&(right * playback.volume).to_le_bytes());
        }
        data.len() / 8 * 8
    };
    let stream =
        pollster::block_on(client.create_playback_stream(params, source.as_playback_source()))
            .map_err(|e| e.to_string())?;
    Ok((stream, commands))
}
