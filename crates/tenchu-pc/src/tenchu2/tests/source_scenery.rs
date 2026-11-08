use serde_json::{Value, json};
use tenchu_assets::{
    tenchu2::{
        effects::Sprites,
        level::{
            Level,
            objects::{Bounds, Static, StaticKind},
        },
    },
    texture::{Image, Vram},
};
use tenchu_core::{
    Result,
    math::{Math, Matrix, Position},
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        environment::{
            self, Context, Environment, Event, Object,
            scenery::{Fire, Input},
        },
        terrain::Cursor,
        visibility::View,
    },
};

#[derive(Default)]
struct Events(Vec<Value>);
impl environment::Events for Events {
    fn emit(&mut self, event: Event, _: &mut Random) -> Result<()> {
        if let Event::Sound { position, id } = event {
            self.0.push(json!([position.to_array(), id]));
        } else {
            panic!("Unexpected scenery event: {event:?}");
        }
        Ok(())
    }
}

fn sprites() -> Sprites {
    let image = |id| Image {
        page: 0,
        palette: 0,
        uv: [id, 0],
        size: [32, 64],
    };
    Sprites {
        vram: Vram::default(),
        effects: vec![image(0); 36],
        scenery: [8, 2, 4, 1, 3, 7, 6, 5].map(image).to_vec(),
    }
}

fn view(position: Position) -> View {
    View {
        eye: position.wrapping_sub(Position::new(0, 0, 1000)),
        look: position,
        focus: position,
        matrix: Matrix::default(),
        distance: 16000,
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SCENERY_EFFECTS_ORACLE from original procedural scenery updates"]
fn native_scenery_phases_fire_collision_and_ambient_cues_match_source() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut level = Level::load(&disc, 0).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let sprites = sprites();
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_SCENERY_EFFECTS_ORACLE") {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let position = super::source_physics::vector(&row["position"]);
        level.scene.statics = vec![Static {
            position: position.to_array(),
            rotation: [0; 3],
            kind: StaticKind::Effect(int("kind") as u16),
        }];
        let mut environment = Environment {
            static_phases: vec![int("phase") as i16],
            fire: Fire {
                coordinate: None,
                previous: int("previous") as i16,
            },
            last_sound_vblank: int("before_vblank") as u16,
            ..Environment::default()
        };
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        for _ in 0..int("slots") {
            let id = scene.add(None, Matrix::default());
            collision.insert(id).unwrap();
        }
        let mut terrain = level.collision.clone();
        let mut cursor = Cursor::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut events = Events::default();
        let frame = environment
            .update_scenery(
                &Input {
                    level: &level,
                    sprites: &sprites,
                    view: view(position),
                    sequence: int("sequence") as u16,
                    vblank: int("vblank") as u16,
                    actors: &[],
                },
                &mut Context {
                    map: &mut terrain,
                    cursor: &mut cursor,
                    collision: &mut collision,
                    scene: &mut scene,
                    math: &math,
                    random: &mut random,
                    events: &mut events,
                },
            )
            .unwrap();
        let fire = environment.fire.coordinate.map(|id| { let index=collision.index(id).unwrap(); let slot=collision.slot(index).unwrap(); json!({"index":index,"position":scene.models[id].local.translation.to_array(),"flags":slot.flags,"size":slot.size.to_array()}) });
        let sprites: Vec<_> = frame.sprites.iter().map(|s| json!({"image":s.image.uv[0],"position":s.position.to_array(),"scale":s.scale.map(|v| {let n=(v*3600/400) as i16; if n<0 {i16::MAX}else{n}}),"color":s.color,"rotation":s.rotation,"blend":s.blend})).collect();
        let actual = json!({"phase":environment.static_phases[0],"last_vblank":environment.last_sound_vblank,"seed":random.seed,"sprites":sprites,"sounds":events.0,"fire":fire});
        assert_eq!(actual, row["expected"], "case {count}: {row}");
        count += 1;
    }
    assert_eq!(count, 4000);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_FALLING_PROP_ORACLE from original falling scenery update"]
fn native_falling_props_preserve_the_stump_and_source_transform() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut level = Level::load(&disc, 0).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    level.scene.statics.clear();
    level.object_sizes = vec![1000];
    level.scene.bounds = vec![Bounds {
        offset: [0; 3],
        top: -1000,
        half_width: 250,
        half_depth: 250,
    }];
    level.grid.cells = vec![Some(0)];
    let sprites = sprites();
    let mut count = 0;
    for row in super::source_control::cases::<Value>("TENCHU2_FALLING_PROP_ORACLE") {
        let int = |key: &str| row[key].as_i64().unwrap() as i32;
        let position = super::source_physics::vector(&row["position"]);
        let mut scene = Scene::default();
        let coordinate = scene.add(
            None,
            Matrix {
                translation: position,
                ..Matrix::default()
            },
        );
        let mut environment = Environment {
            objects: vec![Object {
                coordinate,
                cell: 0,
                position,
                yaw: int("yaw") as i16,
                roll: 0,
                model: Some(0),
                kind: 14,
                link: 1,
                clock: int("clock") as i16 - 1,
                actor: None,
                fall_yaw: Some(int("fall_yaw") as i16),
            }],
            ..Environment::default()
        };
        let mut terrain = level.collision.clone();
        let mut cursor = Cursor::default();
        let mut collision = Collision::default();
        let mut random = Random::new(0);
        let mut events = Events::default();
        let frame = environment
            .update_scenery(
                &Input {
                    level: &level,
                    sprites: &sprites,
                    view: view(position),
                    sequence: 1,
                    vblank: 0,
                    actors: &[],
                },
                &mut Context {
                    map: &mut terrain,
                    cursor: &mut cursor,
                    collision: &mut collision,
                    scene: &mut scene,
                    math: &math,
                    random: &mut random,
                    events: &mut events,
                },
            )
            .unwrap();
        let draws: Vec<_> = frame
            .models
            .iter()
            .map(
                |m| json!({"stump":m.stump,"position":m.position.to_array(),"rotation":m.rotation}),
            )
            .collect();
        let object = &environment.objects[0];
        let actual = json!({"draws":draws,"sounds":events.0,"freed":object.fall_yaw.is_none(),"kind":object.kind,"model":object.model.map_or(-1,|i|i as i32),"clock":object.clock,"link":object.link});
        assert_eq!(actual, row["expected"], "case {count}: {row}");
        count += 1;
    }
    assert_eq!(count, 1000);
}
