use super::*;
use crate::tenchu2::native::campaign::Content;

fn start(sequence: usize, prologue: bool) -> Playback<crate::source::DiscSource> {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let intro = prologue.then(|| super::super::film::Prologue::load(&disc).unwrap());
    let mut playback = Playback::load(disc, sequence, 2, 17, Some([0; 21]), None).unwrap();
    if let Some(intro) = intro {
        playback.content = Content::Prologue(Box::new(intro));
    }
    playback
}

fn input(speed: u16) -> Input {
    Input {
        buttons: 0,
        speed,
        fast_forward: false,
        paused: false,
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; compare all cinematic paths at normal and accelerated speed, including source RNG and the next scene's initial state"]
fn cutscene_acceleration_preserves_ticks_rng_and_content_boundaries() {
    for (sequence, prologue, speed) in [
        (0, true, 16),
        (0, false, 0),
        (15, false, 16),
        (34, false, 0),
        (8, false, 16),
    ] {
        let mut normal = start(sequence, prologue);
        let generation = normal.generation;
        assert!(normal.content.cutscene());
        for _ in 0..10000 {
            normal.step(0).unwrap();
            if normal.generation != generation {
                break;
            }
        }
        assert_ne!(normal.generation, generation);
        let mut fast = start(sequence, prologue);
        let mut transport = Transport::default();
        for _ in 0..10000 {
            let update = transport
                .advance(&mut fast, Duration::from_millis(34), input(speed), None)
                .unwrap();
            if update.changed {
                break;
            }
        }
        assert_eq!(fast.generation, normal.generation);
        assert_eq!(
            serde_json::to_value(&fast.run).unwrap(),
            serde_json::to_value(&normal.run).unwrap()
        );
        assert_eq!(fast.content.tick(), normal.content.tick());
        if let (Content::Story { story: a, .. }, Content::Story { story: b, .. }) =
            (&fast.content, &normal.content)
        {
            assert_eq!(
                serde_json::to_value(&a.state).unwrap(),
                serde_json::to_value(&b.state).unwrap()
            );
        }
        assert!(
            transport.clock.step().is_none(),
            "time spilled over the scene boundary"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; hold/release, pause, narrative audio seek, and gameplay speed isolation"]
fn fast_forward_resumes_audio_at_the_current_scene_time_and_never_speeds_gameplay() {
    let mut playback = start(0, true);
    let (output, received) = Output::recording();
    let mut transport = Transport::default();
    transport
        .advance(
            &mut playback,
            Duration::from_secs(1),
            input(4),
            Some(&output),
        )
        .unwrap();
    assert!(
        received
            .try_iter()
            .any(|c| matches!(c, Command::Music(None, false)))
    );
    assert!(playback.content.tick() > 0);
    let tick = playback.content.tick();
    transport
        .advance(
            &mut playback,
            Duration::ZERO,
            Input {
                paused: true,
                ..input(4)
            },
            Some(&output),
        )
        .unwrap();
    assert_eq!(playback.content.tick(), tick);
    assert!(
        received
            .try_iter()
            .any(|c| matches!(c, Command::SeekMusic(t) if (t - tick as f64 / 15.0).abs() < 0.0001))
    );
    transport
        .advance(
            &mut playback,
            Duration::from_millis(34),
            Input {
                fast_forward: true,
                ..input(1)
            },
            Some(&output),
        )
        .unwrap();
    assert!(
        received
            .try_iter()
            .any(|c| matches!(c, Command::Music(None, false)))
    );
    transport
        .advance(&mut playback, Duration::ZERO, input(1), Some(&output))
        .unwrap();
    assert!(
        received
            .try_iter()
            .any(|c| matches!(c, Command::SeekMusic(_)))
    );

    let mut gameplay = start(9, false);
    let mut transport = Transport::default();
    let tick = gameplay.content.tick();
    transport
        .advance(
            &mut gameplay,
            Duration::from_millis(34),
            Input {
                fast_forward: true,
                ..input(16)
            },
            None,
        )
        .unwrap();
    assert_eq!(gameplay.content.tick(), tick + 1);
    transport
        .advance(
            &mut gameplay,
            Duration::from_secs(10),
            Input {
                paused: true,
                ..input(0)
            },
            None,
        )
        .unwrap();
    assert_eq!(gameplay.content.tick(), tick + 1);
    transport
        .advance(&mut gameplay, Duration::ZERO, input(1), None)
        .unwrap();
    assert_eq!(gameplay.content.tick(), tick + 1);
}
