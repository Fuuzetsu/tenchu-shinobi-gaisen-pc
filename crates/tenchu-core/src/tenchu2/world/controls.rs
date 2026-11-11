use super::{Assets, Instance, World};
use crate::{
    Result,
    math::{Matrix, Position},
    random::Random,
    scene::Scene,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        ai::runtime::Others,
        collision::{Collision, DISABLED, WEAPON},
        control::{self, *},
        inventory::Item,
        physics::Terrain,
        rig::{Contacts, Grip, Rig, WeaponMode},
    },
};
use serde::{Deserialize, Serialize};

mod combat;
mod effects;
mod injury;
mod items;
pub(super) use injury::credit;
mod locomotion;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Controls {
    pub camera_actor: usize,
    pub previous_camera: i16,
    pub ledge_check: i16,
    pub wall_allowed: bool,
    pub story_triggered: bool,
    pub story_damage: i32,
    pub unlocked_items: [bool; 20],
    pub stealth: Option<StealthSession>,
    pub focus: Option<usize>,
    pub armor: bool,
    pub drain_attack: bool,
    pub drain_uses: u8,
    pub death_music: bool,
    pub transition_ready: bool,
    #[serde(default)]
    pub hud: crate::tenchu2::hud::Hud,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            camera_actor: 0,
            previous_camera: 0,
            ledge_check: 0,
            wall_allowed: true,
            story_triggered: false,
            story_damage: 0,
            unlocked_items: [false; 20],
            stealth: None,
            focus: None,
            armor: false,
            drain_attack: false,
            drain_uses: 0,
            death_music: true,
            transition_ready: true,
            hud: Default::default(),
        }
    }
}

impl Controls {
    pub(super) fn observe(&mut self, index: usize, actor: &Actor, event: Event) {
        match event {
            Event::ShowHealth => self.hud.show_health(index, actor.kind),
            Event::Pickup(item) => self.hud.pickup(item),
            _ => {}
        }
    }

