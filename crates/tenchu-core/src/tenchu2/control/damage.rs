use super::Request;
use crate::{
    Result,
    math::{Position, Vector3},
    random::Random,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        ai::direction,
        input::DOWN,
        physics::Terrain,
        terrain::passage,
    },
};
use tenchu_assets::tenchu2::Edition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageTarget {
    Victim,
    Attacker,
}

pub struct DamageAttacker {
    pub actor: usize,
    pub player: bool,
    pub position: Position,
    pub power: u8,
    pub health: i16,
    pub attacking: bool,
    pub recoil: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Weapon,
    Object(u8),
}

pub struct DamageInput {
    pub actor: usize,
    pub player: bool,
    pub controls_enabled: bool,
    pub held: u16,
    pub map: i16,
    pub difficulty: i16,
    pub armor: bool,
    pub drain_attack: bool,
    pub drain_uses: u8,
    pub story_triggered: bool,
    pub story_damage: i32,
    pub camera: i16,
    pub focus: Option<usize>,
    pub death_music: bool,
    pub hit: Hit,
    pub contact: Position,
    pub body: Position,
    pub contact_delta: Vector3<i16>,
    pub hit_disabled: bool,
    pub attacker: Option<DamageAttacker>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCue {
    DropHeldItem,
    HideHeldItem,
    ReleaseGrip {
        first: usize,
        second: usize,
    },
    FinishContacts,
    Root,
    ContactDelta,
    AdvanceMotion,
    Sparks {
        position: Position,
        parried: bool,
    },
    Voice {
        target: DamageTarget,
        id: u8,
    },
    Rumble {
        target: DamageTarget,
        kind: u8,
        strength: u8,
    },
    Burn,
    Sound(u8),
    Alert,
    KillCredit,
    Heal,
    Resurrection,
    ShowHealth,
    DeathMusic,
    Blood {
        joint: usize,
        angle: i16,
    },
}

type Cue<'a> = dyn FnMut(&mut Actor, &mut Request, &mut DamageInput, &mut Random, DamageCue) -> Result<()>
    + 'a;

struct Damage<'a, 't> {
    actor: &'a mut Actor,
    request: &'a mut Request,
    input: &'a mut DamageInput,
    terrain: &'a mut Terrain<'t>,
    resources: &'a Resources,
    random: &'a mut Random,
    cue: &'a mut Cue<'a>,
}

struct Impact {
    power: i16,
    angle: i16,
    knockdown: bool,
    blood: bool,
}

impl Actor {
    pub fn apply_damage(
        &mut self,
        request: &mut Request,
        input: &mut DamageInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        random: &mut Random,
        mut cue: impl FnMut(
            &mut Actor,
            &mut Request,
            &mut DamageInput,
            &mut Random,
            DamageCue,
        ) -> Result<()>,
    ) -> Result<()> {
        Damage {
            actor: self,
            request,
            input,
            terrain,
            resources,
            random,
            cue: &mut cue,
        }
        .apply()
    }
}

impl Damage<'_, '_> {
    fn emit(&mut self, cue: DamageCue) -> Result<()> {
        (self.cue)(self.actor, self.request, self.input, self.random, cue)
    }

    fn pal(&self) -> bool {
        self.resources.registry.edition == Edition::Europe
    }

    fn request(&mut self, action: i16, velocity: Velocity) {
        *self.request = Request { action, velocity };
    }

    fn change(&mut self) -> Result<bool> {
        if self.actor.motion.action != self.request.action
            && self.actor.state == State::Attack
            && self.actor.motion.loops < 1
        {
            self.emit(DamageCue::FinishContacts)?;
        }
        Ok(self
            .actor
            .change_motion(self.request.action, self.request.velocity, self.resources)?
            .is_some())
    }

    fn voice(&mut self, target: DamageTarget, id: u8) -> Result<()> {
        self.emit(DamageCue::Voice { target, id })
    }

    fn rumble(&mut self, target: DamageTarget, kind: u8, strength: u8) -> Result<()> {
        self.emit(DamageCue::Rumble {
            target,
            kind,
            strength,
        })
    }

    fn player_or_attacker(&self) -> DamageTarget {
        if self.input.player {
            DamageTarget::Victim
        } else {
            DamageTarget::Attacker
        }
    }

