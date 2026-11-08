use serde_json::{Value, json};
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::{
    math::Position,
    random::Random,
    scene::Scene,
    tenchu2::{
        collision::Collision,
        items::{
            Context,
            resurrection::{self, Resurrection, ReturnPoint},
        },
        terrain::Cursor,
    },
};

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_RESURRECTION_ORACLE; original return-point, log and transition sequences"]
fn native_resurrection_matches_original_return_and_fade() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let resources = super::source_control::resources();
    let template = super::source_control::template(&resources);
    let math = &resources.math;
    let mut maps = std::collections::BTreeMap::new();
    let mut count = 0;
    for (case, row) in
        super::source_control::cases::<Value>("TENCHU2_RESURRECTION_ORACLE").enumerate()
    {
        let map = maps
            .entry(row["map"].as_u64().unwrap() as usize)
            .or_insert_with_key(|id| Level::load(&disc, *id).unwrap().collision);
        let position = |key: &str| {
            Position::from(serde_json::from_value::<[i32; 3]>(row[key].clone()).unwrap())
        };
        let mut actor = template.clone();
        actor.position = position("origin");
        actor.rotation.y = row["yaw"].as_i64().unwrap() as i16;
        actor.max_health = row["maxhp"].as_i64().unwrap() as i16;
        let markers: Vec<Option<[i32; 3]>> =
            serde_json::from_value(row["markers"].clone()).unwrap();
        let destination = resurrection::destination(
            actor.position,
            position("home"),
            markers.into_iter().flatten().map(Position::from),
        );
        assert_eq!(json!(destination), row["position"], "destination {case}");
        let mut random = Random::new(row["seed"].as_u64().unwrap() as u32);
        let mut scene = Scene::default();
        let mut collision = Collision::default();
        let mut cursor = Cursor::default();
        let mut context = Context {
            map,
            cursor: &mut cursor,
            math,
            scene: &mut scene,
            collision: &mut collision,
            player: 0,
        };
        let initial = Resurrection::new(
            0,
            serde_json::from_value(row["initial"]["attacker"].clone()).unwrap(),
            &actor,
            &mut context,
            &mut random,
        )
        .unwrap();
        assert_eq!(json!(initial), row["initial"], "constructor {case}");
        assert_eq!(
            json!(random.seed),
            row["seed_after"],
            "constructor RNG {case}"
        );
        let mut resurrection: Resurrection = serde_json::from_value(row["before"].clone()).unwrap();
        let mut point: ReturnPoint = serde_json::from_value(row["point"].clone()).unwrap();
        actor.health = row["health_before"].as_i64().unwrap() as i16;
        actor.position = destination;
        for (tick, expected) in row["updates"].as_array().unwrap().iter().enumerate() {
            let update = resurrection.advance(
                &mut actor,
                &mut point,
                expected["ready"].as_bool().unwrap(),
                math,
                &mut random,
            );
            assert_eq!(
                json!(resurrection),
                expected["state"],
                "state {case}:{tick}"
            );
            assert_eq!(json!(point), expected["point"], "shared {case}:{tick}");
            for (name, value) in [
                ("alive", json!(update.alive)),
                ("fade", json!(update.fade)),
                ("voice", json!(update.voice)),
                ("smoke", json!(update.smoke)),
                ("returned", json!(update.returned)),
                ("health", json!(actor.health)),
                ("position", json!(actor.position)),
                ("seed", json!(random.seed)),
            ] {
                assert_eq!(value, expected[name], "{name} {case}:{tick}");
            }
        }
        count += 1;
    }
    assert_eq!(count, 1000);
}
