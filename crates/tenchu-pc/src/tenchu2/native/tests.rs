use super::*;
use tenchu_core::tenchu2::input::{ATTACK, CROUCH, JUMP, RIGHT, UP};

#[test]
#[ignore = "requires TENCHU2_DISC; runs the native frame and all three campaign heroes"]
fn native_frame_controls_and_audio_continue_after_restoring_a_world() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    for hero in 0..3 {
        let sequence = layouts
            .iter()
            .find(|l| match hero {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap()
            .sequence;
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let start = session.world.actors[0].actor.position;
        let mut moved = false;
        let mut sounds = 0;
        for tick in 0..180 {
            let held = match tick {
                0..30 => UP,
                30..60 => CROUCH | UP,
                60..70 => RIGHT,
                70 => JUMP,
                130 => ATTACK,
                _ => 0,
            };
            sounds += session
                .step(held)
                .unwrap_or_else(|e| panic!("hero {hero}, tick {tick}: {e}"))
                .len();
            moved |= session.world.actors[0].actor.position != start;
        }
        assert!(moved, "hero {hero} did not move");
        let mut restored = Session::load(&disc, sequence, 1).unwrap();
        restored.world =
            serde_json::from_slice(&serde_json::to_vec(&session.world).unwrap()).unwrap();
        restored
            .assets
            .resources
            .clips
            .clone_from(&session.assets.resources.clips);
        for tick in 0..120 {
            let held = if tick == 0 { ATTACK } else { 0 };
            assert_eq!(
                session.step(held).unwrap(),
                restored.step(held).unwrap(),
                "hero {hero} sound continuation {tick}"
            );
        }
        assert_eq!(
            serde_json::to_value(&session.world).unwrap(),
            serde_json::to_value(&restored.world).unwrap()
        );
        eprintln!(
            "JP hero {hero} sequence {sequence}: 300 native ticks, {sounds} sounds, pending {:?}",
            session.pending
        );
    }
}
