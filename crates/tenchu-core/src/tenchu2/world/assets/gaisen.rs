use super::*;
mod objects;
use tenchu_assets::{
    stage::MissionAssets,
    tenchu2::{
        campaign::{Kind, Objective, Region, Sequence},
        layout::{Actor, Waypoint},
        level::{
            background::Background,
            grid::Grid,
            objects::{Objects, Static, StaticKind, Trigger, Variant},
        },
    },
};

impl Assets {
    pub fn gaisen(
        disc: &Disc<impl ReadAt>,
        source: &MissionAssets,
        hero: usize,
        alternate: bool,
    ) -> Result<(Self, Layout)> {
        if source.stage.id >= 11 {
            return Err("Invalid Gaisen stage".into());
        }
        let mut layout = Layout {
            sequence: 0,
            map: 8,
            start: source.stage.start,
            facing: source.stage.facing as i16,
            actors: Vec::new(),
        };
        // Map 8 carries all three original heroes. Its stage and campaign are
        // replaced below; only the source character/effect resources are reused.
        let mut assets = Self::equipped(disc, hero, &layout, alternate)?;
        for actor in source.enemies.iter().chain(&source.construction_actors) {
            if let Some(kind) = actor_kind(actor.kind) {
                layout.actors.push(Actor {
                    slot: actor.slot,
                    kind,
                    behavior: behavior(kind, actor.think, !actor.patrol.is_empty()),
                    position: actor.position,
                    facing: actor.facing,
                    route: actor
                        .patrol
                        .iter()
                        .map(|&(p, wait)| Waypoint {
                            x: p[0],
                            z: p[2],
                            action: 0,
                            wait,
                        })
                        .collect(),
                });
            }
        }
        for actor in source.story.iter().filter(|a| {
            (0x80..=0x8e).contains(&a.kind) && a.position.iter().all(|v| v.unsigned_abs() < 300000)
        }) {
            if let Some(kind) = actor_kind(actor.kind) {
                layout.actors.push(Actor {
                    slot: 100 + layout.actors.len(),
                    kind,
                    behavior: 0x2115,
                    position: actor.position,
                    facing: actor.facing,
                    route: Vec::new(),
                });
            }
        }
        assets.cast = layout.actors.iter().map(|a| a.kind).collect();
        assets.cast.sort_unstable();
        assets.cast.dedup();
        if assets.cast.len() > 11 {
            return Err("Too many Tenchu 2 voices in imported stage".into());
        }
        let mut wanted = std::collections::BTreeSet::new();
        for &kind in &assets.cast {
            let actor = assets
                .resources
                .registry
                .actor(kind)
                .ok_or("Missing imported guard definition")?;
            if !assets.models.contains_key(&actor.model) {
                assets
                    .models
                    .insert(actor.model.clone(), characters::rig(disc, &actor.model)?);
            }
            wanted.extend(
                actor
                    .motions
                    .iter()
                    .map(|m| m.clip)
                    .filter(|id| !assets.resources.clips.contains_key(id)),
            );
            for name in assets
                .resources
                .weapons
                .parts(actor.weapon)
                .filter_map(|p| p.model.as_ref())
            {
                if !assets.weapons.contains_key(name) {
                    assets
                        .weapons
                        .insert(name.clone(), weapons::model(disc, name)?);
                }
            }
        }
        assets
            .resources
            .clips
            .extend(motions::collect(disc, wanted)?);
        assets.import_stage(source)?;
        layout.map = assets.level.id;
        let objective = if layout
            .actors
            .iter()
            .any(|a| a.kind & 0xf0 < 0x60 || a.kind >= 0xe0)
        {
            Objective::EliminateAll
        } else {
            Objective::Reach {
                region: Region {
                    center: [38, -9, 46],
                    radius: 2,
                },
                unseen: false,
            }
        };
        assets.campaign.sequences = vec![
            Sequence {
                kind: Kind::Gameplay(objective),
                resource: layout.map as u16,
                next: Some(1),
                failure: None,
            },
            Sequence {
                kind: Kind::Results,
                resource: source.stage.id as u16,
                next: None,
                failure: None,
            },
        ];
        assets.campaign.starts = [[None; 11]; 3];
        assets.campaign.starts[hero][source.stage.id] = Some(0);
        Ok((assets, layout))
    }

