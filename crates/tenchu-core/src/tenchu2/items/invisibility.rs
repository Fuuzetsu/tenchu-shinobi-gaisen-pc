use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invisibility {
    pub owner: usize,
    pub age: i32,
    pub trail: bool,
}

impl Invisibility {
    pub fn begin(owner: usize, concealment: &mut u16) -> Option<Self> {
        if *concealment != 0 {
            return None;
        }
        *concealment = 1;
        Some(Self {
            owner,
            age: 0,
            trail: true,
        })
    }

    pub fn advance(&mut self, concealment: &mut u16, translucent: &mut bool) -> bool {
        let alive = *concealment != 0 && {
            let previous = self.age;
            self.age = self.age.wrapping_add(1);
            previous <= 900
        };
        if !alive {
            *translucent = false;
            *concealment = 0;
        }
        if self.age == 70 {
            self.trail = false;
        }
        if self.age == 60 {
            *translucent = true;
            *concealment = 2;
        }
        alive
    }
}
