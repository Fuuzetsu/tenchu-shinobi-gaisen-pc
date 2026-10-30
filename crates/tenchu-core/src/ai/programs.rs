use crate::Result;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Idle {
    #[default]
    None,
    Player,
    Disabled,
    Trace,
    Watch,
    Wander,
    Ninja,
    Sleep,
    Hunt,
    Follow,
}

impl Idle {
    fn decode(id: u8) -> Result<Self> {
        Ok(match id {
            0 => Self::None,
            1 => Self::Player,
            2 => Self::Disabled,
            3 => Self::Trace,
            4 => Self::Watch,
            5 => Self::Wander,
            6 => Self::Ninja,
            7 => Self::Sleep,
            8 => Self::Hunt,
            9 => Self::Follow,
            _ => return Err(format!("Unknown idle AI program {id}")),
        })
    }

    pub fn source_id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Player => 1,
            Self::Disabled => 2,
            Self::Trace => 3,
            Self::Watch => 4,
            Self::Wander => 5,
            Self::Ninja => 6,
            Self::Sleep => 7,
            Self::Hunt => 8,
            Self::Follow => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Notice {
    #[default]
    None,
    Player,
    Disabled,
    Turn,
    Contact,
    Alarm,
}

impl Notice {
    fn decode(id: u8) -> Result<Self> {
        Ok(match id {
            0 => Self::None,
            1 => Self::Player,
            2 => Self::Disabled,
            3 => Self::Turn,
            4 => Self::Contact,
            5 => Self::Alarm,
            _ => return Err(format!("Unknown notice AI program {id}")),
        })
    }

    pub fn source_id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Player => 1,
            Self::Disabled => 2,
            Self::Turn => 3,
            Self::Contact => 4,
            Self::Alarm => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Combat {
    #[default]
    None,
    Player,
    Disabled,
    CallAid,
    Chase,
    Point,
    Escape,
    Area,
    HitAway,
    FirstAttack,
}

impl Combat {
    fn decode(id: u8) -> Result<Self> {
        Ok(match id {
            0 => Self::None,
            1 => Self::Player,
            2 => Self::Disabled,
            3 => Self::CallAid,
            4 => Self::Chase,
            5 => Self::Point,
            6 => Self::Escape,
            7 => Self::Area,
            8 => Self::HitAway,
            9 => Self::FirstAttack,
            _ => return Err(format!("Unknown combat AI program {id}")),
        })
    }

    pub fn source_id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Player => 1,
            Self::Disabled => 2,
            Self::CallAid => 3,
            Self::Chase => 4,
            Self::Point => 5,
            Self::Escape => 6,
            Self::Area => 7,
            Self::HitAway => 8,
            Self::FirstAttack => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Search {
    #[default]
    None,
    Player,
    Disabled,
    Abandon,
    Investigate,
    Pursue,
}

impl Search {
    fn decode(id: u8) -> Result<Self> {
        Ok(match id {
            0 => Self::None,
            1 => Self::Player,
            2 => Self::Disabled,
            3 => Self::Abandon,
            4 => Self::Investigate,
            5 => Self::Pursue,
            _ => return Err(format!("Unknown search AI program {id}")),
        })
    }

    pub fn source_id(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Player => 1,
            Self::Disabled => 2,
            Self::Abandon => 3,
            Self::Investigate => 4,
            Self::Pursue => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Programs {
    pub idle: Idle,
    pub notice: Notice,
    pub combat: Combat,
    pub search: Search,
}

impl Programs {
    pub fn decode(mix: i16) -> Result<Self> {
        let ids: [u8; 4] = std::array::from_fn(|i| ((mix as u16 >> (i * 4)) & 15) as u8);
        Ok(Self {
            idle: Idle::decode(ids[0])?,
            notice: Notice::decode(ids[1])?,
            combat: Combat::decode(ids[2])?,
            search: Search::decode(ids[3])?,
        })
    }

    pub fn source_ids(self) -> [u8; 4] {
        [
            self.idle.source_id(),
            self.notice.source_id(),
            self.combat.source_id(),
            self.search.source_id(),
        ]
    }
}
