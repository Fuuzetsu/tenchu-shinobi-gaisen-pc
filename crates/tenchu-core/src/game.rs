pub mod activation;
mod setup;

use crate::{
    Result,
    actor::{Actor, Context},
    ai::{Controller, Intelligence, Player},
    camera::{Camera, Environment},
    collision::Collision,
    construction::Construction,
    drawing::Drawing,
    effects::Effects,
    math::Math,
    random::Random,
    roster::Roster,
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
    pub roster: Roster,
    pub player_id: usize,
    pub statistics: setup::Statistics,
    pub random: Random,
    pub effects: Effects,
    pub tick: u64,
    pub scene: Scene,
    pub collision: Collision,
    pub camera: Camera,
    pub intelligence: Intelligence,
    pub visible: Vec<usize>,
    pub activation: activation::Activation,
    pub drawing: Drawing,
    pub construction: Construction,
    pub(crate) body: BodyTables,
    pub(crate) commands: Vec<Command>,
    pub(crate) cursor: Cursor,
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
        let mut drawing = Drawing::new(executable)?;
        let construction = Construction::new(&mission, &mut scene, &math)?;
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
            drawing: &mut drawing,
        };
        let mut roster = Roster::default();
        let (player_id, statistics) = setup::construct(&mut roster, character, &mut context)?;
        roster.get_mut(player_id).initialize_rendered(&scene, &math);
        Ok(Self {
            mission,
            math,
            body,
            commands,
            cursor,
            random,
            effects,
            roster,
            player_id,
            statistics,
            tick: 0,
            scene,
            collision,
            camera,
            intelligence: Intelligence::default(),
            visible: vec![player_id],
            activation: activation::Activation::default(),
            drawing,
            construction,
        })
    }

    pub fn player(&self) -> &Actor {
        self.roster.get(self.player_id)
    }

    pub fn step(&mut self, input: u16) -> Result<()> {
        self.tick += 1;
        self.roster.begin_tick();
        self.visible.clear();
        self.intelligence.voices.clear();
        self.effects.begin_tick();
        self.drawing.bytes = 0;
        self.collision.compute(&mut self.scene)?;
        self.camera.advance(
            self.roster.get(self.player_id),
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
            drawing: &mut self.drawing,
        };
        let mut index = 0;
        while index < self.roster.order.len() {
            let id = self.roster.order[index];
            if self.roster.get(id).attribute & crate::ids::ATTR_SUSPEND == 0 {
                let mut actor = self.roster.take(id);
                let result = (|| -> Result<()> {
                    let center = actor.collision_center(&context);
                    actor.terrain_contact(&mut context, center, id == self.player_id)?;
                    actor.body_contact(&mut context, center)?;
                    context
                        .scene
                        .set_translation(actor.root_model, actor.position);
                    let player = Player::new(
                        if id == self.player_id {
                            &actor
                        } else {
                            self.roster.get(self.player_id)
                        },
                        &context,
                    );
                    Controller::new(
                        &mut actor,
                        player,
                        &mut self.roster,
                        &mut self.intelligence,
                        self.tick as i32,
                        input,
                        &self.commands,
                        &mut context,
                    )
                    .update()?;
                    actor.shadow(&mut context, self.tick as i32)?;
                    if actor.control(&mut context, &self.commands, center, id == self.player_id)? {
                        self.visible.push(id);
                        if self.intelligence.action_halt == 0 {
                            if id == self.player_id {
                                actor.look_at_camera(&mut context)?;
                            } else {
                                actor.look_at_target(player.root, &mut context)?;
                            }
                        }
                    }
                    Ok(())
                })();
                self.roster.restore(actor);
                result?;
            }
            index += 1;
        }
        self.statistics.enemies = self
            .statistics
            .enemies
            .wrapping_add(self.intelligence.population_delta[0]);
        self.statistics.citizens = self
            .statistics
            .citizens
            .wrapping_add(self.intelligence.population_delta[1]);
        self.intelligence.population_delta = [0; 2];
        self.activate()?;
        self.drawing.fog = true;
        self.drawing.bytes += self.construction.packets(
            &self.mission,
            &mut self.scene,
            &self.camera,
            &self.drawing.subdivision,
        )?;
        self.effects.step(
            &mut self.random,
            crate::effects::View {
                position: self.camera.position,
                matrix: self.camera.view,
            },
            &mut self.scene,
        )?;
        self.drawing
            .effects(&self.effects, &self.mission, &self.math, self.camera.view)?;
        for (i, &id) in self.visible.iter().enumerate() {
            self.roster.get_mut(id).prepare_draw(
                &mut self.scene,
                self.camera.view,
                i == 0,
                &mut self.drawing,
                &self.mission,
            )?;
        }
        self.drawing.finish(self.tick);
        self.scene.finish_tick();
        Ok(())
    }
}
