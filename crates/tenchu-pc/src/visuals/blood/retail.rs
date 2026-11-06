use super::*;
use tenchu_core::effects::{Billboard, Graphic};

impl Blood {
    fn retain_spot(&mut self, sample: Sample) {
        self.original.push_back(Billboard {
            gore_source: None,
            graphic: Graphic::Blood {
                sprite: sample.sprite,
                stain: false,
                fading: false,
                airborne: false,
            },
            position: sample.position.to_array().map(|v| v as i32).into(),
            scale: [sample.scale; 2],
            color: [128; 3],
            rotation: sample.rotation,
        });
    }

    pub(super) fn retain_missing_retail(&mut self, sample: Option<Sample>) {
        if let Some(sample) = sample.filter(|s| s.mode > 0 && !s.stained) {
            self.retain_spot(sample);
        }
    }

    pub(super) fn retain_retail(
        &mut self,
        old: Option<Sample>,
        next: &mut Sample,
        continuous: bool,
    ) {
        if continuous {
            next.stained = old.is_some_and(|s| s.stained);
        } else {
            self.retain_missing_retail(old);
        }
        if next.mode == 3 && !next.stained {
            self.retain_spot(old.filter(|s| continuous && s.mode > 0).unwrap_or(*next));
            next.stained = true;
        }
    }
}
