use crate::{session::Options, source::DiscSource};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{
    Result,
    executable::Executable,
    game::GameAssets,
    stage::{Enemy, MissionAssets, Placement},
    tenchu2::{layout::Layout, level::Level},
};
use tenchu_core::{
    game::Game,
    math::{Math, Matrix, Position},
    terrain::{self, Cursor},
};

#[cfg(test)]
mod tests;

pub struct Sandbox {
    pub level: Level,
    pub layout: Option<Layout>,
    pub guard: Option<i16>,
}

impl Sandbox {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(
        disc: &tenchu_assets::disc::Disc<impl tenchu_assets::ReadAt>,
        map: Option<usize>,
        sequence: Option<usize>,
        guard: Option<i16>,
    ) -> Result<Self> {
        let layout = if let Some(sequence) = sequence {
            Some(Layout::load(disc, sequence)?)
        } else {
            Layout::available(disc)?
                .into_iter()
                .find(|layout| layout.map == map.unwrap_or(0))
        };
        let map = map.unwrap_or_else(|| layout.as_ref().map_or(0, |l| l.map));
        if layout.as_ref().is_some_and(|l| l.map != map) {
            return Err("The selected Tenchu 2 sequence belongs to a different map".into());
        }
        if guard.is_some() && layout.is_none() {
            return Err("This map has no Tenchu 2 actor layout to replace with guards".into());
        }
        Ok(Self {
            level: Level::load(disc, map)?,
            layout,
            guard,
        })
    }

    pub fn load(&self, assets: &GameAssets<DiscSource>, options: Options) -> Result<Game> {
        let executable = Executable::new(&assets.executable)?;
        let mut mission = MissionAssets::load_localized(
            assets,
            0,
            options.character,
            1,
            options.language.unwrap_or_default(),
        )?;
        let math = Math {
            tables: executable.math_tables()?,
        };
        mission.map = Arc::new(self.level.map.clone());
        let mut objects = self.level.objects.clone();
        let mut tilted = BTreeMap::new();
        mission.placements.clear();
        for source in &self.level.placements {
            let mut part = source.part;
            let pitch = source.rotation[0];
            if pitch != 0 {
                part = *tilted.entry((part, pitch)).or_insert_with(|| {
                    let mut tilted = objects[part].clone();
                    let matrix = Matrix {
                        rotation: math.rotate_yxz([pitch, 0, 0]),
                        ..Default::default()
                    };
                    for geometry in &mut tilted.geometry {
                        for vertex in &mut geometry.vertices {
                            *vertex = matrix.transform(*vertex).map(|v| v as i16).to_array();
                        }
                    }
                    let index = objects.len();
                    objects.push(tilted);
                    index
                });
            }
            mission.placements.push(Placement {
                part,
                cloned: true,
                position: source.position,
                facing: i32::from(source.rotation[1]),
            });
        }
        mission.objects = Arc::new(objects);
        mission.object_names = (0..mission.objects.len())
            .map(|i| format!("Tenchu 2 object {i}"))
            .collect();
        mission.terrain = self.level.collision.clone().into_terrain()?;
        mission.balma_terrain = None;
        mission.world_vram = Some(Arc::new(self.level.vram.clone()));
        mission.stage.name = format!("Tenchu 2 map {} — sandbox", self.level.id);
        mission.stage.directory = format!("tenchu2/map{}", self.level.id);
        if let Some(layout) = &self.layout {
            mission
                .stage
                .directory
                .push_str(&format!("/seq{}", layout.sequence));
        }
        mission.map_image.size = [0; 2];
        mission.enemies.clear();
        mission.construction_actors.clear();
        mission.construction_pickups.clear();
        mission.pickups.clear();
        mission.story.clear();
        mission.construction.clear();
        mission.layout.clear();
        mission.random_layouts = None;
        mission.misc.spawns.clear();
        mission.scripts.events.clear();
        mission.scripts.cutscenes = tenchu_assets::script::Cutscenes::parse(vec![255; 2])?;
        let mut cursor = Cursor::new(&mission.terrain);
        if let Some(layout) = &self.layout {
            let mut start = layout.start;
            let floor = terrain::level(
                &mission.terrain,
                &mut cursor,
                Position::new(start[0], start[1] - 1500, start[2]),
                0,
            )?;
            if floor == terrain::NONE || (floor - start[1]).abs() > 5000 {
                return Err(format!(
                    "Sequence {} has no nearby starting floor in map {}",
                    layout.sequence, self.level.id
                ));
            }
            start[1] = floor;
            mission.stage.start = start;
            mission.stage.facing = i32::from(layout.facing);
            if let Some(kind) = self.guard {
                if !mission.models.contains_key(&kind) || !matches!(kind, 0x10..=0x14) {
                    return Err("Sandbox guards must be Gaisen kinds 16–20".into());
                }
                for actor in &layout.actors {
                    let mut position = actor.position;
                    let floor = terrain::level(
                        &mission.terrain,
                        &mut cursor,
                        Position::new(position[0], position[1] - 1500, position[2]),
                        0,
                    )?;
                    if floor == terrain::NONE || (floor - position[1]).abs() > 5000 {
                        continue;
                    }
                    position[1] = floor;
                    mission.enemies.push(Enemy {
                        slot: mission.enemies.len(),
                        kind,
                        think: 0x3453,
                        position,
                        facing: actor.facing,
                        patrol: actor
                            .route
                            .iter()
                            .map(|p| ([p.x, 0, p.z], p.wait))
                            .collect(),
                    });
                }
            }
        } else {
            mission.stage.start = spawn(&mission, &math)?;
            mission.stage.facing = 0;
        }
        let mut game = Game::new(mission, &executable, options.character, 1)?;
        game.camera.snap_pending = true;
        game.intelligence.difficulty = options.difficulty;
        game.roster.get_mut(game.player_id).inventory[..4].copy_from_slice(&[255, 10, 5, 2]);
        Ok(game)
    }
}

