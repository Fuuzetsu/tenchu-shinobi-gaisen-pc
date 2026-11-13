mod mesh;
#[cfg(test)]
mod tests;
mod ui;
mod viewport;

use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    stage::{Enemy, MissionAssets, Pickup, Placement},
    tenchu2::{Edition, collision::Collision},
    terrain::Node,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Tenchu {
        stage: usize,
        layout: usize,
    },
    Tenchu2 {
        map: usize,
        sequence: usize,
        edition: Edition,
    },
}

impl Source {
    pub fn label(&self) -> String {
        match self {
            Self::Tenchu { stage, layout } => format!("T1 · area {stage} · layout {layout}"),
            Self::Tenchu2 {
                map,
                sequence,
                edition,
            } => format!("T2 {} · map {map} · sequence {sequence}", edition.name()),
        }
    }
    fn t2(&self) -> bool {
        matches!(self, Self::Tenchu2 { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub enabled: bool,
    pub story: bool,
    pub name: String,
    pub source_kind: i16,
    pub spawn: Enemy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    pub key: [usize; 2],
    pub original: Node,
    pub node: Node,
    pub geometry: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign {
    pub source: Source,
    pub start: [i32; 3],
    pub facing: i16,
    pub actors: Vec<Actor>,
    pub props: Vec<Placement>,
    pub pickups: Vec<Pickup>,
    pub terrain: Vec<Edit>,
}

impl Campaign {
    pub fn edit_surface(&mut self, edit: Edit, follow: bool) {
        let previous = self
            .terrain
            .iter()
            .find(|e| e.key == edit.key)
            .map_or(edit.original, |e| e.node);
        if follow && mesh::changes_surface(previous, edit.node) {
            let t2 = self.source.t2();
            let move_point = |p: &mut [i32; 3]| {
                let point = glam::Vec3::from_array(p.map(|v| v as f32));
                let [x1, z1, x2, z2] = previous.bounds.map(|v| f32::from(v) * 10.0);
                let pad = if t2 { 0.0 } else { 10.0 };
                if point.x < x1 || point.x >= x2 + pad || point.z < z1 || point.z >= z2 + pad {
                    return;
                }
                let y = mesh::height(previous, point, t2);
                if (point.y - y).abs() <= 200.0 {
                    p[1] += (mesh::height(edit.node, point, t2) - y).round() as i32;
                }
            };
            move_point(&mut self.start);
            for actor in &mut self.actors {
                move_point(&mut actor.spawn.position);
                for (point, _) in &mut actor.spawn.patrol {
                    move_point(point);
                }
            }
            for prop in &mut self.props {
                move_point(&mut prop.position);
            }
            for pickup in &mut self.pickups {
                move_point(&mut pickup.position);
            }
        }
        self.terrain.retain(|e| e.key != edit.key);
        if edit.node != edit.original {
            self.terrain.push(edit);
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.source {
            Source::Tenchu { stage, layout } if stage > 10 || layout > 3 => {
                return Err("Invalid T1 campaign source".into());
            }
            Source::Tenchu2 { map, sequence, .. } if map >= 24 || sequence >= 300 => {
                return Err("Invalid T2 campaign source".into());
            }
            _ => (),
        }
        if self.actors.len() > super::limits::Limits::MAX.characters
            || self.props.len() > super::limits::Limits::MAX.props
            || self.pickups.len() > super::limits::Limits::MAX.pickups
            || self.terrain.len() > super::limits::Limits::MAX.terrain
        {
            return Err("Campaign project exceeds the supported authoring limits".into());
        }
        position(self.start)?;
        for actor in &self.actors {
            if actor.enabled {
                position(actor.spawn.position).map_err(|e| format!("{}: {e}", actor.name))?;
            }
            if actor.name.len() > 512
                || actor.spawn.patrol.len() > tenchu_assets::trial::Limits::MAX.patrol
                || actor.spawn.kind < 0
            {
                return Err("Invalid campaign character".into());
            }
            for (point, wait) in &actor.spawn.patrol {
                position(*point)?;
                if !(0..=32767).contains(wait) {
                    return Err("Invalid patrol wait".into());
                }
            }
        }
        for prop in &self.props {
            position(prop.position).map_err(|e| format!("Source prop {}: {e}", prop.part))?;
            if prop.part > 65535 {
                return Err("Invalid source prop".into());
            }
        }
        for pickup in &self.pickups {
            position(pickup.position)?;
            if pickup.kind >= 25 {
                return Err("Invalid item pickup".into());
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        for edit in &self.terrain {
            let end = i32::from(edit.node.y) + i32::from(edit.node.dy);
            if !keys.insert(edit.key)
                || edit.original.bounds != edit.node.bounds
                || edit.geometry && edit.node.division & !edit.original.division != 0
                || !(-32000..=32000).contains(&edit.node.y)
                || !(-32000..=32000).contains(&end)
            {
                return Err("Invalid or duplicate campaign terrain edit".into());
            }
        }
        Ok(())
    }
}
fn position(p: [i32; 3]) -> Result<()> {
    if p.iter().any(|v| !(-320_000..=320_000).contains(v)) {
        Err(format!(
            "Campaign position {p:?} exceeds the supported coordinate range"
        ))
    } else {
        Ok(())
    }
}

pub(super) struct Cache {
    source: Source,
    character: i16,
    mission: MissionAssets,
    native_collision: Option<Collision>,
    nodes: BTreeMap<[usize; 2], Node>,
    initial: Campaign,
    appearances: BTreeMap<usize, String>,
}

impl Cache {
    fn load(assets: &GameAssets<DiscSource>, source: Source, character: i16) -> Result<Self> {
        let (mission, native_collision, actors, start, facing, appearances) = match source {
            Source::Tenchu { stage, layout } => {
                let mission = MissionAssets::load(assets, stage, character, layout)?;
                let mut actors: Vec<_> = mission
                    .enemies
                    .iter()
                    .map(|e| Actor {
                        enabled: position(e.position).is_ok(),
                        story: false,
                        name: format!("Character {} · source slot {}", e.kind, e.slot),
                        source_kind: e.kind,
                        spawn: e.clone(),
                    })
                    .collect();
                for story in &mission.story {
                    if let Some(actor) = actors.iter_mut().find(|a| a.source_kind == story.kind) {
                        actor.enabled = position(story.position).is_ok();
                        actor.story = true;
                        actor.spawn.position = story.position;
                        actor.spawn.facing = story.facing;
                        actor.spawn.think = story.think;
                    } else {
                        actors.push(Actor {
                            enabled: position(story.position).is_ok(),
                            story: true,
                            name: format!("Story character {}", story.kind),
                            source_kind: story.kind,
                            spawn: Enemy {
                                slot: actors.len(),
                                kind: story.kind,
                                think: story.think,
                                position: story.position,
                                facing: story.facing,
                                patrol: vec![],
                            },
                        });
                    }
                }
                let start = mission.stage.start;
                let facing = mission.stage.facing as i16;
                (
                    mission,
                    None::<Collision>,
                    actors,
                    start,
                    facing,
                    BTreeMap::new(),
                )
            }
            Source::Tenchu2 {
                map,
                sequence,
                edition,
            } => {
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = (map, sequence, edition);
                    return Err(
                        "T2 campaign sources require a connected disc in the desktop game".into(),
                    );
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let path = crate::tenchu2::menu::known_discs()
                        .into_iter()
                        .find(|(e, _)| *e == edition)
                        .map(|(_, p)| p)
                        .ok_or_else(|| {
                            format!("Connect your {} T2 disc in the launcher", edition.name())
                        })?;
                    let sandbox =
                        crate::tenchu2::Sandbox::open(&path, Some(map), Some(sequence), None)?;
                    let mission = sandbox.mission(
                        assets,
                        crate::session::Options {
                            character,
                            ..Default::default()
                        },
                    )?;
                    let layout = sandbox.layout.as_ref().ok_or("Missing campaign layout")?;
                    let mut appearances = BTreeMap::new();
                    let actors = layout
                        .actors
                        .iter()
                        .enumerate()
                        .map(|(i, a)| {
                            let body = sandbox
                                .characters
                                .iter()
                                .find(|b| b.kinds.contains(&a.kind));
                            if let Some(body) = body {
                                appearances.insert(
                                    i + 1,
                                    format!("{}{}", edition.model_prefix(), body.name),
                                );
                            }
                            Actor {
                                enabled: position(a.position).is_ok(),
                                story: false,
                                name: format!(
                                    "{} · source slot {}",
                                    body.map_or_else(
                                        || format!("T2 character {}", a.kind),
                                        |b| b.name.clone()
                                    ),
                                    a.slot
                                ),
                                source_kind: a.kind,
                                spawn: Enemy {
                                    slot: i,
                                    kind: 16,
                                    think: if a.route.is_empty() { 0x3454 } else { 0x3453 },
                                    position: a.position,
                                    facing: a.facing,
                                    patrol: a
                                        .route
                                        .iter()
                                        .map(|p| ([p.x, a.position[1], p.z], p.wait.max(0)))
                                        .collect(),
                                },
                            }
                        })
                        .collect();
                    (
                        mission,
                        Some(sandbox.level.collision),
                        actors,
                        layout.start,
                        layout.facing,
                        appearances,
                    )
                }
            }
        };
        let nodes = if let Some(collision) = &native_collision {
            collision
                .cells
                .iter()
                .enumerate()
                .flat_map(|(r, c)| c.nodes.iter().enumerate().map(move |(i, n)| ([r, i], *n)))
                .collect()
        } else {
            let mut nodes = BTreeMap::new();
            for (r, row) in mission.terrain.rows.iter().enumerate() {
                for i in 0..row.count {
                    nodes.insert([r, i], mission.terrain.node(row.list + i * 16)?);
                }
            }
            nodes
        };
        let initial = Campaign {
            source: source.clone(),
            start,
            facing,
            actors,
            props: mission.placements.clone(),
            pickups: mission.pickups.clone(),
            terrain: vec![],
        };
        Ok(Self {
            source,
            character,
            mission,
            native_collision,
            nodes,
            initial,
            appearances,
        })
    }

    fn build(&self, campaign: &Campaign) -> Result<MissionAssets> {
        let mut mission = self.mission.clone();
        for edit in &campaign.terrain {
            if self.nodes.get(&edit.key) != Some(&edit.original) {
                return Err(
                    "Terrain source differs from this project; reconnect the original disc edition"
                        .into(),
                );
            }
        }
        let changes: Vec<_> = campaign
            .terrain
            .iter()
            .map(|e| (e.key[0], e.key[1], e.node))
            .collect();
        mission.terrain = if let Some(source) = &self.native_collision {
            source.edited(&changes)?.into_terrain()?
        } else {
            mission.terrain.edited(&changes)?
        };
        if campaign.terrain.iter().any(|e| e.geometry) {
            let map = Arc::make_mut(&mut mission.map);
            for edit in campaign.terrain.iter().filter(|e| e.geometry) {
                let count = mesh::sculpt(map, edit.original, edit.node, self.source.t2())?;
                if count == 0 && mesh::changes_surface(edit.original, edit.node) {
                    return Err(format!(
                        "Surface {} / {} has no matching visible floor. Clear ‘Move visible terrain with collision’ to edit this collision-only surface.",
                        edit.key[0], edit.key[1]
                    ));
                }
            }
        }
        mission.stage.start = campaign.start;
        mission.stage.facing = i32::from(campaign.facing);
        mission.enemies = campaign
            .actors
            .iter()
            .filter(|a| a.enabled && !a.story)
            .enumerate()
            .map(|(i, a)| {
                let mut s = a.spawn.clone();
                s.slot = i;
                s
            })
            .collect();
        mission.placements = campaign.props.clone();
        if mission
            .placements
            .iter()
            .any(|p| p.part >= mission.objects.len())
        {
            return Err("Source scenery is unavailable on this disc".into());
        }
        mission.pickups = campaign.pickups.clone();
        mission.trial = None;
        mission.balma_terrain = None;
        mission.construction_actors.clear();
        mission.construction_pickups.clear();
        mission.story = campaign
            .actors
            .iter()
            .filter(|a| a.enabled && a.story)
            .map(|a| tenchu_assets::executable::StoryActor {
                kind: a.spawn.kind,
                position: a.spawn.position,
                facing: a.spawn.facing,
                think: a.spawn.think,
            })
            .collect();
        mission.construction.clear();
        mission.layout.clear();
        mission.random_layouts = None;
        mission.misc.spawns.clear();
        mission.scripts.events.clear();
        mission.scripts.cutscenes = tenchu_assets::script::Cutscenes::parse(vec![255; 2])?;
        Ok(mission)
    }
}

impl Editor {
    pub(super) fn load_campaign_cache(
        &mut self,
        assets: &GameAssets<DiscSource>,
        source: &Source,
        character: i16,
    ) -> Result<()> {
        if self
            .campaign_cache
            .as_ref()
            .is_none_or(|c| c.source != *source || c.character != character)
        {
            self.campaign_cache = Some(Cache::load(assets, source.clone(), character)?);
        }
        Ok(())
    }

    pub(crate) fn campaign_mission(
        &mut self,
        assets: &GameAssets<DiscSource>,
    ) -> Result<MissionAssets> {
        let campaign = self
            .project
            .campaign
            .as_ref()
            .ok_or("No campaign source")?
            .clone();
        self.load_campaign_cache(assets, &campaign.source, self.project.character)?;
        self.campaign_cache.as_ref().unwrap().build(&campaign)
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn campaign_choices(&self) -> Vec<(Source, String)> {
        self.campaign_sources.clone()
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn first_campaign_surface(&self) -> Option<([usize; 2], Node)> {
        self.campaign_cache
            .as_ref()?
            .nodes
            .iter()
            .find(|(_, n)| n.division != 0)
            .map(|(&key, &node)| (key, node))
    }

    pub(crate) fn open_campaign(
        &mut self,
        assets: &GameAssets<DiscSource>,
        source: Source,
    ) -> Result<()> {
        self.commit();
        if self.selected_save.is_some() || self.campaign_template.as_ref() != Some(&self.project) {
            self.save(false)?;
        }
        self.load_campaign_cache(assets, &source, self.project.character)?;
        let cache = self.campaign_cache.as_ref().unwrap();
        let mut project = self.new_project(Mission::new(0, Objective::EliminateEnemies, 16)?);
        project.character = self.project.character;
        project.mission.title = format!("{} · Studio", cache.mission.stage.name);
        project.campaign = Some(cache.initial.clone());
        project.actors = cache.appearances.clone();
        self.select(project, assets)?;
        self.campaign_template = Some(self.project.clone());
        self.panel = Panel::Terrain;
        self.tool = Tool::Select;
        self.notice = "Campaign map opened. Edit terrain, characters and scenery; build objectives under Events. Playtests use Gaisen rules; source cutscenes, scripts and T2 dynamic scenery are not imported.".into();
        Ok(())
    }

    pub(super) fn world_cursor(&self) -> [i32; 3] {
        if self.project.campaign.is_some() {
            self.campaign_cursor
        } else {
            let mut point = self.cursor.world();
            if let Some(theme) = &self.theme {
                point[1] = i32::from(theme.height(
                    &self.project.mission,
                    self.cursor.x,
                    self.cursor.z,
                    false,
                )) * 1000;
            }
            point
        }
    }
}
