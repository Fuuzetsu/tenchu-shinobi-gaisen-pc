use serde_json::{Value, json};
use tenchu_assets::{
    tenchu2::collision::{Cell, Collision},
    terrain::Node,
};
use tenchu_core::{
    Result,
    math::{Math, Matrix, Position},
    random::Random,
    scene::Scene,
    tenchu2::{
        collision,
        environment::{
            self, Context, Environment, Event, Object, SurfaceActor, SurfaceContext, Trigger,
        },
        terrain::{Cursor, Probe},
    },
};

#[derive(Default)]
struct Events(Vec<Value>, i32);
impl environment::Events for Events {
    fn emit(&mut self, event: Event, _: &mut Random) -> Result<()> {
        if let Event::Debris(d) = &event {
            self.1 = d.age;
        }
        self.0.push(match event {
            Event::Explosion { position, small } => {
                json!(["explosion", position.to_array(), small])
            }
            Event::Pickup { position, kind } => json!(["pickup", position.to_array(), kind]),
            Event::Sound { position, id } => json!(["sound", position.to_array(), id]),
            Event::Rumble {
                actor,
                kind,
                frames,
            } => json!(["pause", actor == 0, kind, frames]),
            Event::Shoot {
                kind,
                position,
                velocity,
            } => json!([
                "shoot",
                format!("{kind:?}"),
                position.to_array(),
                velocity.to_array()
            ]),
            Event::Debris(d) => json!([
                "debris",
                d.model,
                d.position.to_array(),
                d.extent.to_array(),
                d.velocity.to_array(),
                d.speed,
                d.gravity
            ]),
            Event::Alert { position, kind } => json!(["alert", position.to_array(), kind]),
            Event::Burn {
                actor,
                interval,
                duration,
            } => json!(["burn", actor, interval, duration]),
        });
        Ok(())
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_OBJECTS_ORACLE from original surface/trap routines"]
fn native_surface_traps_and_object_motion_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_OBJECTS_ORACLE") {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let flag = |key: &str| row[key].as_bool().unwrap();
        let mut scene = Scene::default();
        let mut collisions = collision::Collision::default();
        let objects = row["objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let int = |key: &str| r[key].as_i64().unwrap() as i32;
                let coordinate = scene.add(
                    None,
                    Matrix {
                        translation: super::source_physics::vector(&r["coordinate"]),
                        ..Matrix::default()
                    },
                );
                collisions.insert(coordinate).unwrap().flags = 0x1000;
                Object {
                    cell: 0,
                    coordinate,
                    position: super::source_physics::vector(&r["position"]),
                    yaw: int("yaw") as i16,
                    roll: int("roll"),
                    model: usize::try_from(int("model")).ok(),
                    kind: int("kind") as i16,
                    link: int("link") as i16,
                    clock: int("clock") as i16,
                    actor: Some(0),
                    fall_yaw: None,
                }
            })
            .collect();
        let t = &row["trigger"];
        let trigger = Trigger {
            position: super::source_physics::vector(&t["position"]),
            angle: t["angle"].as_i64().unwrap() as i16,
            kind: t["kind"].as_i64().unwrap() as i16,
            target: t["target"].as_u64().unwrap() as u16,
            clock: t["clock"].as_u64().unwrap() as u16,
        };
        let mut environment = Environment {
            objects,
            triggers: vec![
                trigger,
                Trigger {
                    position: Position::default(),
                    angle: 0,
                    kind: -1,
                    target: 0,
                    clock: 0,
                },
            ],
            ..Environment::default()
        };
        let initial = Node {
            y: row["floor"].as_i64().unwrap_or(0) as i16,
            dy: 0,
            bounds: [-3000, -3000, 3000, 3000],
            attributes: 0,
            division: u16::MAX,
        };
        let mut map = Collision {
            gaisen: None,
            origin: [-3000, -3000],
            dimensions: [1, 1],
            cell_size: 6000,
            cells: vec![Cell {
                minimum: i16::MIN,
                maximum: i16::MAX,
                starts: [0; 4],
                nodes: if row["floor"].is_null() {
                    vec![]
                } else {
                    vec![initial]
                },
            }],
        };
        let mut cursor = Cursor::default();
        let mut random = Random::new(1);
        let mut events = Events::default();
        let mut context = Context {
            map: &mut map,
            cursor: &mut cursor,
            collision: &mut collisions,
            scene: &mut scene,
            math: &math,
            random: &mut random,
            events: &mut events,
        };
        let result = if row["op"] == "surface" {
            Some(
                environment
                    .surface(
                        SurfaceActor {
                            index: 0,
                            position: super::source_physics::vector(&row["position"]),
                            height: int("height") as i16,
                            health: int("health") as i16,
                            visible: flag("visible"),
                            focused: flag("focus"),
                        },
                        &Probe {
                            floor: Some(int("probe_floor")),
                            height: int("probe_height"),
                            material: int("material") as u16,
                            ..Probe::default()
                        },
                        flag("hidden"),
                        &mut SurfaceContext {
                            map: context.map,
                            cursor: context.cursor,
                            math: context.math,
                            random: context.random,
                            events: context.events,
                        },
                    )
                    .unwrap(),
            )
        } else {
            environment.update_triggers(&mut context).unwrap();
            None
        };
        let objects:Vec<_>=environment.objects.iter().map(|o|json!({"position":o.position.to_array(),"coordinate":scene.models[o.coordinate].local.translation.to_array(),"model":o.model.map_or(-1,|n|n as i32),"yaw":o.yaw,"roll":o.roll,"kind":o.kind,"link":o.link,"clock":o.clock,"actor":o.actor==Some(0),"flags":collisions.get_mut(o.coordinate).unwrap().flags})).collect();
        let node = map.cells[0].nodes.first().unwrap_or(&initial);
        let actual = json!({"result":result,"objects":objects,"trigger_position":environment.triggers[0].position.to_array(),"clock":environment.triggers[0].clock,"node":[node.y,node.dy,node.bounds[0],node.bounds[1],node.bounds[2],node.bounds[3],node.attributes as i16,node.division as i16],"background":environment.background_only,"events":events.0});
        assert_eq!(actual, row["expected"], "case {count}: {row}");
        count += 1;
    }
    assert_eq!(count, 10000);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_OBJECT_CONTACTS_ORACLE from original object contacts"]
