pub mod activation;
pub mod cinematic;
pub mod events;
pub mod score;
mod setup;
pub use setup::Statistics;

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
    pub items: crate::items::Items,
    pub hud: crate::hud::Hud,
    pub tick: u64,
    pub clock: i32,
    pub input: u16,
    pub rendered_view: crate::math::Matrix,
    pub cinematic: cinematic::Cinematic,
    pub events: events::Events,
    pub music: Vec<Option<i16>>,
    pub scene: Scene,
    pub collision: Collision,
    pub camera: Camera,
    pub intelligence: Intelligence,
    pub combat: crate::actor::Combat,
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
        mut mission: MissionAssets,
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
        let mut items = crate::items::Items::new(&mut scene);
        let construction = Construction::new(&mission, &mut scene, &math)?;
        let mut collision = Collision::default();
        let mut camera = Camera::new(
            tenchu_assets::camera::CameraTables::load(executable)?,
            mission.stage.start,
            &math,
        )?;
        let mut context = Context {
            mission: &mut mission,
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
        let (player_id, statistics) =
            setup::construct(&mut roster, &mut items, character, &mut context)?;
        roster.get_mut(player_id).initialize_rendered(&scene, &math);
        let events = events::Events::new(&mission.scripts.events, &roster, player_id);
        let rendered_view = camera.view;
        Ok(Self {
            mission,
            math,
            body,
            commands,
            cursor,
            random,
            effects,
            items,
            hud: crate::hud::Hud::new(executable)?,
            roster,
            player_id,
            statistics,
            tick: 0,
            clock: 0,
            input: 0,
            rendered_view,
            cinematic: cinematic::Cinematic::default(),
            events,
            music: Vec::new(),
            scene,
            collision,
            camera,
            intelligence: Intelligence::default(),
            combat: crate::actor::Combat::default(),
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
        self.music.clear();
        if self.cinematic.tail > 0 {
            self.cinematic.tail -= 1;
            self.intelligence.voices.clear();
            self.effects.sounds.clear();
            if self.cinematic.tail == 0 {
                self.cinematic.active = false;
                self.music.push(None);
            }
            return Ok(());
        }
        if !self.cinematic.active {
            self.input = input;
        }
        self.roster.begin_tick();
        self.intelligence.voices.clear();
        self.effects.begin_tick();
        self.items.begin_tick();
        self.drawing.bytes = 0;
        self.stage_sequence()?;
        if self.events.outcome != events::Outcome::Running {
            return Ok(());
        }
        self.visible.clear();
        self.cinematic.frame = self.cinematic.active;
        let input = if self.cinematic.active {
            self.cinematic.input
        } else {
            input
        };
        self.collision.compute(&mut self.scene)?;
        self.clock = self.clock.wrapping_add(1);
        if self.cinematic.active {
            self.cinematic_camera(input)?;
        } else {
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
        }
        let mut context = Context {
            mission: &mut self.mission,
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
                    let center = if context.scene.models[actor.parts[0]].conflict.is_some() {
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
                            &mut self.items,
                            &mut self.intelligence,
                            self.clock,
                            input,
                            &self.commands,
                            &mut context,
                        )
                        .update()?;
                        actor.shadow(&mut context, self.clock)?;
                        center
                    } else {
                        if actor.status == 17 {
                            actor.corpse(
                                &mut context,
                                &mut crate::actor::ActionContext {
                                    items: &mut self.items,
                                    roster: &mut self.roster,
                                    intelligence: &mut self.intelligence,
                                    statistics: &mut self.statistics,
                                    combat: &mut self.combat,
                                    player: self.player_id,
                                    selected_item: self.hud.selected,
                                    clock: self.clock,
                                },
                            )?;
                        }
                        [0; 3]
                    };
                    if actor.control(
                        &mut context,
                        &self.commands,
                        center,
                        &mut crate::actor::ActionContext {
                            items: &mut self.items,
                            roster: &mut self.roster,
                            intelligence: &mut self.intelligence,
                            statistics: &mut self.statistics,
                            combat: &mut self.combat,
                            player: self.player_id,
                            selected_item: self.hud.selected,
                            clock: self.clock,
                        },
                    )? {
                        self.visible.push(id);
                        if self.intelligence.action_halt == 0 {
                            if id == self.player_id {
                                actor.look_at_camera(&mut context)?;
                            } else {
                                actor.look_at_target(
                                    self.roster.get(self.player_id).root_model,
                                    &mut context,
                                )?;
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
        if !self.cinematic.active {
            self.activate()?;
        }
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
            &mut crate::effects::Environment {
                terrain: &self.mission.terrain,
                cursor: &mut self.cursor,
                clock: self.clock,
                owner_position: self.roster.get(self.player_id).position,
            },
        )?;
        self.drawing
            .effects(&self.effects, &self.mission, &self.math, self.camera.view)?;
        self.items.step(
            &mut Context {
                mission: &mut self.mission,
                math: &self.math,
                body: &self.body,
                cursor: &mut self.cursor,
                random: &mut self.random,
                effects: &mut self.effects,
                scene: &mut self.scene,
                collision: &mut self.collision,
                camera: &mut self.camera,
                drawing: &mut self.drawing,
            },
            &mut self.roster,
            &mut self.intelligence,
            self.player_id,
            self.clock,
        )?;
        for (actor, health, maximum) in self.combat.life_bars.drain(..) {
            self.hud.request_life(actor, health, maximum);
        }
        if self.cinematic.active {
            self.hud.sprites.clear();
            self.cinematic_text();
        } else {
            self.cinematic.sprites.clear();
            self.hud.step(
                self.roster.get(self.player_id),
                &self.mission.images,
                self.intelligence.strain,
                self.clock,
                &self.math,
                &mut self.effects,
                &mut self.drawing,
            );
        }
        for (i, &id) in self.visible.iter().enumerate() {
            self.roster.get_mut(id).prepare_draw(
                &mut self.scene,
                self.camera.view,
                i == 0,
                &mut self.drawing,
                &self.mission,
            )?;
        }
        self.drawing.finish(self.clock);
        self.scene.finish_tick();
        self.rendered_view = self.camera.view;
        self.advance_cinematic()?;
        Ok(())
    }
}
