use super::{
    Effect, Effects, Frame,
    points::Points,
    sprites,
    surface::{Blend, Plane, Quad},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::{ModelId, Scene},
    tenchu2::{
        terrain::{self, Cursor, Query},
        visibility::View,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{collision::Collision, effects::Rules};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum Contact {
    #[default]
    Air,
    Wall,
    Ground,
    Water,
    Submerged,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Drop {
    pub position: Position,
    pub floor: i32,
    pub velocity: Vector3<i16>,
    pub contact: Contact,
    pub radius: i32,
    pub plane: Plane,
    pub age: i16,
    pub slide: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blood {
    pub parent: Option<ModelId>,
    pub drops: [Drop; 4],
    pub lifetime: i16,
    pub gravity: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pool {
    pub position: Position,
    pub parent: Option<ModelId>,
    pub age: i16,
    pub lifetime: i16,
    pub plane: Plane,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spurt {
    pub position: Position,
    pub yaw: i16,
    pub speed: i32,
    pub gravity: i16,
}

impl Pool {
    pub fn update(&mut self, drawings: &mut Vec<Quad>) -> Result<bool> {
        let previous = self.age;
        self.age = self.age.wrapping_add(1);
        if previous >= self.lifetime {
            return Ok(false);
        }
        let radius = if self.age < 40 {
            (i32::from(self.age) + 20) * 10
        } else {
            600
        };
        let plane = if self.plane.extent.contains(&0) {
            Plane::flat()
        } else {
            self.plane
        };
        Quad {
            image: 4,
            position: self.position,
            parent: self.parent,
            corners: plane.corners(radius)?,
            color: [128; 3],
            blend: Blend::Opaque,
        }
        .blood(
            (i32::from(self.lifetime) - i32::from(self.age)) * 2,
            drawings,
        );
        Ok(true)
    }
}

impl Blood {
    pub fn update(
        &mut self,
        scene: &mut Scene,
        view: View,
        map: &Collision,
        cursor: &mut Cursor,
        frame: &mut Frame,
    ) -> Result<bool> {
        let parent = self.parent.map(|p| scene.world(p)).transpose()?;
        let mut alive = false;
        for (index, drop) in self.drops.iter_mut().enumerate() {
            let previous = drop.age;
            drop.age = drop.age.wrapping_add(1);
            if previous >= self.lifetime {
                continue;
            }
            alive = true;
            if drop.age < 0 {
                continue;
            }
            let fade = (i32::from(self.lifetime) - i32::from(drop.age)) * 2;
            match drop.contact {
                Contact::Air => {
                    drop.position = drop.position.wrapping_add(drop.velocity.map(i32::from));
                    drop.billboard(1, Blend::Opaque, 128, parent, view, &mut frame.sprites);
                    drop.velocity.y = drop.velocity.y.wrapping_add(self.gravity);
                    let impact = drop.position;
                    if drop.land(map, cursor, self.lifetime)? && index == 0 {
                        frame.blood_sounds.push(impact);
                    }
                }
                Contact::Wall => {
                    drop.slide = drop.slide.wrapping_add(1);
                    let radius = drop.radius as i16;
                    let limit = drop.floor.wrapping_sub(drop.position.y).wrapping_add(100);
                    let top =
                        (drop.slide.wrapping_mul(4).wrapping_sub(radius) as i32).min(limit) as i16;
                    let bottom =
                        (radius.wrapping_add(drop.slide.wrapping_mul(16)) as i32).min(limit) as i16;
                    let (x, z) = if drop.plane.extent[0] == 0 {
                        (0, radius)
                    } else {
                        (radius, 0)
                    };
                    Quad {
                        image: 3,
                        position: drop.position,
                        parent: self.parent,
                        corners: [
                            Vector3::new(x.wrapping_neg(), top, z.wrapping_neg()),
                            Vector3::new(x, top, z),
                            Vector3::new(x.wrapping_neg(), bottom, z.wrapping_neg()),
                            Vector3::new(x, bottom, z),
                        ],
                        color: [128; 3],
                        blend: Blend::Opaque,
                    }
                    .blood(fade, &mut frame.quads);
                }
                Contact::Ground | Contact::Water => {
                    let corners = drop.plane.corners(drop.radius)?;
                    let water = matches!(drop.contact, Contact::Water);
                    if water {
                        drop.radius = drop.radius.wrapping_add(10);
                    }
                    if !water || fade < 128 {
                        Quad {
                            image: 2,
                            position: drop.position,
                            parent: self.parent,
                            corners,
                            color: [128; 3],
                            blend: Blend::Opaque,
                        }
                        .blood(fade, &mut frame.quads);
                    }
                }
                Contact::Submerged => {
                    drop.position.x = drop
                        .position
                        .x
                        .wrapping_add(i32::from(drop.velocity.x) >> 1);
                    drop.position.z = drop
                        .position
                        .z
                        .wrapping_add(i32::from(drop.velocity.z) >> 1);
                    if fade < 128 {
                        for blend in [Blend::Add, Blend::SubtractMask] {
                            drop.billboard(2, blend, fade as u8, parent, view, &mut frame.sprites);
                        }
                    }
                    drop.radius = drop.radius.wrapping_add(30);
                }
            }
        }
        Ok(alive)
    }
}

impl Drop {
    fn billboard(
        &self,
        image: u8,
        blend: Blend,
        shade: u8,
        parent: Option<Matrix>,
        view: View,
        drawings: &mut Vec<sprites::Drawing>,
    ) {
        let mut matrix = Matrix {
            translation: self.position,
            ..Matrix::default()
        };
        if let Some(parent) = parent {
            matrix = parent.compose(&matrix);
        }
        if let Some(depth) = view.actor(&matrix) {
            let scale = self.radius.wrapping_mul(9600) / (i32::from(depth) * 4);
            drawings.push(sprites::Drawing {
                image,
                position: matrix.translation,
                size: [self.radius as i16; 2],
                scale: [scale as i16; 2],
                color: [shade; 3],
                rotation: 0,
                depth: depth as i16,
                blend,
            });
        }
    }

    fn land(&mut self, map: &Collision, cursor: &mut Cursor, lifetime: i16) -> Result<bool> {
        let mut sound = false;
        if let Some(floor) = terrain::level(map, cursor, self.position, Query::Uncached)? {
            self.floor = floor;
            if self.position.y.wrapping_add(i32::from(self.velocity.y)) < floor {
                if cursor.material & 0xf00 != 0x300
                    || terrain::level(
                        map,
                        cursor,
                        self.position.wrapping_sub(Position::new(0, 500, 0)),
                        Query::Uncached,
                    )?
                    .is_none()
                {
                    return Ok(false);
                }
                self.contact = if cursor.material & 0xf00 == 0x300 {
                    Contact::Submerged
                } else {
                    Contact::Water
                };
                self.age = lifetime.wrapping_sub(63);
            } else {
                self.position.y = floor;
                if matches!(cursor.material & 0xf00, 0x200 | 0x300) {
                    self.contact = Contact::Water;
                    self.age = lifetime.wrapping_sub(63);
                } else {
                    self.contact = Contact::Ground;
                    sound = true;
                }
            }
        } else {
            terrain::level(
                map,
                cursor,
                self.position.wrapping_sub(Position::new(
                    i32::from(self.velocity.x),
                    0,
                    i32::from(self.velocity.z),
                )),
                Query::Uncached,
            )?;
            // A droplet can leave the map entirely; there is then no wall to stain.
            let Some(id) = cursor.current else {
                self.age = lifetime;
                return Ok(false);
            };
            let node = &map.cells[id.cell].nodes[id.node];
            self.contact = Contact::Wall;
            self.plane.extent = if self.position.x < i32::from(node.bounds[0]) * 10
                || self.position.x > i32::from(node.bounds[2]) * 10
            {
                [0, 1]
            } else {
                [1, 0]
            };
            sound = true;
        }
        if matches!(self.contact, Contact::Ground | Contact::Water) {
            let id = cursor
                .current
                .ok_or("Missing Tenchu 2 blood contact surface")?;
            self.plane = Plane::from_node(&map.cells[id.cell].nodes[id.node]);
        }
        Ok(sound)
    }
}

impl Effects {
    pub fn blood(
        &mut self,
        spurt: Spurt,
        math: &Math,
        rules: &Rules,
        random: &mut Random,
    ) -> Result<()> {
        let Spurt {
            position,
            yaw,
            speed,
            gravity,
        } = spurt;
        let half = speed >> 1;
        if half == 0 {
            return Err("Zero Tenchu 2 blood speed".into());
        }
        for i in 0..3 {
            let controls = std::array::from_fn(|j| {
                let mut velocity = Vector3::from(math.velocity(
                    yaw.wrapping_add(if j & 1 == 0 { -64 } else { 64 }),
                    (speed - random.next15() % half) as i16,
                    0,
                ));
                velocity.y = if j < 2 { -20 } else { 0 };
                velocity
            });
            let mut points = Points::burst(position, controls, gravity >> 1, rules);
            points.age = -i * 4;
            points.color = [128, 0, 0];
            self.insert(Effect::Points(points))?;
        }
        self.blood_phase = !self.blood_phase;
        let phase = i32::from(self.blood_phase);
        let mut drops = std::array::from_fn(|i| Drop {
            velocity: math
                .velocity(
                    yaw.wrapping_add([-160, 160, -320, 320][i]),
                    (speed + phase * 32 - random.next15() % half) as i16,
                    0,
                )
                .into(),
            age: 180,
            ..Drop::default()
        });
        let count = (random.next15() & 3) as usize + 1;
        for (i, drop) in drops[..count].iter_mut().enumerate() {
            drop.velocity.y = (-phase * 32 - half - random.next15() % half) as i16;
            drop.position = position;
            drop.age = (-phase * 4 - i as i32) as i16;
            drop.radius = random.next15() % 200 + 150;
        }
        self.insert(Effect::Blood(Blood {
            parent: None,
            drops,
            lifetime: 180,
            gravity,
        }))
    }

    pub fn death_pool(
        &mut self,
        mut position: Position,
        map: &Collision,
        cursor: &mut Cursor,
    ) -> Result<()> {
        let mut plane = Plane::flat();
        let mut age = 0;
        if let Some(floor) = terrain::level(map, cursor, position, Query::Surface)? {
            position.y = floor;
            let id = cursor
                .current
                .ok_or("Missing Tenchu 2 death-pool surface")?;
            plane = Plane::from_node(&map.cells[id.cell].nodes[id.node]);
            if matches!(cursor.material & 0xf00, 0x200 | 0x300) {
                age = 172;
            }
        }
        self.insert(Effect::Pool(Pool {
            position,
            parent: None,
            age,
            lifetime: 300,
            plane,
        }))
    }
}