    fn import_stage(&mut self, source: &MissionAssets) -> Result<()> {
        let mut cursor = crate::terrain::Cursor::new(&source.terrain);
        for spawn in &source.pickups {
            let Some(mut position) =
                crate::items::layout_position(&source.terrain, &mut cursor, spawn.position.into())?
            else {
                continue;
            };
            position.y = crate::terrain::level(&source.terrain, &mut cursor, position, 0)?;
            let item = match spawn.kind {
                3 => 1,
                4 => 6,
                5 => 10,
                _ => return Err("Unsupported imported Gaisen pickup".into()),
            };
            self.gaisen_pickups.push((item, position));
        }
        let level = &mut self.level;
        level.id = 24 + source.stage.id;
        level.gaisen = Some(source.stage.clone());
        level.draw_distance = 17000;
        self.draw_distance = level.draw_distance;
        level.collision = collision::Collision::from_gaisen(&source.terrain)?;
        level
            .collision
            .set_gaisen_water_depth(collision::Gaisen::DEFAULT_WATER_DEPTH)?;
        level.map = source.map.as_ref().clone();
        level.objects = source.objects.as_ref().clone();
        level.stumps.clear();
        level.background = Background::default();
        level.vram = source.vram.clone();
        self.gaisen_scrolls = source
            .misc
            .spawns
            .iter()
            .filter_map(|spawn| match spawn.kind {
                tenchu_assets::misc::Kind::TextureScroll {
                    rect,
                    source,
                    velocity,
                } => Some(crate::effects::TextureScroll {
                    offset: [0; 2],
                    destination: rect,
                    source,
                    velocity,
                }),
                _ => None,
            })
            .collect();
        level.object_sizes = level
            .objects
            .iter()
            .map(|part| {
                let (low, high) = part
                    .geometry
                    .iter()
                    .flat_map(|g| &g.vertices)
                    .fold((i16::MAX, i16::MIN), |(low, high), v| {
                        (low.min(v[1]), high.max(v[1]))
                    });
                (i32::from(high) - i32::from(low)) / 2
            })
            .collect();
        level.scene = Objects {
            variant: Variant::Base,
            bounds: Vec::new(),
            dynamic: Vec::new(),
            statics: source
                .placements
                .iter()
                .map(|p| Static {
                    position: p.position,
                    rotation: [0, p.facing as i16, 0],
                    kind: StaticKind::Model(p.part),
                })
                .collect(),
            triggers: vec![
                Trigger {
                    position: [0; 3],
                    angle: 0,
                    kind: 19,
                    target: 0,
                },
                // ACM damage floors reserve material 0x401. Their ground
                // contacts feed the native fire collider in update_scenery.
                Trigger {
                    position: [0; 3],
                    angle: 0,
                    kind: crate::tenchu2::environment::GAISEN_HEAT,
                    target: 0,
                },
            ],
        };
        level.grid = grid(source)?;
        self.import_hinged_objects(source)?;
        Ok(())
    }
}

fn grid(source: &MissionAssets) -> Result<Grid> {
    let mut low = [i32::MAX; 3];
    let mut high = [i32::MIN; 3];
    for row in &source.terrain.rows {
        for i in 0..row.count {
            let node = source.terrain.node(row.list + i * 16)?;
            let y = i32::from(node.y);
            let end = y + i32::from(node.dy);
            for point in [
                [
                    i32::from(node.bounds[0]),
                    y.min(end),
                    i32::from(node.bounds[1]),
                ],
                [
                    i32::from(node.bounds[2]),
                    y.max(end),
                    i32::from(node.bounds[3]),
                ],
            ] {
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis] * 10);
                    high[axis] = high[axis].max(point[axis] * 10);
                }
            }
        }
    }
    let origin = [
        low[0].div_euclid(16000) * 16 - 16,
        high[1].div_euclid(16000) * 16 + 32,
        low[2].div_euclid(16000) * 16 - 16,
    ];
    let dimensions = [
        (high[0] / 1000 - origin[0]) / 16 + 2,
        (origin[1] - low[1] / 1000) / 16 + 2,
        (high[2] / 1000 - origin[2]) / 16 + 2,
    ];
    let count = dimensions.iter().product::<i32>();
    if dimensions.iter().any(|n| !(1..=64).contains(n)) || count > i32::from(i16::MAX) {
        return Err("Gaisen scenery exceeds Tenchu 2 grid".into());
    }
    Ok(Grid {
        origin: origin.map(|v| v as i16),
        dimensions: dimensions.map(|v| v as i16),
        height: 16,
        cells: (0..count as usize).map(Some).collect(),
    })
}

fn actor_kind(kind: i16) -> Option<i16> {
    Some(match kind {
        7 | 0x10 | 0x13 | 0x16 | 0x19 => 0x23,
        8 | 0x11 | 0x14 | 0x17 | 0x1a => 0x24,
        9 | 0x12 | 0x15 | 0x18 | 0x1b => 0x25,
        0x20 => 0x40,
        0x21 => 0x41,
        0x22 => 0x48,
        0x30 | 0x31 => 0x30,
        0x32 => 0x32,
        0x33 => 0x33,
        0x40 => 0x31,
        0x41 => 0x33,
        0x42 => 0x32,
        0x50 => 0x53,
        0x51 => 0x54,
        0x52 => 0x55,
        0x60 => 0x4f,
        0x61 => 0x49,
        0x62 | 0x63 => 0x48,
        0x70 => 0x56,
        0x71 => 0x57,
        0x72 => 0x58,
        0x73 | 0x74 => 0x5e,
        0x80 => 0xe0,
        0x81 => 0xf0,
        0x82 => 0xe7,
        0x83 => 0xfa,
        0x84 => 0xe2,
        0x85 => 0xee,
        0x86 | 0x89 => 0x71,
        0x87 | 0x8a => 0xf6,
        0x88 => 0xf5,
        0x8b => 0xf0,
        0x8c => 0xe1,
        0x8d => 0xe7,
        0x8e => 0xe0,
        0x90 => 0x64,
        0x91 | 0x92 | 0x95 | 0x96 => 0x61,
        0x93 | 0x94 => 0x69,
        0xa0 | 0xa1 => 0x7b,
        0xa2 | 0xa3 => 0x7a,
        0xa4 | 0xa5 => 0x79,
        0xa9 => 0x1a,
        _ => return None,
    })
}

fn behavior(kind: i16, original: i16, patrol: bool) -> u16 {
    if kind & 0xf0 == 0x60 {
        return 0x1514;
    }
    let idle = if patrol {
        8
    } else {
        match original & 15 {
            4 => 4,
            5 => 5,
            6 => 6,
            _ => 3,
        }
    };
    0x2120 | idle
}
