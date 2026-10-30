use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::oracle;

#[test]
fn optional_sight_range_changes_thresholds_without_changing_angles_or_polling() {
    let query = |status, clock| Query {
        position: Position::ZERO,
        delta: Position::new(0, 0, 9000),
        height: 1800,
        player_height: 1800,
        player_status: ActorState::from_source_id(status),
        clock,
        slot: 0,
        alert: 0,
    };
    let sight = Sight {
        distance: 9000,
        direction: 0,
    };
    assert_eq!(
        sight
            .evaluate_range(query(0, 32), 100, |_, _, _| Ok(false))
            .unwrap(),
        Detection::Seen
    );
    assert_eq!(
        sight
            .evaluate_range(query(0, 32), 50, |_, _, _| Ok(false))
            .unwrap(),
        Detection::Unseen
    );
    assert_eq!(
        sight
            .evaluate_range(query(0, 32), 0, |_, _, _| panic!(
                "zero range must not raycast"
            ))
            .unwrap(),
        Detection::Gone
    );
    assert_eq!(
        sight
            .evaluate_range(query(0, 31), 200, |_, _, _| panic!(
                "cadence must not change"
            ))
            .unwrap(),
        Detection::None
    );
    assert_eq!(
        sight
            .evaluate_range(query(0, 32), 200, |_, _, _| Ok(true))
            .unwrap(),
        Detection::Gone
    );
    let sight = Sight {
        distance: 9000,
        direction: 500,
    };
    assert_eq!(
        sight
            .evaluate_range(query(11, 32), 200, |_, _, _| panic!(
                "crouch cone must stay narrow"
            ))
            .unwrap(),
        Detection::Unseen
    );
}

#[test]
#[ignore = "requires TENCHU_DECOMP and a C compiler; compares SearchTarget guards and ray arguments"]
fn sight_guards_and_ray_queries_match_original_c() {
    let source = oracle::source("HUMAN.C");
    let start = source
        .find("typedef struct\n{\n    s32 sight_distance;")
        .unwrap();
    let function = oracle::function(&source, "search_result SearchTarget(");
    let end = source.find(function).unwrap() + function.len();
    let routine = &source[start..end];
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
typedef int32_t s32;
typedef int16_t s16;
typedef uint16_t u16;
typedef s16 search_result;
typedef struct {{ s32 vx, vy, vz; }} VECTOR;
typedef struct {{ s16 vx, vy, vz; }} SVECTOR;
typedef struct {{ struct {{ s32 t[3]; }} coord; }} Coordinate;
typedef struct {{ s16 id; }} Model;
typedef struct {{ Model **object; }} Archive;
typedef struct {{ VECTOR *locate; SVECTOR *rotate; Coordinate *target; Archive *model; s16 height, status; }} Humanoid;
#define MODEL_PART_WAIST 0
#define ANGLE_HALF 2048
#define ANGLE_FULL 4096
#define STAT_SQUAT 11
#define STAT_STICKON 12
#define STAT_HANG 10
#define SR_NONE 0
#define SR_GONE -2
#define SR_UNSEEN -1
#define SR_SEEN 1
#define SR_GLIMPSE 2
static Humanoid *StagePlayer;
static s32 GameClock, EmergencyNotice;
static void *GlobalAreaMap;
static s32 distance_value, angle_value, blocked, calls, ray[7];
static long SquareRoot0(s32 ignored) {{ return distance_value; }}
static long ratan2(s32 y, s32 x) {{ return angle_value; }}
static VECTOR *GetAreaMapPassage(void *map, VECTOR *p, SVECTOR *d, short n) {{
    calls++;
    ray[0] = p->vx; ray[1] = p->vy; ray[2] = p->vz;
    ray[3] = d->vx; ray[4] = d->vy; ray[5] = d->vz; ray[6] = n;
    return blocked ? p : NULL;
}}
{routine}
int main(void) {{
    int in[16];
    while (scanf("%d", &in[0]) == 1) {{
        for (int j = 1; j < 16; j++) if (scanf("%d", &in[j]) != 1) return 1;
        VECTOR position = {{in[0], in[1], in[2]}};
        Coordinate target = {{{{in[3], in[4], in[5]}}}};
        SVECTOR rotation = {{0, in[6], 0}};
        Model waist = {{in[8]}}; Model *objects[] = {{&waist}};
        Archive model = {{objects}};
        Humanoid actor = {{&position, &rotation, &target, &model, in[7], 0}}, player = {{0}};
        player.height = in[9]; player.status = in[10]; StagePlayer = &player;
        GameClock = in[11]; EmergencyNotice = in[12];
        distance_value = in[13]; angle_value = in[14]; blocked = in[15];
        calls = 0; for (int j = 0; j < 7; j++) ray[j] = 0;
        long distance; short degree;
        int result = SearchTarget(&actor, &distance, &degree);
        printf("%d %ld %d %d", result, distance, degree, calls);
        for (int j = 0; j < 7; j++) printf(" %d", ray[j]);
        puts("");
    }}
}}
"#
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    for status in [0, 10, 11, 12, 16] {
        for vertical in [-3001, -3000, -2999, -1, 0, 1, 2999, 3000] {
            for distance in [
                0, 3999, 4000, 6999, 7000, 9999, 10000, 11999, 12000, 15999, 16000, 19999, 20000,
            ] {
                for angle in [-2048, -900, -899, -450, -449, 0, 449, 450, 899, 900, 2048] {
                    for flags in 0..8 {
                        let position = [123, -16000, -567];
                        let target = [124 - distance, -16000 + vertical, -568 + distance / 2];
                        let clock = if flags & 1 == 0 { 29 } else { 30 };
                        let alert = (flags >> 1) & 1;
                        let blocked = flags & 4 != 0;
                        let sight = Sight {
                            distance,
                            direction: if angle == -2048 { 2048 } else { angle },
                        };
                        let query = Query {
                            position: position.into(),
                            delta: Position::from_fn(|i| target[i] - position[i]),
                            height: 2300,
                            player_height: 1901,
                            player_status: ActorState::from_source_id(status),
                            clock,
                            slot: 3,
                            alert,
                        };
                        let mut calls = 0;
                        let mut ray = [0; 7];
                        let result = sight
                            .evaluate(query, |p, d, n| {
                                calls += 1;
                                ray = [
                                    p[0],
                                    p[1],
                                    p[2],
                                    i32::from(d[0]),
                                    i32::from(d[1]),
                                    i32::from(d[2]),
                                    i32::from(n),
                                ];
                                Ok(blocked)
                            })
                            .unwrap();
                        let fields = position
                            .into_iter()
                            .chain(target)
                            .chain([
                                0,
                                2300,
                                3,
                                1901,
                                i32::from(status),
                                clock,
                                alert,
                                distance,
                                i32::from(angle),
                                i32::from(blocked),
                            ])
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>();
                        input.push_str(&fields.join(" "));
                        input.push('\n');
                        expected.push(
                            [
                                result.source_value(),
                                distance,
                                i32::from(sight.direction),
                                calls,
                            ]
                            .into_iter()
                            .chain(ray)
                            .collect::<Vec<_>>(),
                        );
                    }
                }
            }
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), 45760);
    for (case, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, &expected, "sight case {case}");
    }
}
