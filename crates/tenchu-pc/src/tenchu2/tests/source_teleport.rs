use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::{Math, Position},
    random::Random,
    scene::Scene,
    tenchu2::{
        actor::State,
        collision::Collision,
        effects::{
            Effect, Effects,
            paper::{Motion, Paper},
        },
        items::{
            Context,
            teleport::{self, Teleport},
        },
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TELEPORT_TARGET_ORACLE; original teleport eligibility, range and list-order targeting"]
fn native_teleport_target_matches_original() {
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_TELEPORT_TARGET_ORACLE").enumerate()
    {
        let actors: Vec<_> = row["actors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| {
                let mut actor = template.clone();
                actor.kind = source["kind"].as_i64().unwrap() as i16;
                actor.flags = source["flags"].as_u64().unwrap() as u32;
                actor.state =
                    State::from_action((source["state"].as_i64().unwrap() as i16) << 8).unwrap();
                actor.position = vector(&source["position"]);
                actor
            })
            .collect();
        let owner = row["owner"].as_u64().unwrap() as usize;
        let target = teleport::nearest(
            owner,
            &actors[owner],
            actors.iter().enumerate(),
            row["player"].as_u64().unwrap() as usize,
            row["range"].as_i64().unwrap() as i32,
            &resources.math,
        );
        assert_eq!(json!(target), row["target"], "target {case}");
        count += 1;
    }
    assert_eq!(count, 1600);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_TELEPORT_UPDATE_ORACLE; original teleport delay, interruption and destination placement"]
fn native_teleport_placement_matches_original() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_TELEPORT_UPDATE_ORACLE").enumerate()
    {
        let map = maps
            .entry(row["map"].as_u64().unwrap() as usize)
            .or_insert_with_key(|id| Level::load(&disc, *id).unwrap().collision);
        let mut owner = template.clone();
        owner.position = vector(&row["position"]);
        owner.rotation.y = row["yaw"].as_i64().unwrap() as i16;
        owner.flags = row["flags"].as_u64().unwrap() as u32;
        let mut camera = row["camera"].as_i64().unwrap() as i16;
        let mut teleport: Teleport = serde_json::from_value(row["before"].clone()).unwrap();
        let target = teleport.target.map(|_| {
            (
                vector(&row["target_position"]),
                row["target_yaw"].as_i64().unwrap() as i16,
            )
        });
        let alive = teleport
            .advance(
                &mut owner,
                target,
                &mut camera,
                &mut Context {
                    map,
                    cursor: &mut Cursor::default(),
                    math: &resources.math,
                    scene: &mut Scene::default(),
                    collision: &mut Collision::default(),
                    player: 0,
                },
            )
            .unwrap();
        assert_eq!(json!(teleport), row["after"], "state {case}");
        assert_eq!(json!(alive), row["alive"], "expiry {case}");
        assert_eq!(
            json!(owner.position.to_array()),
            row["after_position"],
            "position {case}"
        );
        assert_eq!(json!(owner.rotation.y), row["after_yaw"], "yaw {case}");
        assert_eq!(json!(camera), row["after_camera"], "camera {case}");
        count += 1;
    }
    assert_eq!(count, 600);
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_PAPER_ORACLE; original paper spawning, RNG, pool eviction and motion transitions"]
fn native_paper_particles_match_original() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    let mut count = 0;
    for (case, row) in super::source_control::cases::<Value>("TENCHU2_PAPER_ORACLE").enumerate() {
        let mut effects = Effects::default();
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        for command in row["commands"].as_array().unwrap() {
            let motion = [
                Motion::Outward,
                Motion::Rising,
                Motion::Burst,
                Motion::Scatter,
            ][command["kind"].as_u64().unwrap() as usize];
            effects
                .paper(vector(&command["position"]), motion, &math, &mut random)
                .unwrap();
        }
        let papers: Vec<_> = effects
            .entries
            .iter()
            .map(|e| match e {
                Effect::Paper(p) => p,
                _ => panic!("Unexpected effect"),
            })
            .collect();
        assert_eq!(json!(papers), row["before"], "spawn {case}");
        assert_eq!(json!(random.seed), row["seed_after"], "RNG {case}");
        let mut papers: Vec<Paper> = serde_json::from_value(row["initial"].clone()).unwrap();
        let mut drawings = Vec::new();
        for _ in 0..row["ticks"].as_u64().unwrap() {
            drawings.clear();
            papers.retain_mut(|p| p.update(&math, &mut drawings));
        }
        assert_eq!(json!(papers), row["after"], "motion {case}");
        count += 1;
    }
    assert_eq!(count, 240);
}

fn vector(value: &Value) -> Position {
    serde_json::from_value::<[i32; 3]>(value.clone())
        .unwrap()
        .into()
}