fn native_doors_breakables_and_hazards_match_source_contacts() {
    use tenchu_assets::tenchu2::level::objects::Bounds;
    use tenchu_core::math::Vector3;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_OBJECT_CONTACTS_ORACLE") {
        let mut scene = Scene::default();
        let mut objects = Vec::new();
        let mut collision = serde_json::to_value(collision::Collision::default()).unwrap();
        collision["len"] = json!(2);
        for (i, r) in row["objects"].as_array().unwrap().iter().enumerate() {
            let int = |key: &str| r[key].as_i64().unwrap() as i32;
            let coordinate = scene.add(
                None,
                Matrix {
                    translation: super::source_physics::vector(&r["coordinate"]),
                    ..Matrix::default()
                },
            );
            objects.push(Object {
                cell: i,
                coordinate,
                position: super::source_physics::vector(&r["position"]),
                yaw: int("yaw") as i16,
                roll: int("roll"),
                model: usize::try_from(int("model")).ok(),
                kind: int("kind") as i16,
                link: int("link") as i16,
                clock: int("clock") as i16,
                actor: None,
                fall_yaw: None,
            });
            let owner = if i == 0 {
                None
            } else {
                Some(usize::from(!row["owner"].as_bool().unwrap()))
            };
            collision["slots"][i] = json!({"model":coordinate,"owner":owner,"position":if i==0 {Position::default()}else{super::source_physics::vector::<i32>(&row["distance"])},"offset":Vector3::<i16>::default(),"size":Vector3::<i16>::default(),"flags":if i==0 {0x1000}else{row["flags"].as_u64().unwrap()},"contacts":if i==0 {2}else{0}});
        }
        let mut collision: collision::Collision = serde_json::from_value(collision).unwrap();
        let t = &row["trigger"];
        let mut environment = Environment {
            objects,
            triggers: vec![
                Trigger {
                    position: Position::default(),
                    angle: 0,
                    kind: t["kind"].as_i64().unwrap() as i16,
                    target: t["target"].as_u64().unwrap() as u16,
                    clock: t["clock"].as_u64().unwrap() as u16,
                },
                Trigger {
                    position: Position::default(),
                    angle: 0,
                    kind: -1,
                    target: 0,
                    clock: 0,
                },
            ],
            ..Environment::default()
        };
        let initial = Node {
            y: 0,
            dy: 0,
            bounds: [-3000, -3000, 3000, 3000],
            attributes: 0x100,
            division: u16::MAX,
        };
        let mut map = Collision {
            gaisen: None,
            origin: [-3000, -3000],
            dimensions: [1, 1],
            cell_size: 6000,
            cells: vec![Cell {
                minimum: i16::MIN,
                maximum: i16::MAX,
                starts: [0; 4],
                nodes: vec![initial],
            }],
        };
        let mut cursor = Cursor::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut events = Events::default();
        let b: Vec<i16> = row["bounds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_i64().unwrap() as i16)
            .collect();
        let bounds = Bounds {
            offset: [b[0], b[1], b[2]],
            top: b[3],
            half_width: b[4],
            half_depth: b[5],
        };
        environment
            .update_contacts(
                &[bounds.clone(), bounds],
                (0, Position::new(1000, 0, 500)),
                |object| object.cell == 0,
                &mut Context {
                    map: &mut map,
                    cursor: &mut cursor,
                    collision: &mut collision,
                    scene: &mut scene,
                    math: &math,
                    random: &mut random,
                    events: &mut events,
                },
            )
            .unwrap();
        let objects:Vec<_>=environment.objects.iter().map(|o|json!({"position":o.position.to_array(),"coordinate":scene.models[o.coordinate].local.translation.to_array(),"model":o.model.map_or(-1,|n|n as i32),"yaw":o.yaw,"roll":o.roll,"kind":o.kind,"link":o.link,"clock":o.clock,"actor":o.actor.map(|i|i==0)})).collect();
        let node = &map.cells[0].nodes[0];
        let actual = json!({"objects":objects,"clock":environment.triggers[0].clock,"node":[node.y,node.dy,node.bounds[0],node.bounds[1],node.bounds[2],node.bounds[3],node.attributes as i16,node.division as i16],"events":events.0,"flags":collision.slot(0).unwrap().flags,"seed":random.seed,"age":events.1});
        assert_eq!(actual, row["expected"], "case {count}: {row}");
        count += 1;
    }
    assert_eq!(count, 6000);
}
