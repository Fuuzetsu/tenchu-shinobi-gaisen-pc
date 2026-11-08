use super::Request;
use crate::{
    Result,
    math::Position,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        input::{DOWN, LEFT, RIGHT, UP},
        physics::Terrain,
        terrain::{Query, Surface, level},
    },
};
use tenchu_assets::tenchu2::Edition;

pub struct LedgeInput {
    pub held: u16,
    pub player: bool,
    pub tick: i32,
    pub check: i16,
    pub grapple_camera: bool,
    pub map: i16,
    pub story_triggered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeCue {
    Voice(u8),
    Grabbed,
    Recover,
    ReleaseCamera,
    HiddenMission,
}

impl Actor {
    pub fn enter_ledge(
        &mut self,
        request: &mut Request,
        input: &mut LedgeInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, u16, LedgeCue) -> Result<()>,
    ) -> Result<bool> {
        if self.health < 1
            || self.state == State::Hurt
            || matches!(request.action, 0x602 | 0x605)
            || self.terrain.height < i32::from(self.height)
            || self.kind & 0xf0 == 0x70
            || matches!(self.kind, 0x11 | 0x1a)
        {
            return Ok(false);
        }
        let mut point = self.position;
        point.y = point.y.wrapping_sub(2300);
        if level(terrain.map, terrain.cursor, point, Query::Uncached)?
            .is_none_or(|floor| floor <= self.position.y)
        {
            return Ok(false);
        }
        let width = self.width / 4;
        let rounded = self.rotation.y.wrapping_add(256);
        let mut angle = rounded & 0xe00;
        let reach = if self.height < 1600 { 1800 } else { 2000 };
        let eligible = |floor: i32, material: u16| {
            floor
                .wrapping_sub(self.position.y.wrapping_sub(reach))
                .wrapping_abs()
                <= 400
                && material & 0xf00 == 0
                && !terrain.rules.blocked[usize::from(self.terrain.material >> 12 & 3)]
                    [usize::from(material >> 12 & 3)]
        };
        if self.state == State::Ledge {
            let [x, _, z] = resources.math.velocity(angle, width.wrapping_mul(5), 0);
            point.x = point.x.wrapping_add(i32::from(x));
            point.z = point.z.wrapping_add(i32::from(z));
            if let Some(floor) = level(terrain.map, terrain.cursor, point, Query::Uncached)?
                && eligible(floor, terrain.cursor.material)
            {
                self.position.y = floor.wrapping_add(i32::from(self.height));
                return Ok(true);
            }
            return Ok(false);
        }
        let sample = |angle: i16, terrain: &mut Terrain<'_>| -> Result<LedgeSamples> {
            let mut samples = LedgeSamples::default();
            for (i, offset) in [-448i16, 0, 448].into_iter().enumerate() {
                let [x, _, z] = resources.math.velocity(
                    angle.wrapping_add(offset),
                    width.wrapping_mul(if i == 1 { 7 } else { 9 }),
                    0,
                );
                let at = Position::new(
                    point.x.wrapping_add(i32::from(x)),
                    point.y,
                    point.z.wrapping_add(i32::from(z)),
                );
                samples.floors[i] = level(terrain.map, terrain.cursor, at, Query::Uncached)?;
                if samples.floors[i].is_some_and(|floor| eligible(floor, terrain.cursor.material)) {
                    samples.count += 1;
                }
                if i == 1 {
                    samples.center = terrain.cursor.current;
                }
            }
            Ok(samples)
        };
        let mut samples = sample(angle, terrain)?;
        if samples.count != 3 && rounded & 0x200 != 0 {
            angle = self.rotation.y.wrapping_add(512) & 0xc00;
            samples = sample(angle, terrain)?;
            samples.center = None;
        } else if samples.count == 2
            && rounded & 0x200 == 0
            && (samples.floors[0].is_some_and(|y| y > self.position.y)
                || samples.floors[2].is_some_and(|y| y > self.position.y))
            && let Some(surface) = samples.center
        {
            let node = &terrain.map.cells[surface.cell].nodes[surface.node];
            let [x1, z1, x2, z2] = node.bounds.map(i32::from);
            if node.division == u16::MAX && x2 - x1 > 199 && z2 - z1 > 199 {
                samples.count = 3;
            }
        }
        if samples.floors[1].is_none() || samples.count != 3 {
            return Ok(false);
        }
        if let Some(surface) = samples.center {
            let slope = terrain.map.cells[surface.cell].nodes[surface.node].attributes & 0xc000;
            if (slope == 0x8000 && angle & 0x400 == 0) || (slope == 0xc000 && angle & 0x400 != 0) {
                return Ok(false);
            }
        } else if samples.floors[0] != samples.floors[2] {
            return Ok(false);
        }
        for step in 0..15 {
            let [x, _, z] = resources
                .math
                .velocity(angle, width.wrapping_mul(step + 2), 0);
            let at = Position::new(
                point.x.wrapping_add(i32::from(x)),
                point.y,
                point.z.wrapping_add(i32::from(z)),
            );
            let Some(floor) = level(terrain.map, terrain.cursor, at, Query::Uncached)? else {
                return Ok(false);
            };
            if floor >= self.position.y {
                continue;
            }
            if floor.wrapping_add(i32::from(self.height)) >= self.position.y {
                return Ok(false);
            }
            self.rotation.y = angle;
            let [x, _, z] = resources
                .math
                .velocity(angle, width.wrapping_mul(step - 2), 0);
            self.position.x = self.position.x.wrapping_add(i32::from(x));
            self.position.z = self.position.z.wrapping_add(i32::from(z));
            self.position.y = floor.wrapping_add(i32::from(self.height));
            request.action = 0x400;
            request.velocity = Velocity::Stop;
            self.change_motion(request.action, request.velocity, resources)?;
            cue(self, request, input.held, LedgeCue::Voice(20))?;
            cue(self, request, input.held, LedgeCue::Grabbed)?;
            if resources.registry.edition == Edition::Japan
                && !input.story_triggered
                && input.map == 23
                && self.position.x / 1000 == 0
                && self.position.z / 1000 == -112
            {
                input.story_triggered = true;
                cue(self, request, input.held, LedgeCue::HiddenMission)?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub fn update_ledge(
        &mut self,
        request: &mut Request,
        input: &mut LedgeInput,
        terrain: &mut Terrain<'_>,
        resources: &Resources,
        mut cue: impl FnMut(&mut Actor, &mut Request, u16, LedgeCue) -> Result<()>,
    ) -> Result<()> {
        self.velocity.y = 0;
        if input.held & (RIGHT | LEFT) != 0 {
            let [x, _, z] = resources.math.velocity(self.rotation.y, 0, self.width);
            let at = Position::new(
                self.position.x.wrapping_add(i32::from(x)),
                self.position.y.wrapping_sub(i32::from(self.height)),
                self.position.z.wrapping_add(i32::from(z)),
            );
            let left = level(terrain.map, terrain.cursor, at, Query::Surface)?;
            let at = Position::new(
                self.position.x.wrapping_sub(i32::from(x)),
                at.y,
                self.position.z.wrapping_sub(i32::from(z)),
            );
            let right = level(terrain.map, terrain.cursor, at, Query::Surface)?;
            if left.is_none_or(|floor| floor <= self.position.y) {
                input.held &= !LEFT;
            }
            if right.is_none_or(|floor| floor <= self.position.y) {
                input.held &= !RIGHT;
            }
        }
        let pal = resources.registry.edition == Edition::Europe;
        let mut next = None;
        match self.motion.action {
            0x400 if self.motion.loops > 0 => {
                next = Some(if input.player { 0x401 } else { 0x410 });
                if input.grapple_camera {
                    input.grapple_camera = false;
                    cue(self, request, input.held, LedgeCue::ReleaseCamera)?;
                }
            }
            0x401 => {
                let interval = if input.check == 2 {
                    5
                } else if pal {
                    25
                } else {
                    30
                };
                if input.tick % interval == 0 {
                    input.check =
                        i16::from(self.enter_ledge(request, input, terrain, resources, &mut cue)?);
                    if input.check == 0 {
                        input.held |= DOWN;
                    }
                }
                if input.held & DOWN != 0 {
                    let mut y =
                        self.position
                            .y
                            .wrapping_add(if self.height < 1701 { 600 } else { 300 });
                    loop {
                        y = y.wrapping_add(100);
                        self.position.y = y;
                        if !self.enter_ledge(request, input, terrain, resources, &mut cue)? {
                            break;
                        }
                    }
                    next = Some(0x702);
                } else if input.held & UP != 0 {
                    next = Some(0x410);
                } else if input.held & RIGHT != 0 {
                    next = Some(0x402);
                } else if input.held & LEFT != 0 {
                    next = Some(0x403);
                }
            }
            action @ (0x402 | 0x403) => {
                if !self.enter_ledge(request, input, terrain, resources, &mut cue)? {
                    next = Some(0x702);
                }
                if input.held & if action == 0x402 { RIGHT } else { LEFT } == 0 {
                    next = Some(0x401);
                }
                let duration = resources
                    .clips
                    .get(&self.motion.clip.ok_or("No Tenchu 2 ledge motion")?)
                    .ok_or("Missing Tenchu 2 ledge motion")?
                    .duration;
                if self.motion.frame == duration / 2 {
                    cue(self, request, input.held, LedgeCue::Voice(20))?;
                }
            }
            0x410 => {
                if self.motion.loops > 0 {
                    cue(self, request, input.held, LedgeCue::Recover)?;
                } else if self.motion.frame < 0 {
                    self.update_velocity(Velocity::Stop, resources)?;
                } else {
                    let speed = if self.motion.frame > if pal { 32 } else { 39 } {
                        if pal { 144 } else { 120 }
                    } else {
                        0
                    };
                    self.velocity = resources.math.velocity(self.rotation.y, speed, 0).into();
                    if self.terrain.height > i32::from(self.height) {
                        self.velocity.y = if pal { -42 } else { -35 };
                    } else {
                        self.velocity.x /= 2;
                        self.velocity.z /= 2;
                    }
                }
            }
            _ => {}
        }
        if let Some(action) = next {
            request.action = action;
            request.velocity = Velocity::Motion;
        }
        Ok(())
    }
}

#[derive(Default)]
struct LedgeSamples {
    floors: [Option<i32>; 3],
    center: Option<Surface>,
    count: usize,
}
