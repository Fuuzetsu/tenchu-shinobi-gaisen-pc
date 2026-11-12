use super::*;
use sha2::{Digest, Sha256};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_CULTIST_REPLAY; recorded Mission 6 kill and original sound-channel reuse"]
fn recorded_cultist_death_retains_retail_sound_sequence() {
    let path = std::env::var_os("TENCHU_CULTIST_REPLAY").expect("Set TENCHU_CULTIST_REPLAY");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "0683ad09558bf97cfe03fab6cd42aed028fba6a44242cc737ca168d73fe3cd56"
    );
    let tape: crate::retail::tape::Tape = serde_json::from_slice(&bytes).unwrap();
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let (mut audio, commands) = recording(disc.as_ref(), &assets);
    assert_eq!(tape.missions.len(), 1);
    let mission = &tape.missions[0];
    assert_eq!(mission.updates.len(), 359);
    let mut game = mission.initial.load(&assets).unwrap();
    let mut events = Vec::new();
    let mut digest = Sha256::new();
    for update in &mission.updates {
        update.apply(&mut game).unwrap();
        audio.tick(&game, None, Default::default(), []).unwrap();
        for command in commands.try_iter() {
            if let Command::Effect(event) = command {
                digest.update(game.clock.to_le_bytes());
                digest.update(event.sound.id.to_le_bytes());
                events.push((game.clock, event.sound));
            }
        }
    }
    assert_eq!(events.len(), 55);
    assert_eq!(
        format!("{:x}", digest.finalize()),
        "03cacf32875736191c017dac42802f29be4e548510c34289751f33952c069fe2"
    );
    let scream = events
        .iter()
        .position(|(_, sound)| sound.id == 0x68)
        .unwrap();
    assert_eq!(events[scream], (65, Sound::local(0x68)));
    let replacement = events[scream + 24];
    assert_eq!(replacement.0, 149);
    assert_eq!(replacement.1.id, 0x44);
}
