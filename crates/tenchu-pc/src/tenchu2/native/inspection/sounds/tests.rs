use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC; overlay bounds follow the original quantized lesson checks, including negative coordinates"]
fn sound_regions_match_lesson_entry_conditions() {
    use tenchu_core::tenchu2::tutorial::{Input, State};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let session = Session::load(&disc, 1, 1).unwrap();
    let mut player = session.world.actors[session.world.player].actor.clone();
    let mut data = (*session.world.tutorial.as_ref().unwrap().data).clone();
    let actual_starts: Vec<_> = data
        .starts
        .iter()
        .map(|&index| data.lessons[index].clone())
        .collect();
    let mut lessons = actual_starts;
    for center in [[-3, -2, -1], [0, 0, 0], [3, 2, 1]] {
        for radius in [1, 3] {
            let mut lesson = lessons[0].clone();
            lesson.position = center;
            lesson.radius = radius;
            lessons.push(lesson);
        }
    }
    data.starts = [0; 4];
    for lesson in lessons {
        let [min, max] = lesson_bounds(&lesson).unwrap();
        data.lessons[0] = lesson;
        let center = ((min + max) * 0.5).as_ivec3();
        for axis in 0..3 {
            for coordinate in [
                min[axis] as i32 - 1,
                min[axis] as i32,
                center[axis],
                max[axis] as i32,
                max[axis] as i32 + 1,
            ] {
                let mut position = center;
                position[axis] = coordinate;
                player.position = position.to_array().into();
                let update = State::default().step(
                    &data,
                    Input {
                        player: &player,
                        duration: 0,
                        camera: 0,
                        defeated_practice_guards: 0,
                        audio_finished: true,
                    },
                );
                let inside =
                    position.as_vec3().cmpge(min).all() && position.as_vec3().cmple(max).all();
                assert_eq!(
                    update.entered.is_some(),
                    inside,
                    "position {position:?}, region {min:?}..{max:?}"
                );
            }
        }
    }
}