    pub(super) fn select_item(
        &mut self,
        inventory: &mut crate::tenchu2::inventory::Inventory,
        previous: bool,
        player: bool,
    ) {
        if player {
            self.hud.select(inventory, previous);
        } else {
            inventory.select(previous);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Voice(u8),
    Sound(u8),
    SoundVolume {
        id: u8,
        volume: u8,
    },
    ItemSound(ItemSound),
    Splash(u8),
    Dust,
    DustTrail,
    Rumble {
        action: crate::effects::RumbleAction,
        kind: u8,
        strength: i16,
    },
    DrowningRumble(i16),
    Landing,
    WallJump,
    ShowHealth,
    Pickup(usize),
    HiddenMission,
    HiddenCarry,
    Throw {
        bone: usize,
        center: Position,
        direction: Position,
    },
    ThrowItem {
        item: Item,
        direction: Position,
    },
    NockArrow,
    Fire(Projectile),
    Combat(CombatCue),
    StealthTint,
    WaterRipple {
        position: Position,
        size: i16,
    },
    Blood {
        position: Position,
        angle: i16,
    },
    Bubbles,
    HeldItem(Option<u8>),
    DropHeldItem,
    Sparks {
        position: Position,
        parried: bool,
    },
    Burn,
    Heal,
    DeathMusic,
    UseItem {
        item: Item,
        direction: Option<Position>,
    },
    Medicine(u8),
    SoundPosition {
        id: u8,
        position: Position,
    },
    SoundAt {
        id: u8,
        positional: bool,
    },
}

pub struct EventSource<'a> {
    pub listener: crate::tenchu2::audio::Listener,
    pub voice: u8,
    pub index: usize,
    pub actor: &'a Actor,
    pub rig: &'a Rig,
    pub scene: &'a mut Scene,
}

pub trait Events: crate::tenchu2::environment::Events {
    fn emit(&mut self, source: EventSource<'_>, event: Event, random: &mut Random) -> Result<()>;

    /// Observe source blood emission at a scene joint without consuming gameplay randomness.
    fn blood(&mut self, _actor: usize, _joint: usize, _position: Position) {}

    fn melee_hit(&mut self, _attacker: usize) {}
}

impl World {
    pub fn control_actor(
        &mut self,
        index: usize,
        assets: &mut Assets,
        events: &mut impl Events,
    ) -> Result<()> {
        let (before, tail) = self.actors.split_at_mut(index);
        let (agent, after) = tail.split_first_mut().ok_or("Missing Tenchu 2 actor")?;
        let mut input = ControlInput {
            held: agent.actor.pad.held,
            player: index == self.player,
            camera_actor: index == self.controls.camera_actor,
            camera: self.camera.mode,
            script: self.script,
            tick: self.tick as i32,
            water: self.water.clone(),
            model_height: self.instances[index].model_height,
        };
        let mut request = Request {
            action: agent.actor.motion.action,
            velocity: Velocity::Keep,
        };
        let mut driver = Driver {
            context: Context {
                environment: &mut self.environment,
                view_distance: self.view_distance,
                items: &mut self.items,
                effects: &mut self.effects,
                effect_rules: &assets.effect_rules,
                math: assets.resources.math.clone(),
                hidden: self.hidden_scene,
                water: self.water.clone(),
                index,
                player: self.player,
                hero: assets.hero,
                map: self.map,
                sequence: self.sequence,
                difficulty: self.difficulty,
                home: agent.behavior.home,
                enabled: agent.behavior.programs.idle != crate::tenchu2::ai::Program::Disabled,
                others: Others { before, after },
                instances: &mut self.instances,
                scene: &mut self.scene,
                collision: &mut self.collision,
                controls: &mut self.controls,
                camera: &mut self.camera,
                detections: &mut self.ai.detections,
                tracking: &mut self.tracking,
                spline: &assets.spline,
                stats: &mut self.statistics,
                alert: &mut self.ai.global.perception.alert,
                rules: &assets.combat_rules,
                script: &mut self.script,
                events,
            },
            terrain: Terrain {
                map: &mut self.terrain,
                rules: &assets.terrain_rules,
                cursor: &mut self.cursor,
                scratch: &mut self.probe,
                water: &mut self.water,
                options: self.terrain_options,
            },
            random: &mut self.random,
        };
        agent.actor.update_controls(
            &mut request,
            &mut input,
            &mut assets.resources,
            &mut driver,
        )?;
        self.terrain_options = driver.terrain.options;
        driver.context.camera.mode = input.camera;
        driver.context.sync_contacts(&agent.actor);
        Ok(())
    }
}

struct Context<'a, E> {
    environment: &'a mut crate::tenchu2::environment::Environment,
    view_distance: i32,
    items: &'a mut crate::tenchu2::items::Items,
    effects: &'a mut crate::tenchu2::effects::Effects,
    effect_rules: &'a tenchu_assets::tenchu2::effects::Rules,
    math: std::sync::Arc<crate::math::Math>,
    hidden: bool,
    water: crate::tenchu2::physics::Water,
    index: usize,
    player: usize,
    hero: usize,
    map: usize,
    sequence: u16,
    difficulty: u8,
    enabled: bool,
    home: Position,
    others: Others<'a>,
    instances: &'a mut [Instance],
    scene: &'a mut Scene,
    collision: &'a mut Collision,
    controls: &'a mut Controls,
    camera: &'a mut crate::tenchu2::camera::Camera,
    detections: &'a mut u8,
    tracking: &'a mut Option<usize>,
    spline: &'a [[i16; 4]; 33],
    stats: &'a mut super::Statistics,
    alert: &'a mut crate::tenchu2::ai::status::Alert,
    rules: &'a tenchu_assets::tenchu2::combat::Rules,
    script: &'a mut u16,
    events: &'a mut E,
}

struct Driver<'a, E> {
    context: Context<'a, E>,
    terrain: Terrain<'a>,
    random: &'a mut Random,
}

fn listener(
    others: &Others<'_>,
    camera_actor: usize,
    actor: &Actor,
    view_distance: i32,
) -> Result<crate::tenchu2::audio::Listener> {
    let actor = others
        .get(camera_actor, actor)
        .ok_or("Missing Tenchu 2 sound listener")?;
    Ok(crate::tenchu2::audio::Listener {
        actor: camera_actor,
        position: actor.position,
        yaw: actor.rotation.y,
        distance: view_distance >> 4,
    })
}