    fn animal(&self) -> bool {
        self.actor.kind & 0xf0 == 0x70 || matches!(self.actor.kind, 0xe5 | 0x1a)
    }

    fn angle(&self, delta: Position) -> Result<i16> {
        direction(
            &self.resources.math,
            delta.x,
            delta.z,
            self.actor.rotation.y,
        )
    }

    fn apply(&mut self) -> Result<()> {
        if (0xa23..=0xa25).contains(&self.request.action)
            || self.request.action == 0xa26 && (self.pal() || self.input.player)
            || self.actor.health <= 0
            || self.actor.health == i16::MAX
        {
            return Ok(());
        }
        if self.input.player {
            if !self.input.controls_enabled {
                return Ok(());
            }
            self.actor.target = None;
            self.input.camera = 0;
            if self.request.action == 0x809 {
                let duration = i32::from(
                    self.resources
                        .clips
                        .get(&self.actor.motion.clip.ok_or("No Tenchu 2 damage motion")?)
                        .ok_or("Missing Tenchu 2 damage motion")?
                        .duration,
                );
                let used_at = if self.pal() {
                    duration / 2
                } else {
                    duration * 2 / 3
                };
                if i32::from(self.actor.motion.frame) < used_at {
                    self.emit(DamageCue::DropHeldItem)?;
                }
                self.emit(DamageCue::HideHeldItem)?;
            } else if matches!(self.request.action, 0x536 | 0x80b) {
                self.emit(DamageCue::HideHeldItem)?;
            }
        }
        self.actor.motion.mask = u32::MAX;
        self.actor.rotation.x = 0;
        self.actor.rotation.z = 0;
        if self.actor.flags & 0x200000 != 0 {
            let slots = match self.actor.kind {
                2 | 0xed | 0xf5 => Some((0, 3)),
                0xf1 => Some((1, 4)),
                _ => None,
            };
            if let Some((first, second)) = slots {
                self.emit(DamageCue::ReleaseGrip { first, second })?;
            }
        }
        self.emit(DamageCue::FinishContacts)?;
        let impact = match self.input.hit {
            Hit::Weapon => self.weapon()?,
            Hit::Object(kind) => self.object(kind)?,
        };
        if let Some(impact) = impact {
            self.resolve(impact)?;
        }
        Ok(())
    }

    fn weapon(&mut self) -> Result<Option<Impact>> {
        self.input.focus = Some(self.input.actor);
        let attacker = self
            .input
            .attacker
            .as_ref()
            .ok_or("Tenchu 2 weapon has no owner")?;
        let mut step = attacker
            .position
            .wrapping_sub(self.actor.position)
            .map(|v| v as i16);
        let mut count = 1i16;
        while step.into_iter().any(|v| i32::from(v).abs() > 100) {
            step = step.map(|v| v >> 1);
            count *= 2;
        }
        let mut start = self.actor.position;
        start.y = start.y.wrapping_sub(i32::from(self.actor.height / 2));
        if passage(
            self.terrain.map,
            self.terrain.cursor,
            start,
            step,
            count,
            self.resources.registry.edition,
            &self.resources.math,
        )?
        .is_some()
        {
            return Ok(None);
        }
        self.input.hit_disabled = true;
        self.emit(DamageCue::Root)?;
        self.actor.rotation.y = self
            .actor
            .rotation
            .y
            .wrapping_add(self.actor.motion.pose.rotation[0][1]);
        let attacker = self
            .input
            .attacker
            .as_ref()
            .ok_or("Tenchu 2 weapon owner removed")?;
        let angle = self.angle(attacker.position.wrapping_sub(self.actor.position))?;
        let mut power = i16::from(attacker.power);
        if power != 0 {
            if self.input.player && self.input.armor {
                power /= 3;
            } else if self.input.difficulty != 3 {
                power = (i32::from(power) - i32::from(power) * i32::from(self.actor.skill % 5) / 10)
                    as i16;
            }
        }
        let attacker = self.input.attacker.as_mut().unwrap();
        if attacker.attacking {
            attacker.recoil = !(power / 3);
        }
        let mut parried = matches!(self.request.action, 0x916 | 0x104)
            || !self.pal() && self.request.action == 0x501;
        if !parried
            && self.actor.weapon_drawn()
            && (self.request.action == 0x110
                || !self.input.player && self.input.held & DOWN != 0
                || self.actor.skill > 4
                    && self.random.next15() % if self.pal() { 3 } else { 5 } == 0)
        {
            parried = true;
        }
        if !self.input.player && self.random.next15() & 3 == 0 {
            parried = false;
        }
        let cost = if self.pal() { 2 } else { 5 };
        let attacker = self.input.attacker.as_mut().unwrap();
        if attacker.player && attacker.health > cost && self.input.drain_attack {
            parried = false;
            attacker.health -= cost;
            power = if cost == 2 { 100 } else { 1000 };
            self.input.drain_uses = self.input.drain_uses.wrapping_add(1);
        }
        let body = Position::new(self.input.body.x, self.input.contact.y, self.input.body.z);
        if parried
            && i32::from(angle).abs() < if self.input.player { 1280 } else { 768 }
            && self.parry(power, angle, body, self.pal())?
        {
            self.rumble(self.player_or_attacker(), 2, 2)?;
            return Ok(None);
        }
        if self.input.player {
            if self.input.map < 2 {
                power /= 2;
            }
        } else if self.input.attacker.as_ref().unwrap().player {
            if self.actor.is_stealth_vulnerable() {
                power = -2;
            }
        } else {
            power /= 3;
        }
        let point = body.wrapping_add(self.input.contact).map(|v| v / 2);
        self.emit(DamageCue::Sparks {
            position: point,
            parried: false,
        })?;
        self.voice(DamageTarget::Attacker, if power < 20 { 4 } else { 5 })?;
        self.rumble(self.player_or_attacker(), 4, 1)?;
        Ok(Some(Impact {
            power,
            angle,
            knockdown: false,
            blood: true,
        }))
    }

