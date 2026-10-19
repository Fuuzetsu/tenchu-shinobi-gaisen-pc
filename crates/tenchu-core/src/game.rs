use crate::{
    Result,
    actor::{Actor, Context},
    camera::{Camera, Environment},
    collision::Collision,
    effects::Effects,
    math::Math,
    random::Random,
    scene::Scene,
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
    pub scene: Scene,
    pub collision: Collision,
    pub camera: Camera,
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
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let mut camera = Camera::new(
            tenchu_assets::camera::CameraTables::load(executable)?,
            mission.stage.start,
            &math,
        )?;
        let mut context = Context {
            mission: &mission,
            math: &math,
            body: &body,
            cursor: &mut cursor,
            random: &mut random,
            effects: &mut effects,
            scene: &mut scene,
            collision: &mut collision,
            camera: &mut camera,
        };
        let mut player = Actor::new(0, character, [0; 3], 0, &mut context)?;
        player.position = mission.stage.start;
        player.rotation = [0, mission.stage.facing as i16, 0];
        scene.set_translation(player.root_model, player.position);
        player.initialize_rendered(&scene, &math);
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
            scene,
            collision,
            camera,
        })
    }

    pub fn step(&mut self, input: u16) -> Result<()> {
        self.tick += 1;
        self.collision.compute(&mut self.scene)?;
        self.camera.advance(
            &self.player,
            input,
            &mut Environment {
                terrain: &self.mission.terrain,
                cursor: &mut self.cursor,
                math: &self.math,
                body: &self.body,
            },
            &mut self.scene,
        )?;
        let mut context = Context {
            mission: &self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
        };
        let center = self.player.collision_center(&context);
        self.player.terrain_contact(&mut context, center, true)?;
        self.player.body_contact(&mut context, center)?;
        self.player
            .control(&mut context, input, &self.commands, center)?;
        self.player.look_at_camera(&mut context)?;
        self.effects.step(&mut self.random);
        self.player.prepare_draw(&mut self.scene)?;
        self.scene.finish_tick();
        Ok(())
    }
}
