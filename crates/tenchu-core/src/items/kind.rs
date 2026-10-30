use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Grapple,
    Shuriken,
    Caltrops,
    Medicine,
    Grenade,
    SmokeBomb,
    Mine,
    PoisonedFood,
    ColoredRice,
    SleepingGas,
    Resurrection,
    Disguise,
    ProtectionCharm,
    SpeedScroll,
    Decoy,
    Darts,
    Dog,
    FireBreath,
    Flute,
    Armor,
    Gun,
    Arrow,
    Napalm,
    Lightning,
    Teleport,
}

impl Kind {
    pub const ALL: [Self; 25] = [
        Self::Grapple,
        Self::Shuriken,
        Self::Caltrops,
        Self::Medicine,
        Self::Grenade,
        Self::SmokeBomb,
        Self::Mine,
        Self::PoisonedFood,
        Self::ColoredRice,
        Self::SleepingGas,
        Self::Resurrection,
        Self::Disguise,
        Self::ProtectionCharm,
        Self::SpeedScroll,
        Self::Decoy,
        Self::Darts,
        Self::Dog,
        Self::FireBreath,
        Self::Flute,
        Self::Armor,
        Self::Gun,
        Self::Arrow,
        Self::Napalm,
        Self::Lightning,
        Self::Teleport,
    ];

    pub fn index(self) -> usize {
        match self {
            Self::Grapple => 0,
            Self::Shuriken => 1,
            Self::Caltrops => 2,
            Self::Medicine => 3,
            Self::Grenade => 4,
            Self::SmokeBomb => 5,
            Self::Mine => 6,
            Self::PoisonedFood => 7,
            Self::ColoredRice => 8,
            Self::SleepingGas => 9,
            Self::Resurrection => 10,
            Self::Disguise => 11,
            Self::ProtectionCharm => 12,
            Self::SpeedScroll => 13,
            Self::Decoy => 14,
            Self::Darts => 15,
            Self::Dog => 16,
            Self::FireBreath => 17,
            Self::Flute => 18,
            Self::Armor => 19,
            Self::Gun => 20,
            Self::Arrow => 21,
            Self::Napalm => 22,
            Self::Lightning => 23,
            Self::Teleport => 24,
        }
    }
}

impl TryFrom<usize> for Kind {
    type Error = String;
    fn try_from(index: usize) -> Result<Self> {
        Self::ALL
            .get(index)
            .copied()
            .ok_or_else(|| format!("Invalid item kind {index}"))
    }
}