    fn parry(
        &mut self,
        power: i16,
        angle: i16,
        position: Position,
        pal_speed: bool,
    ) -> Result<bool> {
        self.request(0x916, Velocity::Keep);
        if !self.change()? {
            return Ok(false);
        }
        self.emit(DamageCue::AdvanceMotion)?;
        self.actor.motion.loops = power.wrapping_add(8).wrapping_neg();
        self.actor.rotation.y = self.actor.rotation.y.wrapping_add(angle);
        let mut speed = -(i32::from(power) * 5 / 2 + 80);
        if pal_speed {
            speed = speed * 6 / 5;
        }
        self.actor
            .set_horizontal_velocity(speed as i16, 0, &self.resources.math);
        self.emit(DamageCue::Sparks {
            position,
            parried: true,
        })?;
        self.voice(DamageTarget::Victim, 3)?;
        Ok(true)
    }

    fn object(&mut self, kind: u8) -> Result<Option<Impact>> {
        if self.actor.state == State::Hurt
            && self.request.action != 0xa13
            && self.actor.motion.frame < 4
        {
            return Ok(None);
        }
        let mut power = 0i16;
        let mut knockdown = false;
        let mut blood = true;
        let mut effect_only = false;
        match kind {
            4 => power = 15,
            5 if self.request.action == 0xa12 => effect_only = true,
            5 => {
                power = 3;
                blood = false;
            }
            6 | 7 | 9 => {
                power = match kind {
                    6 => 30,
                    7 => 35,
                    _ => 25,
                };
                knockdown = true;
                self.emit(DamageCue::Burn)?;
                if kind == 7
                    && !self.pal()
                    && self.input.map == 19
                    && self.actor.position.x / 1000 == -24
                    && self.actor.position.z / 1000 == -29
                {
                    if self.input.story_triggered {
                        self.actor.inventory.counts[1..].fill(0);
                        self.actor.inventory.resurrections = 0;
                        power = 55;
                    } else {
                        self.input.story_triggered = true;
                        for item in 1..=20 {
                            let count = if item == 20 {
                                &mut self.actor.inventory.resurrections
                            } else {
                                &mut self.actor.inventory.counts[item]
                            };
                            *count = count.wrapping_mul(2);
                            self.emit(DamageCue::Sound(1))?;
                        }
                    }
                }
            }
            8 => {
                self.actor.flags |= 0x100;
                if self.actor.is_stealth_vulnerable() {
                    power = -1;
                } else {
                    power = 1;
                    blood = false;
                }
            }
            10 | 25 | 28 => {
                effect_only = true;
                self.request(0xa10, Velocity::Motion);
                self.actor.flags |= if kind == 28 { 0x100 } else { 0x200 };
            }
            14 => {
                if self.request.action == 0x70c {
                    return Ok(None);
                }
                effect_only = true;
                if self.actor.flags & 0x800 == 0 {
                    self.request(0xa15, Velocity::Motion);
                    self.actor.flags |= 0x400;
                }
            }
            18 => {
                if self.request.action == 0x70c {
                    return Ok(None);
                }
                effect_only = true;
                self.request(0xa13, Velocity::Motion);
                self.actor.flags = self.actor.flags & !0x400 | 0x800;
            }
            22 => power = 5,
            24 => power = 10,
            26 | 33 => {
                if kind == 26 {
                    self.actor.flags |= 0x100;
                }
                knockdown = true;
                if !self.input.player && self.actor.flags & 0x100040 != 0x100040 {
                    return Ok(None);
                }
                power = 5;
            }
            27 => {
                self.actor.flags |= 0x100;
                power = 10;
            }
            29 => power = 20,
            _ => return Ok(None),
        }
        if power == 0
            && (self.actor.state == State::Hurt
                || self.actor.state == State::Swimming && self.actor.motion.action > 0x50f)
        {
            self.request(self.actor.motion.action, Velocity::Keep);
            return Ok(None);
        }
        self.rumble(DamageTarget::Victim, 4, 1)?;
        if effect_only {
            if self.animal() {
                self.request(0xa00, Velocity::Motion);
            }
            self.change()?;
            self.random_voice()?;
            return Ok(None);
        }
        self.emit(DamageCue::ContactDelta)?;
        let delta = self.input.attacker.as_ref().map_or_else(
            || self.input.contact_delta.map(i32::from),
            |a| a.position.wrapping_sub(self.actor.position),
        );
        let angle = self.angle(delta)?;
        self.emit(DamageCue::Root)?;
        self.actor.rotation.y = self
            .actor
            .rotation
            .y
            .wrapping_add(self.actor.motion.pose.rotation[0][1]);
        if !self.pal() {
            self.actor.flags &= !0x200;
        }
        if power > 0 && self.actor.skill != 0 {
            power /= 2;
        }
        let mut parried = kind == 4
            && self.actor.skill > 4
            && self.random.next15() % 3 == 0
            && i32::from(angle).abs() < 768
            && self.actor.flags & 0x1000 == 0;
        if !parried
            && !self.pal()
            && self.actor.state == State::Ready
            && (matches!(self.actor.kind, 0xf2 | 0xf5 | 0xf6 | 0xe7)
                || self.input.player && self.input.armor)
        {
            if kind == 22 {
                parried = true;
            } else if kind == 9 {
                power /= 2;
                knockdown = false;
            }
        }
        if parried && self.parry(power, angle, self.input.contact, false)? {
            return Ok(None);
        }
        Ok(Some(Impact {
            power,
            angle,
            knockdown,
            blood,
        }))
    }

