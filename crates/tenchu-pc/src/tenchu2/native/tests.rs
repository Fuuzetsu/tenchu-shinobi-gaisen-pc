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

#[test]
#[ignore = "requires TENCHU2_DISC; enters real deep water through the complete native frame"]
fn native_swimming_uses_the_imported_water_surface() {
    use tenchu_core::tenchu2::{
        actor::State,
        terrain::{self, Cursor, Query},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sequence = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.map == 0)
        .unwrap()
        .sequence;
    let mut session = Session::load(&disc, sequence, 1).unwrap();
    let terrain = &session.world.terrain;
    let mut cursor = Cursor::default();
    let mut water = None;
    for node in terrain
        .cells
        .iter()
        .flat_map(|c| &c.nodes)
        .filter(|n| n.attributes & 0xf00 == 0x200)
    {
        let [x1, z1, x2, z2] = node.bounds.map(i32::from);
        for x in [x1 + (x2 - x1) / 4, (x1 + x2) / 2, x2 - (x2 - x1) / 4] {
            for z in [z1 + (z2 - z1) / 4, (z1 + z2) / 2, z2 - (z2 - z1) / 4] {
                let p = Position::new(x * 10, i32::from(node.y) * 10, z * 10);
                if terrain::level(
                    terrain,
                    &mut cursor,
                    p.wrapping_sub(Position::new(0, 1000, 0)),
                    Query::Uncached,
                )
                .unwrap()
                    == Some(p.y)
                    && terrain::level(
                        terrain,
                        &mut cursor,
                        p.wrapping_add(Position::new(0, 10, 0)),
                        Query::Uncached,
                    )
                    .unwrap()
                    .is_some_and(|y| y - p.y > 1000)
                {
                    water = Some(p);
                    break;
                }
            }
            if water.is_some() {
                break;
            }
        }
        if water.is_some() {
            break;
        }
    }
    let water = water.expect("No deep water in the source map");
    let actor = &mut session.world.actors[0].actor;
    actor.position = water;
    actor.previous = water;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    let mut entered = false;
    for _ in 0..60 {
        session.step(UP).unwrap();
        entered |= session.world.actors[0].actor.state == State::Swimming;
    }
    assert!(entered, "player never entered native swimming");
    assert!(session.world.actors[0].actor.health > 0);
    assert_ne!(session.world.actors[0].actor.position, water);
    eprintln!(
        "Native water at {water:?}: {:?}, {:?}",
        session.world.actors[0].actor.position, session.world.actors[0].actor.state
    );
}
