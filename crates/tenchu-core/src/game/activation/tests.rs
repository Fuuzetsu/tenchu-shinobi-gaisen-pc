use super::*;
use crate::oracle;
use std::fmt::Write;

#[test]
fn budget_retains_its_previous_value_in_the_full_buffer_branch() {
    let mut activation = Activation {
        budget: 6,
        ..Activation::default()
    };
    for expected in [5, 4, 3, 3] {
        activation.update_budget(65536);
        assert_eq!(activation.budget, expected);
    }
    for (bytes, expected) in [
        (25536, 6),
        (25537, 5),
        (30536, 5),
        (30537, 4),
        (35536, 4),
        (35537, 3),
    ] {
        activation.update_budget(bytes);
        assert_eq!(activation.budget, expected);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_DECOMP and a C compiler; runs original ActivateHumans"]
fn activation_order_and_parking_match_original_c() {
    let source = oracle::source("WORLD.C");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
typedef int32_t s32; typedef uint32_t u32; typedef int16_t s16; typedef uint16_t u16;
typedef struct {{ s32 vx, vy, vz; }} VECTOR;
typedef struct {{ struct {{ s32 t[3]; }} coord; }} Coordinate;
typedef struct Model {{ Coordinate locate; u16 attribute; struct Model *object[1]; }} Model;
typedef struct {{ s16 mid; }} Motion;
typedef struct Humanoid {{ s16 type, life, status; u16 attribute; VECTOR *locate; Model *model; Motion *motion; s32 point[2]; }} Humanoid;
typedef struct {{ s16 stage, chrid; VECTOR position; }} Story;
Humanoid actors[40], *HumanGroup[40], *StagePlayer, *VISIBLE_CHARACTERS_ON_STAGE_[41];
Model models[40], waists[40]; Motion motions[40];
struct {{ Humanoid *Owner; }} CamState;
Story StageChar[3];
s32 PacketUsed, GameClock, StageID, VISIBLE_ENEMIES_;
s16 ThinkBudgetRaw, ThinkCount, ThinkBudget, Humans, SkipFrame;
int distances[40], home_distances[40], levels[40], current;
void *GlobalAreaMap;
#define THINK_PACKET_LIMIT 60536
#define THINK_PACKET_COST 5000
#define HUMAN_ACTIVATION_PERIOD 30
#define ACTIVATE_RADIUS_WIDE 26000
#define ACTIVATE_RADIUS 13000
#define DEACTIVATE_RADIUS 17000
#define MOT_ITEM_SHINSOKU 0xf05
#define PAGE_MASK 0xf0
#define PAGE_BOSS 0x80
#define NINKEN 0xa9
#define STAGE_ID_TRAINING 8
#define STAGE_ID_RECLAIM_CASTLE 6
#define STAGE_ID_FREE_PRINCESS 7
#define NINJA_0 0x87
#define GOO 0x83
#define ON 0x84
#define ATTR_SUSPEND 0x80
#define ATTR_FLOAT 0x20
#define MODEL_PART_WAIST 0
#define MODEL_ATTR_COLLIDE 0x4000
#define STAGE_CHAR_END -1
#define STAGE_NUMBER(x) ((x) + 1)
#define STAGE_CHARACTER_POSITION_SCALE 1000
#define HOME_LEVEL_PROBE_DEPTH 1500
#define HUMANOID_HOME_X 0
#define HUMANOID_HOME_Z 1
#define STAT_DEAD 0x11
#define AREA_LEVEL_STEP_DOWN 1
#define LEVEL_NONE INT32_MIN
enum human_activation_state {{ HUMAN_INACTIVE, HUMAN_ACTIVE }};
s32 GetVectorDistance(VECTOR *a, VECTOR *b) {{
    for (int i = 0; i < Humans; ++i) if (a == actors[i].locate) {{ current = i; return distances[i]; }}
    return home_distances[current];
}}
s32 GetAreaMapLevel(void *map, s32 x, s32 y, s32 z, s32 flags) {{ return levels[current]; }}
{}
int main(void) {{
    int clock, stage, packet, budget, raw, count, action, n;
    while (scanf("%d%d%d%d%d%d%d%d", &clock, &stage, &packet, &budget, &raw, &count, &action, &n) == 8) {{
        GameClock = clock; StageID = stage; PacketUsed = packet; ThinkBudget = budget;
        ThinkBudgetRaw = raw; ThinkCount = count; Humans = n; VISIBLE_ENEMIES_ = 0;
        for (int i = 0; i < n; ++i) {{
            int kind, life, status, attr, collision, x, y, z, hx, hz, visible;
            scanf("%d%d%d%d%d%d%d%d%d%d%d%d%d%d", &kind, &life, &status, &attr, &collision, &x, &y, &z, &hx, &hz, &visible, &distances[i], &home_distances[i], &levels[i]);
            models[i].locate.coord.t[0] = x; models[i].locate.coord.t[1] = y; models[i].locate.coord.t[2] = z;
            models[i].object[0] = &waists[i]; waists[i].attribute = collision;
            actors[i] = (Humanoid){{ kind, life, status, attr, (VECTOR*)models[i].locate.coord.t, &models[i], &motions[i], {{ hx, hz }} }};
            HumanGroup[i] = &actors[i];
            if (visible) VISIBLE_CHARACTERS_ON_STAGE_[VISIBLE_ENEMIES_++] = &actors[i];
        }}
        VISIBLE_CHARACTERS_ON_STAGE_[VISIBLE_ENEMIES_] = 0;
        StageChar[0] = (Story){{ stage + 1, 0x83, {{ 7, -3, 11 }} }};
        StageChar[1] = (Story){{ stage + 1, 0x87, {{ -9, -2, 13 }} }};
        StageChar[2].stage = -1;
        StagePlayer = CamState.Owner = &actors[0]; motions[0].mid = action;
        ActivateHumans();
        printf("%d %d %d", ThinkBudgetRaw, ThinkBudget, ThinkCount);
        for (int i = 0; i < n; ++i) printf(" %d %d %d %d %d %d", actors[i].attribute, actors[i].life, waists[i].attribute, actors[i].locate->vx, actors[i].locate->vy, actors[i].locate->vz);
        puts("");
    }}
}}
"#,
        oracle::function(&source, "void ActivateHumans(void)")
    );
    let mut game = oracle::game(0, 0);
    game.mission.story = [(0x83, [7000, -3000, 11000]), (0x87, [-9000, -2000, 13000])]
        .into_iter()
        .map(|(kind, position)| tenchu_assets::executable::StoryActor {
            kind,
            position,
            facing: 0,
            think: 0,
        })
        .collect();
    let player_position = game.player().position;
    let mut random = 3u32;
    let mut next = || {
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        random.rotate_right(16)
    };
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..4096 {
        game.clock = [29, 30, 60, 90, 120, 122][case % 6];
        game.mission.stage.id = [0, 6, 7, 8][case % 4];
        game.drawing.sampled_bytes = next() % 65537;
        game.activation = Activation {
            budget: [0, 3, 4, 5, 6, -1, i16::MAX, i16::MIN][case % 8],
            raw_budget: -7,
            count: 9,
        };
        let action = if case & 8 == 0 { 0xf05 } else { 0x500 };
        game.roster.get_mut(game.player_id).motion.action = action;
        game.visible.clear();
        writeln!(
            input,
            "{} {} {} {} {} {} {} {}",
            game.clock,
            game.mission.stage.id,
            game.drawing.sampled_bytes,
            game.activation.budget,
            game.activation.raw_budget,
            game.activation.count,
            action,
            game.roster.order.len()
        )
        .unwrap();
        for (i, &id) in game.roster.order.iter().enumerate() {
            let actor = game.roster.actors[id].as_mut().unwrap();
            actor.kind = [0x10, 0x84, 0x83, 0x87, 0xa9, 0x90, 0x81][(next() % 7) as usize];
            actor.health = [-1, 0, 1, 100][(next() % 4) as usize];
            actor.status = if next() & 2 != 0 { 0x11 } else { 0 };
            actor.attribute = next() as u16;
            actor.position = if i == 0 {
                player_position
            } else {
                std::array::from_fn(|axis| player_position[axis] + (next() % 42001) as i32 - 21000)
            };
            actor.home = [
                actor.position[0] + (next() % 20000) as i32,
                actor.position[2],
            ];
            let waist = &mut game.scene.models[actor.parts[0]];
            waist.attribute = next() as u16;
            let collision = waist.attribute;
            game.scene.set_translation(actor.root_model, actor.position);
            game.scene.models[actor.root_model].flags = 777;
            let visible = next() & 4 != 0;
            if visible {
                game.visible.push(id);
            }
            let distance = game.math.vector_distance(std::array::from_fn(|axis| {
                actor.position[axis].wrapping_sub(player_position[axis])
            }));
            let home = [actor.home[0], actor.position[1] - 1500, actor.home[1]];
            let home_distance = game.math.vector_distance(std::array::from_fn(|axis| {
                home[axis].wrapping_sub(player_position[axis])
            }));
            let level = terrain::level(
                &game.mission.terrain,
                &mut game.cursor,
                home,
                terrain::STEP_DOWN,
            )
            .unwrap();
            writeln!(
                input,
                "{} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                actor.kind,
                actor.health,
                actor.status,
                actor.attribute,
                collision,
                actor.position[0],
                actor.position[1],
                actor.position[2],
                actor.home[0],
                actor.home[1],
                i32::from(visible),
                distance,
                home_distance,
                level
            )
            .unwrap();
        }
        game.activate().unwrap();
        let mut row = vec![
            i32::from(game.activation.raw_budget),
            i32::from(game.activation.budget),
            i32::from(game.activation.count),
        ];
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            row.extend([
                i32::from(actor.attribute),
                i32::from(actor.health),
                i32::from(game.scene.models[actor.parts[0]].attribute),
            ]);
            row.extend(actor.position);
            assert_eq!(game.scene.models[actor.root_model].flags, 777);
            assert_eq!(
                game.scene.models[actor.root_model].local.translation,
                actor.position
            );
        }
        expected.push(row);
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (expected, actual)) in expected.iter().zip(&actual).enumerate() {
        assert_eq!(expected, actual, "activation case {case}");
    }
}