    fn random_voice(&mut self) -> Result<()> {
        let id = if self.random.next15() & 1 != 0 {
            13
        } else {
            14
        };
        self.voice(DamageTarget::Victim, id)
    }

    fn resurrect(&mut self) -> Result<bool> {
        if self.actor.health != 0 || self.actor.inventory.resurrections == 0 {
            return Ok(false);
        }
        self.actor.inventory.resurrections -= 1;
        if self.terrain.water.depth.is_some() {
            self.emit(DamageCue::Heal)?;
            self.actor.health = self.actor.max_health;
        } else {
            self.emit(DamageCue::Resurrection)?;
        }
        Ok(true)
    }

    fn resolve(&mut self, impact: Impact) -> Result<()> {
        let Impact {
            mut power,
            angle,
            knockdown,
            blood,
        } = impact;
        if power > 0 {
            self.emit(DamageCue::Alert)?;
        }
        self.actor.health = self.actor.health.wrapping_sub(power);
        if self.actor.health <= 0 || power < 0 {
            self.actor.health = 0;
            if power < 0 {
                power = if power == -1 {
                    0
                } else {
                    i16::from(
                        self.input
                            .attacker
                            .as_ref()
                            .ok_or("Tenchu 2 fatal hit has no owner")?
                            .power,
                    )
                };
            }
            self.emit(DamageCue::KillCredit)?;
        }
        if knockdown
            || power > 20
            || matches!(self.request.action, 0x70c | 0xa13)
            || self.actor.terrain.height > i32::from(self.actor.height)
        {
            if self.resurrect()? {
                return Ok(());
            }
            if self.request.action != 0xa13 {
                self.voice(
                    DamageTarget::Victim,
                    if self.actor.health != 0 { 14 } else { 15 },
                )?;
            }
            let speed = if self.pal() { 84 } else { 100 };
            let speed = if i32::from(angle).abs() < 1024 {
                self.request(0xa21, Velocity::Keep);
                -speed
            } else {
                self.request(0xa22, Velocity::Keep);
                self.actor.rotation.y = self.actor.rotation.y.wrapping_add(2048);
                speed
            };
            self.actor.rotation.y = self.actor.rotation.y.wrapping_add(angle);
            self.actor
                .set_horizontal_velocity(speed, 0, &self.resources.math);
            self.rumble(DamageTarget::Victim, 3, 4)?;
        } else if self.actor.health == 0 {
            if self.input.hit == Hit::Weapon {
                self.rumble(self.player_or_attacker(), 4, 2)?;
            }
            if self.resurrect()? {
                return Ok(());
            }
            let id = if power == 0 {
                self.request(0xb06, Velocity::Motion);
                13
            } else {
                self.request(0xb00, Velocity::Motion);
                if !self.change()? {
                    let action = 0xb01 + (self.random.next15() % 5) as i16;
                    self.request(action, Velocity::Motion);
                }
                15
            };
            self.voice(DamageTarget::Victim, id)?;
        } else {
            if i32::from(angle).abs() < 768 {
                self.request(0xa00, Velocity::Motion);
                self.actor.rotation.y = self.actor.rotation.y.wrapping_add(angle);
            } else if i32::from(angle).abs() > 1792 {
                self.request(0xa01, Velocity::Motion);
                self.actor.rotation.y =
                    self.actor.rotation.y.wrapping_add(2048).wrapping_add(angle);
            } else {
                self.request(if angle > 0 { 0xa02 } else { 0xa03 }, Velocity::Motion);
            }
            if self.input.hit == Hit::Weapon {
                self.rumble(DamageTarget::Victim, 4, 1)?;
                self.request.velocity = Velocity::Keep;
                let mut speed = i32::from(power) * 5 / 2 + 80;
                if self.pal() {
                    speed = speed * 6 / 5;
                }
                let speed = speed as i16;
                let (forward, side) = match self.request.action {
                    0xa00 => (speed.wrapping_neg(), 0),
                    0xa01 => (speed, 0),
                    0xa02 => (0, speed),
                    _ => (0, speed.wrapping_neg()),
                };
                self.actor
                    .set_horizontal_velocity(forward, side, &self.resources.math);
            } else if self.input.hit == Hit::Object(5) {
                self.request(0xa12, Velocity::Motion);
            }
            self.random_voice()?;
        }
        if !self.input.player {
            self.emit(DamageCue::ShowHealth)?;
            if !self.pal() {
                self.input.story_damage = self.input.story_damage.wrapping_add(if power < 1 {
                    1000
                } else {
                    i32::from(power) * 10
                });
            }
        }
        if self.actor.state == State::Swimming
            && self.actor.motion.action > 0x50f
            && self.actor.motion.action != 0x520
        {
            self.request(
                if self.actor.health == 0 {
                    0xb08
                } else if self.actor.terrain.material & 0xf00 == 0x300 {
                    0xa04
                } else {
                    0xa16
                },
                Velocity::Motion,
            );
        }
        if self.animal() || self.actor.kind == 0x6b {
            self.request(
                if self.actor.health != 0 {
                    0xa00
                } else if self.actor.kind == 0x75 {
                    0xb0a
                } else {
                    0xb00
                },
                Velocity::Motion,
            );
        }
        if self.input.player && self.actor.health == 0 && self.input.death_music {
            self.emit(DamageCue::DeathMusic)?;
        }
        if blood && self.input.map != 12 {
            for offset in [0x840i16, 0x7c0] {
                self.emit(DamageCue::Blood {
                    joint: 0,
                    angle: self
                        .actor
                        .rotation
                        .y
                        .wrapping_add(angle)
                        .wrapping_add(offset),
                })?;
            }
        }
        self.actor.motion.action = -1;
        self.change()?;
        Ok(())
    }
}