impl<E: Events> Context<'_, E> {
    fn emit(&mut self, actor: &Actor, event: Event, random: &mut Random) -> Result<()> {
        self.controls.observe(self.index, actor, event);
        if let Event::HeldItem(model) = event {
            self.items.held = model;
            return Ok(());
        }
        if self.particle(actor, event, random)? {
            return Ok(());
        }
        if self.item_event(actor, event, random)? {
            return Ok(());
        }
        Events::emit(
            self.events,
            EventSource {
                listener: listener(
                    &self.others,
                    self.controls.camera_actor,
                    actor,
                    self.view_distance,
                )?,
                voice: self.instances[self.index].voice,
                index: self.index,
                actor,
                rig: &self.instances[self.index].rig,
                scene: self.scene,
            },
            event,
            random,
        )
    }

    fn root(&mut self, actor: &Actor) -> Result<Position> {
        self.instances[self.index]
            .rig
            .root_position(actor, self.scene)
    }

    fn publish_root(&mut self, actor: &Actor, resources: &Resources) {
        self.scene.update(
            self.instances[self.index].rig.root,
            Matrix {
                rotation: resources.math.rotate_yxz(actor.rotation.to_array()),
                translation: actor.position,
            },
        );
    }

    fn recover(&mut self, actor: &mut Actor, request: &mut Request, held: u16) -> Result<()> {
        request.action = actor.normal_motion(held, |actor| self.root(actor))?;
        request.velocity = Velocity::Motion;
        Ok(())
    }

    fn sync_contacts(&mut self, actor: &Actor) {
        for (bone, &joint) in self.instances[self.index].rig.joints.iter().enumerate() {
            if let Some(slot) = self.collision.get_mut(joint)
                && slot.flags & WEAPON != 0
                && actor.attack.contacts & 1u32.wrapping_shl(bone as u32) == 0
            {
                slot.flags |= DISABLED;
            }
        }
    }

    fn nearest(
        &self,
        actor: &Actor,
        distance: i32,
        enemies_only: bool,
        resources: &Resources,
    ) -> Option<usize> {
        control::find_target(
            self.others.len(),
            |i| self.others.get(i, actor),
            self.index,
            Some(self.player),
            TargetQuery::Nearby {
                distance,
                enemies_only,
            },
            &resources.math,
        )
    }

    fn weapon(&mut self, actor: &mut Actor, mode: WeaponMode, resources: &Resources) -> Result<()> {
        actor.set_weapon(
            &mut self.instances[self.index].rig,
            self.index,
            self.collision,
            resources,
            mode,
        )
    }

    fn grip(
        &mut self,
        actor: &mut Actor,
        slot: usize,
        spare: usize,
        grip: Grip,
        random: &mut Random,
    ) -> Result<()> {
        if let Some(sound) =
            actor.swap_attachments(&mut self.instances[self.index].rig, slot, spare, grip)?
        {
            self.emit(actor, Event::Voice(sound), random)?;
        }
        Ok(())
    }

    fn item(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        held: u16,
        scripted: bool,
        resources: &Resources,
        random: &mut Random,
    ) -> Result<()> {
        if self.index == self.player && self.controls.hud.bandage_selected() {
            if held & !actor.pad.previous & crate::tenchu2::input::ITEM != 0 {
                if !scripted && !matches!(self.camera.mode, 6 | 20) && actor.use_bandage() {
                    self.emit(actor, Event::Sound(5), random)?;
                } else {
                    self.emit(actor, Event::ItemSound(ItemSound::Unavailable), random)?;
                }
            }
            return Ok(());
        }
        let mut sound = None;
        actor.item_input(
            request,
            &ItemInput {
                held,
                player: self.index == self.player,
                scripted,
            },
            resources,
            |cue| {
                sound = Some(cue);
                Ok(())
            },
        )?;
        if let Some(sound) = sound {
            self.emit(actor, Event::ItemSound(sound), random)?;
        }
        Ok(())
    }
}

impl<E: Events> ControlDriver for Driver<'_, E> {
    fn motion(
        &mut self,
        actor: &mut Actor,
        request: &Request,
        resources: &Resources,
    ) -> Result<()> {
        actor.change_motion(request.action, request.velocity, resources)?;
        self.context.sync_contacts(actor);
        Ok(())
    }

    fn cue(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        cue: ControlCue,
        resources: &Resources,
    ) -> Result<()> {
        let c = &mut self.context;
        match cue {
            ControlCue::Recover => c.recover(actor, request, input.held),
            ControlCue::Voice(id) => c.emit(actor, Event::Voice(id), self.random),
            ControlCue::Splash(id) => c.emit(actor, Event::Splash(id), self.random),
            ControlCue::DrowningRumble(strength) => {
                c.emit(actor, Event::DrowningRumble(strength), self.random)
            }
            ControlCue::SelectItem { previous } => {
                c.controls
                    .select_item(&mut actor.inventory, previous, c.index == c.player);
                Ok(())
            }
            ControlCue::ItemSound => c.emit(actor, Event::Sound(5), self.random),
            ControlCue::Ledge => self
                .ledge(actor, request, input, resources, true)
                .map(|_| ()),
            ControlCue::Damage => self.damage(actor, request, input, resources),
        }
    }

    fn state(
        &mut self,
        actor: &mut Actor,
        request: &mut Request,
        input: &mut ControlInput,
        resources: &mut Resources,
    ) -> Result<()> {
        match actor.state {
            State::Normal | State::Ready => self.standing(actor, request, input, resources),
            State::Crouched => self.crouched(actor, request, input, resources),
            State::Wall => self.wall(actor, request, input, resources),
            State::Ledge => self
                .ledge(actor, request, input, resources, false)
                .map(|_| ()),
            State::Jumping => self.jumping(actor, request, input, resources),
            State::Special => self.special(actor, request, input, resources),
            State::Attack => self.attack(actor, request, input, resources),
            State::Hurt => self.hurt(actor, request, input, resources),
            State::Dead => self.context.dead(
                actor,
                request,
                input,
                &mut self.terrain,
                resources,
                self.random,
            ),
            State::Swimming => self.swimming(actor, request, input, resources),
            State::Item => self.item_control(actor, request, input, resources),
        }
    }
}
