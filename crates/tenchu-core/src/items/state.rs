use super::Processor;
use crate::math::Position;

#[derive(Clone, Copy, Debug)]
pub struct Used<P> {
    pub owner: usize,
    pub phase: P,
}

impl<P> Used<P> {
    pub fn new(owner: usize, phase: P) -> Self {
        Self { owner, phase }
    }
    pub fn with_phase(self, phase: P) -> Self {
        Self { phase, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    Flying,
    Embedded,
    Fading,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gun {
    Muzzle { target: Position },
    Impact { target: Position },
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medicine {
    Begin,
    Drinking,
    Recover,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caltrops {
    Falling,
    Armed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Smoke {
    Fuse,
    Emitting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fire {
    Fuse,
    Detonate,
    Burning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mine {
    Place,
    Armed,
    Detonate,
    Burning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poison {
    Delay,
    Bait,
    Eating,
    Poisoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Napalm {
    Begin,
    Flying,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breath {
    Begin,
    Casting,
    Breathing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sleep {
    Begin,
    Throwing,
    Flying,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flute {
    Begin,
    Playing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resurrection {
    Begin,
    Particles,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rice {
    Begin,
    Planting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Begin,
    Casting,
    Moving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charm {
    Begin,
    Casting,
    Protected { remaining: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decoy {
    Fuse,
    Growing,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disguise {
    Begin,
    Casting,
    Apply,
    Active,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dog {
    Thrown {
        remaining: u16,
        player: usize,
    },
    Summoning {
        player: usize,
    },
    Active {
        slave: usize,
        remaining: u16,
        player: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lightning {
    Begin,
    Target,
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    Rolling,
    Available,
    Transfer { actor: usize },
}

#[derive(Clone, Copy, Debug, Default)]
pub enum Behavior {
    #[default]
    Inactive,
    Pickup {
        owner: Option<usize>,
        phase: Pickup,
    },
    Arrow(Used<Arrow>),
    Gun(Used<Gun>),
    Medicine(Used<Medicine>),
    Caltrops(Used<Caltrops>),
    Smoke(Used<Smoke>),
    Fire(Used<Fire>),
    Mine(Used<Mine>),
    Poison(Used<Poison>),
    Napalm(Used<Napalm>),
    Breath(Used<Breath>),
    Sleep(Used<Sleep>),
    Flute(Used<Flute>),
    Resurrection(Used<Resurrection>),
    Rice(Used<Rice>),
    Speed(Used<Speed>),
    Charm(Used<Charm>),
    Decoy(Used<Decoy>),
    Disguise(Used<Disguise>),
    Dog(Used<Dog>),
    Lightning(Used<Lightning>),
    Shuriken {
        owner: usize,
    },
    SightShot {
        owner: usize,
    },
    Grapple {
        owner: usize,
    },
    Darts {
        owner: usize,
    },
    Teleport {
        owner: usize,
    },
}

impl Behavior {
    pub fn owner(self) -> Option<usize> {
        match self {
            Self::Inactive => None,
            Self::Pickup {
                phase: Pickup::Transfer { actor },
                ..
            } => Some(actor),
            Self::Pickup { owner, .. } => owner,
            Self::Arrow(used) => Some(used.owner),
            Self::Gun(used) => Some(used.owner),
            Self::Medicine(used) => Some(used.owner),
            Self::Caltrops(used) => Some(used.owner),
            Self::Smoke(used) => Some(used.owner),
            Self::Fire(used) => Some(used.owner),
            Self::Mine(used) => Some(used.owner),
            Self::Poison(used) => Some(used.owner),
            Self::Napalm(used) => Some(used.owner),
            Self::Breath(used) => Some(used.owner),
            Self::Sleep(used) => Some(used.owner),
            Self::Flute(used) => Some(used.owner),
            Self::Resurrection(used) => Some(used.owner),
            Self::Rice(used) => Some(used.owner),
            Self::Speed(used) => Some(used.owner),
            Self::Charm(used) => Some(used.owner),
            Self::Decoy(used) => Some(used.owner),
            Self::Disguise(used) => Some(used.owner),
            Self::Dog(used) => Some(used.owner),
            Self::Lightning(used) => Some(used.owner),
            Self::Shuriken { owner } => Some(owner),
            Self::SightShot { owner } => Some(owner),
            Self::Grapple { owner } => Some(owner),
            Self::Darts { owner } => Some(owner),
            Self::Teleport { owner } => Some(owner),
        }
    }

    pub fn processor(self) -> Option<Processor> {
        Some(match self {
            Self::Inactive => return None,
            Self::Pickup { .. } => Processor::Pickup,
            Self::Arrow(..) => Processor::Arrow,
            Self::Gun(..) => Processor::Gun,
            Self::Medicine(..) => Processor::Medicine,
            Self::Caltrops(..) => Processor::Caltrops,
            Self::Smoke(..) => Processor::Smoke,
            Self::Fire(..) => Processor::Fire,
            Self::Mine(..) => Processor::Mine,
            Self::Poison(..) => Processor::Poison,
            Self::Napalm(..) => Processor::Napalm,
            Self::Breath(..) => Processor::Breath,
            Self::Sleep(..) => Processor::Sleep,
            Self::Flute(..) => Processor::Flute,
            Self::Resurrection(..) => Processor::Resurrection,
            Self::Rice(..) => Processor::Rice,
            Self::Speed(..) => Processor::Speed,
            Self::Charm(..) => Processor::Charm,
            Self::Decoy(..) => Processor::Decoy,
            Self::Disguise(..) => Processor::Disguise,
            Self::Dog(..) => Processor::Dog,
            Self::Lightning(..) => Processor::Lightning,
            Self::Shuriken { .. } => Processor::Shuriken,
            Self::SightShot { .. } => Processor::SightShot,
            Self::Grapple { .. } => Processor::Grapple,
            Self::Darts { .. } => Processor::Darts,
            Self::Teleport { .. } => Processor::Teleport,
        })
    }

    pub fn source_mode(self) -> u8 {
        match self {
            Self::Inactive => 0,
            Self::Pickup { phase, .. } => match phase {
                Pickup::Rolling => 0,
                Pickup::Available => 1,
                Pickup::Transfer { .. } => 2,
            },
            Self::Arrow(Used { phase, .. }) => match phase {
                Arrow::Flying => 0,
                Arrow::Embedded => 1,
                Arrow::Fading => 2,
            },
            Self::Gun(Used { phase, .. }) => match phase {
                Gun::Muzzle { .. } => 0,
                Gun::Impact { .. } => 1,
                Gun::Finished => 2,
            },
            Self::Medicine(Used { phase, .. }) => match phase {
                Medicine::Begin => 0,
                Medicine::Drinking => 1,
                Medicine::Recover => 2,
            },
            Self::Caltrops(Used { phase, .. }) => match phase {
                Caltrops::Falling => 0,
                Caltrops::Armed => 1,
            },
            Self::Smoke(Used { phase, .. }) => match phase {
                Smoke::Fuse => 0,
                Smoke::Emitting => 1,
            },
            Self::Fire(Used { phase, .. }) => match phase {
                Fire::Fuse => 0,
                Fire::Detonate => 1,
                Fire::Burning => 2,
            },
            Self::Mine(Used { phase, .. }) => match phase {
                Mine::Place => 0,
                Mine::Armed => 1,
                Mine::Detonate => 2,
                Mine::Burning => 3,
            },
            Self::Poison(Used { phase, .. }) => match phase {
                Poison::Delay => 0,
                Poison::Bait => 1,
                Poison::Eating => 2,
                Poison::Poisoned => 3,
            },
            Self::Napalm(Used { phase, .. }) => match phase {
                Napalm::Begin => 0,
                Napalm::Flying => 1,
                Napalm::Finished => 2,
            },
            Self::Breath(Used { phase, .. }) => match phase {
                Breath::Begin => 0,
                Breath::Casting => 1,
                Breath::Breathing => 2,
            },
            Self::Sleep(Used { phase, .. }) => match phase {
                Sleep::Begin => 0,
                Sleep::Throwing => 1,
                Sleep::Flying => 2,
                Sleep::Finished => 3,
            },
            Self::Flute(Used { phase, .. }) => match phase {
                Flute::Begin => 0,
                Flute::Playing => 1,
            },
            Self::Resurrection(Used { phase, .. }) => match phase {
                Resurrection::Begin => 0,
                Resurrection::Particles => 1,
                Resurrection::Finished => 2,
            },
            Self::Rice(Used { phase, .. }) => match phase {
                Rice::Begin => 0,
                Rice::Planting => 1,
            },
            Self::Speed(Used { phase, .. }) => match phase {
                Speed::Begin => 0,
                Speed::Casting => 1,
                Speed::Moving => 2,
            },
            Self::Charm(Used { phase, .. }) => match phase {
                Charm::Begin => 0,
                Charm::Casting => 1,
                Charm::Protected { .. } => 2,
            },
            Self::Decoy(Used { phase, .. }) => match phase {
                Decoy::Fuse => 0,
                Decoy::Growing => 1,
                Decoy::Active => 2,
            },
            Self::Disguise(Used { phase, .. }) => match phase {
                Disguise::Begin => 0,
                Disguise::Casting => 1,
                Disguise::Apply => 2,
                Disguise::Active => 3,
            },
            Self::Dog(Used { phase, .. }) => match phase {
                Dog::Thrown { .. } => 0,
                Dog::Summoning { .. } => 1,
                Dog::Active { .. } => 2,
            },
            Self::Lightning(Used { phase, .. }) => match phase {
                Lightning::Begin => 0,
                Lightning::Target => 1,
                Lightning::Hold => 2,
            },
            Self::Shuriken { .. } => 0,
            Self::SightShot { .. } => 0,
            Self::Grapple { .. } => 0,
            Self::Darts { .. } => 0,
            Self::Teleport { .. } => 0,
        }
    }
}
