use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::story::{Command, Script};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Presentation {
    next: usize,
    pub fade: Option<Fade>,
    pub tint: Option<Overlay>,
    pub warp: Option<Warp>,
    pub portraits: [Portrait; 7],
    pub picture: Option<Picture>,
    pub caption: Option<Caption>,
    pub camera_roll: i16,
    pub weather: bool,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            next: 0,
            fade: Some(Fade {
                white: false,
                outward: false,
                duration: 8,
                age: 0,
            }),
            tint: None,
            warp: None,
            portraits: [Portrait::default(); 7],
            picture: None,
            caption: None,
            camera_roll: 0,
            weather: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fade {
    pub white: bool,
    pub outward: bool,
    pub duration: i16,
    pub age: i16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portrait {
    pub position: [i16; 2],
    pub age: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picture {
    pub index: usize,
    pub duration: i16,
    pub remaining: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caption {
    pub command: usize,
    pub remaining: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warp {
    pub variant: bool,
    pub phase: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overlay {
    pub rgb: [i16; 3],
    pub subtract: bool,
    pub layer: u8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PresentationFrame {
    pub overlays: Vec<Overlay>,
    pub portraits: Vec<(usize, Portrait, u8)>,
    pub picture: Option<(usize, i16)>,
    pub caption: Option<usize>,
    pub effects: Vec<usize>,
}

impl Presentation {
    pub fn update(&mut self, script: &Script, frame: i16, draw: bool) -> PresentationFrame {
        let mut output = PresentationFrame::default();
        // Pictures precede command dispatch, so a new picture starts next draw.
        if draw && let Some(picture) = &mut self.picture {
            let intensity = if picture.remaining < 8 {
                picture.remaining.wrapping_mul(16)
            } else if i32::from(picture.duration) - 8 < i32::from(picture.remaining) {
                picture
                    .duration
                    .wrapping_sub(picture.remaining)
                    .wrapping_mul(16)
            } else {
                128
            };
            output.picture = Some((picture.index, intensity));
            if picture.remaining <= 0 {
                self.picture = None;
            } else {
                picture.remaining -= 1;
            }
        }
        if i32::from(frame) < script.duration {
            while let Some(timed) = script.commands.get(self.next)
                && timed.frame == frame
            {
                let index = self.next;
                self.next += 1;
                match &timed.command {
                    Command::Effect { .. } => output.effects.push(index),
                    Command::Fade {
                        white,
                        outward,
                        duration,
                    } => {
                        self.fade = Some(Fade {
                            white: *white,
                            outward: *outward,
                            duration: *duration,
                            age: 0,
                        });
                    }
                    Command::ClearFadeAndWarp => {
                        self.fade = None;
                        self.warp = None;
                    }
                    Command::Tint { rgb, subtract } => {
                        self.tint = Some(Overlay {
                            rgb: rgb.map(i16::from),
                            subtract: *subtract,
                            layer: 1,
                        })
                    }
                    Command::ClearTint => self.tint = None,
                    Command::Warp { variant, phase } => {
                        self.warp = Some(Warp {
                            variant: *variant,
                            phase: *phase,
                        })
                    }
                    Command::Portrait { index, position } => {
                        self.portraits[*index] = Portrait {
                            position: *position,
                            age: 1,
                        }
                    }
                    Command::HidePortrait(index) => self.portraits[*index].age = 82,
                    Command::Picture { index, duration } => {
                        self.picture = Some(Picture {
                            index: *index,
                            duration: *duration,
                            remaining: *duration,
                        })
                    }
                    Command::CameraRoll(value) => self.camera_roll = *value,
                    Command::Weather(enabled) => self.weather = *enabled,
                    Command::Caption { duration, .. } => {
                        self.caption = (*duration != 0).then_some(Caption {
                            command: index,
                            remaining: *duration,
                        })
                    }
                    Command::ClearCaption => self.caption = None,
                }
            }
        }
        if let Some(fade) = &mut self.fade {
            let amount = i32::from(fade.age) * 255 / i32::from(fade.duration);
            if draw {
                output.overlays.push(Overlay {
                    rgb: [if fade.outward { amount } else { 255 - amount } as i16; 3],
                    subtract: !fade.white,
                    layer: 2,
                });
            }
            let previous = fade.age;
            fade.age = fade.age.wrapping_add(1);
            if previous > fade.duration {
                if fade.outward {
                    fade.age = fade.duration;
                } else {
                    self.fade = None;
                }
            }
        }
        if draw && let Some(tint) = self.tint {
            output.overlays.push(tint);
        }
        for (index, portrait) in self.portraits.iter_mut().enumerate() {
            if portrait.age == 0 {
                continue;
            }
            if draw {
                let intensity = match portrait.age {
                    1..=8 => portrait.age * 16 - 1,
                    9..=82 => 128,
                    _ => (90 - portrait.age) * 16,
                };
                output.portraits.push((index, *portrait, intensity));
            }
            portrait.age = if portrait.age == 90 {
                0
            } else {
                portrait.age + 1
            };
        }
        if draw
            && i32::from(frame) < script.duration
            && let Some(caption) = &mut self.caption
        {
            output.caption = Some(caption.command);
            caption.remaining = caption.remaining.wrapping_sub(1);
            if caption.remaining == 0 {
                self.caption = None;
            }
        }
        output
    }
}