fn spawn(mission: &MissionAssets, math: &Math) -> Result<[i32; 3]> {
    let map = &mission.terrain;
    let surfaces =
        crate::visuals::surfaces::Surfaces::new(&crate::render_mesh::stage(mission, math, None));
    let mut choices = Vec::new();
    for row in &map.rows {
        for i in 0..row.count {
            let node = map.node(row.list + i * 16)?;
            if node.attributes & 0xc000 != 0 || node.division != u16::MAX {
                continue;
            }
            let [x1, z1, x2, z2] = node.bounds.map(i32::from);
            let area = i64::from(x2 - x1) * i64::from(z2 - z1);
            choices.push((area, [(x1 + x2) * 5, i32::from(node.y) * 10, (z1 + z2) * 5]));
        }
    }
    choices.sort_by_key(|(area, _)| std::cmp::Reverse(*area));
    let mut cursor = Cursor::new(map);
    for (_, point) in choices {
        let floor = terrain::level(
            map,
            &mut cursor,
            Position::new(point[0], point[1] - 2200, point[2]),
            0,
        )?;
        let p = glam::Vec3::from_array(point.map(|v| v as f32));
        if floor == point[1]
            && surfaces
                .cast(p - glam::Vec3::Y * 100.0, p + glam::Vec3::Y * 100.0)
                .is_some()
        {
            return Ok(point);
        }
    }
    Err("No open starting floor; specify a Tenchu 2 --sequence".into())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn command(args: &[std::ffi::OsString]) -> Result<()> {
    let help = "tenchu-pc inspect-t2 TENCHU2.cue\ntenchu-pc play-t2 GAISEN.cue TENCHU2.cue [--map 0..23] [--sequence 0..299] [--guards sword|spear|archer] [--character rikimaru|ayame]\ntenchu-pc render-t2 GAISEN.cue TENCHU2.cue OUTPUT.png [OPTIONS]\nExperimental scenery sandbox using Tenchu 1 gameplay. Map 7 is empty. Uses the map's first T2 starting position by default; --sequence selects another layout and its map. T2 story, enemies, scripted objects and water behavior are not ported. F1 settings/free camera work, F5 restarts, Esc returns to the normal launcher.";
    if args.len() < 2 || args[1] == "--help" {
        println!("{help}");
        return Ok(());
    }
    if args[0] == "inspect-t2" && args.len() == 2 {
        let disc = crate::source::open_disc(args[1].as_ref())?;
        for id in Level::available(&disc)? {
            let level = Level::load(&disc, id)?;
            println!(
                "map {id}: {} terrain chunks, {} static props, {} unported effects",
                level.map.len(),
                level.placements.len(),
                level.omitted_effects
            );
        }
        for layout in Layout::available(&disc)? {
            println!(
                "sequence {}: map {}, start {:?}, {} actor placements",
                layout.sequence,
                layout.map,
                layout.start,
                layout.actors.len()
            );
        }
        return Ok(());
    }
    let render = args[0] == "render-t2";
    let first = if render { 4 } else { 3 };
    if args.len() < first {
        return Err(help.into());
    }
    let mut map = None;
    let mut sequence = None;
    let mut guard = None;
    let mut options = Vec::new();
    for pair in args[first..].chunks(2) {
        if pair.len() != 2 {
            return Err("Sandbox options require a value".into());
        }
        let value = pair[1].to_str().ok_or("Invalid sandbox option")?;
        match pair[0].to_str() {
            Some("--map") => map = Some(value.parse::<usize>().map_err(|e| e.to_string())?),
            Some("--sequence") => {
                sequence = Some(value.parse::<usize>().map_err(|e| e.to_string())?)
            }
            Some("--guards") => {
                guard = Some(match value {
                    "sword" => 0x10,
                    "spear" => 0x12,
                    "archer" => 0x11,
                    _ => return Err("--guards needs sword, spear or archer".into()),
                })
            }
            Some("--character" | "--difficulty" | "--language") => options.extend_from_slice(pair),
            _ => {
                return Err(format!(
                    "Unknown sandbox option {}",
                    pair[0].to_string_lossy()
                ));
            }
        }
    }
    let options = Options::parse(&options)?;
    let disc = crate::source::open_disc(args[2].as_ref())?;
    let sandbox = Sandbox::open(&disc, map, sequence, guard)?;
    let map = sandbox.level.id;
    eprintln!(
        "Tenchu 2 map {map}: {} terrain chunks, {} static props; {} procedural effects not imported",
        sandbox.level.map.len(),
        sandbox.level.placements.len(),
        sandbox.level.omitted_effects
    );
    let assets = crate::startup::open(args[1].as_ref())?;
    if render {
        let mut game = sandbox.load(&assets, options)?;
        for _ in 0..30 {
            game.step(0)?;
        }
        crate::capture::frame(&game, args[3].as_ref(), &assets)
    } else {
        crate::window::run(
            assets,
            options,
            args[1].as_ref(),
            None,
            None,
            crate::window::Startup {
                sandbox: Some(sandbox),
                difficulty: Some(options.difficulty),
                ..Default::default()
            },
        )
    }
}
