#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Action,
    Walk,
    Swim,
    Grapple,
    Engage,
    Chase,
    Attack,
    Transition,
    Jump,
    Hang,
    Crouch,
    Wall,
    CeilingHang,
    AimShuriken,
    UseItem,
    Damaged,
    Dead,
    Unsupported(i16),
}

impl State {
    pub fn from_source_id(id: i16) -> Self {
        match id {
            0 => Self::Normal,
            1 => Self::Action,
            2 => Self::Walk,
            3 => Self::Swim,
            4 => Self::Grapple,
            5 => Self::Engage,
            6 => Self::Chase,
            7 => Self::Attack,
            8 => Self::Transition,
            9 => Self::Jump,
            10 => Self::Hang,
            11 => Self::Crouch,
            12 => Self::Wall,
            13 => Self::CeilingHang,
            14 => Self::AimShuriken,
            15 => Self::UseItem,
            16 => Self::Damaged,
            17 => Self::Dead,
            other => Self::Unsupported(other),
        }
    }

    pub fn from_motion(action: i16) -> Self {
        Self::from_source_id(i16::from((action >> 8) as i8))
    }

    pub fn source_id(self) -> i16 {
        match self {
            Self::Normal => 0,
            Self::Action => 1,
            Self::Walk => 2,
            Self::Swim => 3,
            Self::Grapple => 4,
            Self::Engage => 5,
            Self::Chase => 6,
            Self::Attack => 7,
            Self::Transition => 8,
            Self::Jump => 9,
            Self::Hang => 10,
            Self::Crouch => 11,
            Self::Wall => 12,
            Self::CeilingHang => 13,
            Self::AimShuriken => 14,
            Self::UseItem => 15,
            Self::Damaged => 16,
            Self::Dead => 17,
            Self::Unsupported(id) => id,
        }
    }
}
