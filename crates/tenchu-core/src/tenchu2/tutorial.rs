use super::actor::Actor;
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::tutorial::{Command, Lesson, Tutorial};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub course: usize,
    pub lesson: Option<usize>,
    pub spoken: Option<u16>,
    pub repetitions: i16,
    pub camera: i16,
}

pub struct Input<'a> {
    pub player: &'a Actor,
    pub duration: i16,
    pub camera: i16,
    pub defeated_practice_guards: usize,
    pub audio_finished: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Update {
    pub cue: Option<u16>,
    pub entered: Option<usize>,
    pub confirmation: bool,
    pub clear_alert: bool,
}

fn near(lesson: &Lesson, position: [i16; 3]) -> bool {
    lesson.position[1] == position[1]
        && (i32::from(position[0]) - i32::from(lesson.position[0])).abs() < i32::from(lesson.radius)
        && (i32::from(position[2]) - i32::from(lesson.position[2])).abs() < i32::from(lesson.radius)
}

impl State {
    pub fn step(&mut self, tutorial: &Tutorial, input: Input<'_>) -> Update {
        let mut update = Update::default();
        let player = input.player;
        let position = player.position.to_array().map(|v| (v / 1000) as i16);
        let Some(index) = self.lesson else {
            for (course, &index) in tutorial.starts.iter().enumerate() {
                if near(&tutorial.lessons[index], position) {
                    self.course = course;
                    self.lesson = Some(index);
                    self.spoken = None;
                    update.entered = Some(course);
                    break;
                }
            }
            return update;
        };
        let [left, right, back, front] = tutorial.bounds[self.course];
        if position[0] < left || position[0] > right || position[2] < back || position[2] > front {
            self.lesson = None;
            self.spoken = None;
            return update;
        }
        let lesson = &tutorial.lessons[index];
        if self.spoken != Some(lesson.cue) {
            update.cue = Some(lesson.cue);
            self.spoken = Some(lesson.cue);
            self.repetitions = 0;
            update.confirmation = true;
        }
        let audio_finished = input.audio_finished && update.cue.is_none();
        let advance = match lesson.command {
            Command::Finished => false,
            Command::Reach | Command::ReachNext => {
                let target = &tutorial.lessons
                    [index + usize::from(matches!(lesson.command, Command::ReachNext))];
                near(target, position) && player.terrain.height == i32::from(player.height)
            }
            Command::Action | Command::ActionRange => {
                if (matches!(lesson.command, Command::ActionRange)
                    || player.motion.action == lesson.action)
                    && player.motion.loops == 0
                    && i32::from(player.motion.frame) == i32::from(input.duration) * 2 / 3
                    && player.motion.action >= lesson.action
                    && i32::from(player.motion.action) <= i32::from(lesson.action) + 3
                {
                    self.repetitions = self.repetitions.wrapping_add(1);
                    self.repetitions >= i16::from(lesson.repetitions)
                } else {
                    false
                }
            }
            Command::DefeatPracticeGuards => {
                update.clear_alert = input.defeated_practice_guards >= 2;
                update.clear_alert
            }
            Command::ThrowShuriken => {
                if player.motion.action == 0x803
                    && player.inventory.active.index() == 4
                    && player.motion.frame == input.duration / 2
                {
                    self.repetitions = self.repetitions.wrapping_add(1);
                    if self.repetitions >= i16::from(lesson.repetitions) {
                        true
                    } else {
                        if audio_finished {
                            update.cue = Some(19);
                        }
                        false
                    }
                } else {
                    false
                }
            }
            Command::CameraDirections => {
                if matches!(
                    (input.camera, self.camera),
                    (9 | 10, 0) | (11, 9) | (12, 10)
                ) {
                    self.repetitions = self.repetitions.wrapping_add(1);
                }
                self.camera = input.camera;
                self.repetitions >= 2
            }
            Command::LeaveWall => !(0x530..=0x537).contains(&player.motion.action),
            Command::SelectShuriken => {
                if self.repetitions == -1
                    && player
                        .inventory
                        .selected
                        .is_some_and(|item| item.index() == 4)
                {
                    true
                } else {
                    if player
                        .inventory
                        .selected
                        .is_some_and(|item| item.index() == 0)
                    {
                        self.repetitions = -1;
                    }
                    false
                }
            }
            Command::Look => {
                if input.camera == 6 {
                    self.repetitions = 1;
                }
                self.repetitions != 0 && audio_finished
            }
        };
        if advance {
            self.lesson = Some(index + 1);
        }
        update
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Training {
    pub data: std::sync::Arc<Tutorial>,
    pub state: State,
    pub playing: Option<(u16, u32)>,
}

impl Training {
    pub fn new(data: Tutorial) -> Self {
        Self {
            data: std::sync::Arc::new(data),
            state: State::default(),
            playing: None,
        }
    }
    pub fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> Result<(), C::Error> {
        out.shared_arc(&self.data, true)?;
        out.value(&self.state)?;
        out.value(&self.playing)
    }
    pub fn validate(&self) -> crate::Result<()> {
        let data = &self.data;
        if data.lessons.len() != 27
            || data.tracks.values().any(|track| track.start >= track.end)
            || data.starts.iter().any(|&i| i >= data.lessons.len())
            || self.state.course >= data.starts.len()
            || self.state.lesson.is_some_and(|i| i >= data.lessons.len())
            || data.lessons.iter().enumerate().any(|(i, l)| {
                !data.tracks.contains_key(&l.cue)
                    || i + 1 == data.lessons.len() && !matches!(l.command, Command::Finished)
            })
            || self
                .state
                .spoken
                .is_some_and(|cue| !data.tracks.contains_key(&cue))
            || self
                .playing
                .is_some_and(|(cue, _)| !data.tracks.contains_key(&cue))
        {
            return Err("Invalid training lesson or narration state".into());
        }
        Ok(())
    }
}
