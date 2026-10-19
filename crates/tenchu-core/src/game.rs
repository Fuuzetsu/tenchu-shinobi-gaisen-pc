use crate::{
    Result,
    actor::{Actor, Context},
    effects::Effects,
    math::Math,
    random::Random,
    terrain::Cursor,
};
use tenchu_assets::{
    executable::{BodyTables, Command, Executable},
    stage::MissionAssets,
};

pub struct Game {
    pub mission: MissionAssets,
    pub math: Math,
    pub player: Actor,
    pub random: Random,
    pub effects: Effects,
    pub tick: u64,
    body: BodyTables,
    commands: Vec<Command>,
    cursor: Cursor,
}

impl Game {
    pub fn new(
        mission: MissionAssets,
        executable: &Executable,
        character: i16,
        seed: u32,
    ) -> Result<Self> {
        let math = Math {
            tables: executable.math_tables()?,
        };
        let body = executable.body_tables()?;
        let commands = executable.commands()?;
        let mut cursor = Cursor::new(&mission.terrain);
        let mut random = Random::new(seed);
        let mut effects = Effects::default();
        let mut context = Context {
            mission: &mission,
            math: &math,
            body: &body,
            cursor: &mut cursor,
            random: &mut random,
            effects: &mut effects,
        };
        let mut player = Actor::new(character, [0; 3], 0, &mut context)?;
        player.position = mission.stage.start;
        player.rotation = [0, mission.stage.facing as i16, 0];
        Ok(Self {
            mission,
            math,
            body,
            commands,
            cursor,
            random,
            effects,
            player,
            tick: 0,
        })
    }

    pub fn step(&mut self, input: u16) -> Result<()> {
        self.tick += 1;
        let mut context = Context {
            mission: &self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
        };
        let center = self.player.collision_center(&context);
        self.player.terrain_contact(&mut context, center, true)?;
        self.player
            .control(&mut context, input, &self.commands, center)?;
        self.effects.step(&mut self.random);
        Ok(())
    }
}
