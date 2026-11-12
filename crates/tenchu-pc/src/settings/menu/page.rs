#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    #[default]
    General,
    Graphics,
    Overlays,
    Camera,
    Actor,
    Simulation,
    Lighting,
    Assets,
    Sky,
    ImageEffects,
    PlayerModel,
    PersistentEffects,
    Time,
    Weather,
    Water,
    ModelRefinement,
    SecondaryMotion,
    Experiment,
    ContactEffects,
    Vibration,
    Movement,
    EnemyBehavior,
    Bleeding,
    Tenchu2,
}

impl Page {
    pub const ALL: [Self; 24] = [
        Self::General,
        Self::Graphics,
        Self::Overlays,
        Self::Camera,
        Self::Actor,
        Self::Simulation,
        Self::Lighting,
        Self::Assets,
        Self::Sky,
        Self::ImageEffects,
        Self::PlayerModel,
        Self::PersistentEffects,
        Self::Time,
        Self::Weather,
        Self::Water,
        Self::ModelRefinement,
        Self::SecondaryMotion,
        Self::Experiment,
        Self::ContactEffects,
        Self::Vibration,
        Self::Movement,
        Self::EnemyBehavior,
        Self::Bleeding,
        Self::Tenchu2,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Graphics => "Graphics",
            Self::Overlays => "Overlays",
            Self::Camera => "Camera",
            Self::Actor => "Teleport / actor",
            Self::Simulation => "Simulation",
            Self::Lighting => "Lighting",
            Self::Assets => "Visual assets",
            Self::Sky => "Sky lighting",
            Self::ImageEffects => "Image effects",
            Self::PlayerModel => "Player model",
            Self::PersistentEffects => "Persistent effects",
            Self::Time => "Time of day",
            Self::Weather => "Weather",
            Self::Water => "Water",
            Self::ModelRefinement => "Model refinement",
            Self::SecondaryMotion => "Secondary motion",
            Self::Experiment => "Experiment",
            Self::ContactEffects => "Contact effects",
            Self::Vibration => "Vibration",
            Self::Movement => "Movement",
            Self::EnemyBehavior => "Enemy behavior",
            Self::Bleeding => "Bleeding",
            Self::Tenchu2 => "Tenchu 2",
        }
    }

    pub fn uses_models(self) -> bool {
        matches!(self, Self::Actor | Self::PlayerModel)
    }

    pub fn offset(self, direction: i32) -> Self {
        Self::ALL[(self as i32 + direction).rem_euclid(Self::ALL.len() as i32) as usize]
    }
}
