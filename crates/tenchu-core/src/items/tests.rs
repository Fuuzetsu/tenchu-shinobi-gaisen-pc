use super::*;
use crate::{game::Game, input::UP, oracle, random::Random};
use std::fmt::Write;

fn with_items<T>(game: &mut Game, f: impl FnOnce(&mut Items, &mut Context, &mut Roster) -> T) -> T {
    f(
        &mut game.items,
        &mut Context {
            mission: &game.mission,
            math: &game.math,
            body: &game.body,
            cursor: &mut game.cursor,
            random: &mut game.random,
            effects: &mut game.effects,
            scene: &mut game.scene,
            collision: &mut game.collision,
            camera: &mut game.camera,
            drawing: &mut game.drawing,
        },
        &mut game.roster,
    )
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises pickup through ordinary ticks and allocation/transfer quirks"]
fn pickups_transfer_and_full_pool_replacement_preserve_source_order() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        let position = game.player().position;
        with_items(&mut game, |items, ctx, _| {
            for id in 0..CAPACITY {
                items.dispose(id, ctx).unwrap();
            }
            items.stay(3, position, ctx).unwrap();
        });
        let mut transferred = false;
        for _ in 0..40 {
            game.step(UP).unwrap();
            if game.player().inventory[3] == 1 {
                transferred = true;
                break;
            }
        }
        assert!(
            transferred,
            "character {character} never collected the item"
        );
        assert_eq!(game.player().motion.action, crate::ids::MOT_STATE_PICKUP);
        assert!(game.effects.sounds.iter().any(|(_, sound)| *sound == 0x0d));
        with_items(&mut game, |items, ctx, roster| {
            items.drop(3, Some(player), position, [0; 3], ctx).unwrap();
            let id = items.cursor;
            items.slots[id].mode = 2;
            items.slots[id].count = 9;
            roster.get_mut(player).motion.action = crate::ids::MOT_NORMAL;
            let calls = ctx.random.calls;
            items.pickup(id, ctx, roster, 0).unwrap();
            assert_eq!(ctx.random.calls - calls, 3);
            assert_eq!(roster.get(player).inventory[3], 2);
            assert!(items.slots[id].processor.is_none());

            for id in 0..CAPACITY {
                items.dispose(id, ctx).unwrap();
            }
            items.cursor = 0;
            for _ in 0..CAPACITY {
                assert!(items.drop(1, None, position, [0; 3], ctx).unwrap());
            }
            assert_eq!(items.cursor, 0);
            items.contact(29, 180, crate::collision::SOFT, ctx).unwrap();
            let model = items.slots[29].model;
            assert!(!items.drop(3, None, [999000; 3], [0; 3], ctx).unwrap());
            assert_eq!(items.cursor, 29);
            assert!(items.slots[29].processor.is_none());
            assert!(ctx.scene.models[model].conflict.is_none());
            assert_eq!(
                items.slots.iter().filter(|i| i.processor.is_some()).count(),
                29
            );
        });
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares MoveKorogari with original C using recorded terrain responses"]
fn rolling_matches_original_c_at_terrain_boundaries() {
    let source = oracle::source("ITEM.C");
    let function = oracle::function(&source, "static void MoveKorogari(");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
typedef int32_t s32;
typedef struct {{ int16_t vx,vy,vz,pad; }} SVECTOR;
typedef struct {{ int32_t vx,vy,vz,pad; }} VECTOR;
typedef struct {{ int level,height,attrib,vector; }} MapVector;
typedef struct {{ struct {{ struct {{ int t[3]; }} coord; }} locate; }} ModelType;
typedef struct {{ ModelType *locate; }} TItem;
typedef struct {{ uintptr_t hint; int16_t vx,vy,vz; uint8_t status; }} param_korogari;
#define KORO_NORMAL 0
#define KORO_WATER 1
#define KORO_GRAND 2
#define KORO_WALL 3
#define KORO_STAY 4
#define KORO_OUT 5
#define LEVEL_NONE INT32_MIN
#define AREA_LEVEL_DEFAULT 0
#define MAP_WATER 4
#define FIXED_ONE 4096
#define MODEL_POSITION(m) ((VECTOR *)(m)->locate.coord.t)
#define setVector(v,x,y,z) ((v)->vx=(x),(v)->vy=(y),(v)->vz=(z))
static int GlobalAreaMap,RefrectMove[16][2],a[19],queries,probes,splashes,calls;
static uint32_t seed;
int rand(void) {{ seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767; }}
static int CGetLevel(uintptr_t *hint,int x,int y,int z,int mode) {{
    assert(mode==0);
    int i=queries++;
    assert(i<2);
    assert(x==a[0]+(i==0?a[3]:0) && y==a[1]+(i==0?a[4]:0) && z==a[2]+(i==0?a[5]:0));
    *hint=a[10+i*2];return a[9+i*2];
}}
static void GetAreaMapVector(int map,MapVector *v,VECTOR *p,int width,int mode) {{
    assert(p->vx==a[0] && p->vy==a[1] && p->vz==a[2] && width==500 && mode==0);
    *v=(MapVector){{a[13],a[14],a[15],a[16]}};probes++;
}}
static void SetSplash(VECTOR *p,int x,int y,int speed) {{
    assert(p->vx==a[0] && p->vy==a[1] && p->vz==a[2] && x==8192 && y==8192 && speed==4);splashes++;
}}
{function}
int main(void) {{
    while(scanf("%d",a)==1) {{
        for(int i=1;i<19;i++) assert(scanf("%d",a+i)==1);
        queries=probes=splashes=calls=0;seed=(uint32_t)a[8];
        RefrectMove[a[16]][0]=a[17];RefrectMove[a[16]][1]=a[18];
        ModelType model={{.locate.coord.t={{a[0],a[1],a[2]}}}};
        TItem item={{&model}};param_korogari param={{a[7],a[3],a[4],a[5],a[6]}};
        MoveKorogari(&item,&param);
        printf("%d %d %d %d %d %d %d %d %d %d %d %d %d\n",model.locate.coord.t[0],model.locate.coord.t[1],model.locate.coord.t[2],param.vx,param.vy,param.vz,param.status,(int)param.hint,(int)seed,calls,splashes,queries,probes);
    }}
}}
"#
    );
    let mut game = oracle::game(0, 0);
    let mut cases = Random::new(0x512ea);
    let mut input = String::new();
    let mut expected = Vec::new();
    let nodes: Vec<_> = game
        .mission
        .terrain
        .rows
        .iter()
        .flat_map(|r| (0..r.count).map(move |i| r.list + i * 16))
        .collect();
    for case in 0..4096 {
        let at = nodes[cases.next15() as usize % nodes.len()];
        let node = game.mission.terrain.node(at).unwrap();
        let mut position = [
            i32::from(node.bounds[0]) * 10,
            i32::from(node.y) * 10,
            i32::from(node.bounds[1]) * 10,
        ];
        position[0] += cases.next15() % 3000 - 1500;
        position[1] += cases.next15() % 3000 - 1500;
        position[2] += cases.next15() % 3000 - 1500;
        let mut rolling = Rolling {
            hint: if case % 3 == 0 { Some(at) } else { None },
            velocity: std::array::from_fn(|_| (cases.next15() as u16 * 2) as i16),
            status: (case % 6) as u8,
        };
        if case % 2 == 0 {
            rolling.velocity = std::array::from_fn(|_| (cases.next15() % 300 - 150) as i16);
        }
        let seed = cases.next15() as u32 * 65536 + cases.next15() as u32;
        let next =
            std::array::from_fn(|i| position[i].wrapping_add(i32::from(rolling.velocity[i])));
        let mut cursor = game.cursor;
        let mut hint = rolling.hint;
        let first = terrain::particle_floor(&game.mission.terrain, &mut cursor, next, &mut hint, 0)
            .unwrap();
        let hint1 = hint.map_or(0, |h| h + 1) as i32;
        let probe = terrain::vector(&game.mission.terrain, &mut cursor, position, 500, 0).unwrap();
        let second =
            terrain::particle_floor(&game.mission.terrain, &mut cursor, position, &mut hint, 0)
                .unwrap();
        let hint2 = hint.map_or(0, |h| h + 1) as i32;
        let reflection = game.body.reflect_move[usize::from(probe.vector)];
        let queries = if rolling.status == 4 {
            0
        } else {
            1 + i32::from(first < next[1] && hint1 == 0)
        };
        let probes = i32::from(rolling.status != 4 && first < next[1]);
        let values = [
            position[0],
            position[1],
            position[2],
            i32::from(rolling.velocity[0]),
            i32::from(rolling.velocity[1]),
            i32::from(rolling.velocity[2]),
            i32::from(rolling.status),
            rolling.hint.map_or(0, |h| h + 1) as i32,
            seed as i32,
            first,
            hint1,
            second,
            hint2,
            probe.level,
            probe.height,
            i32::from(probe.attribute),
            i32::from(probe.vector),
            i32::from(reflection[0]),
            i32::from(reflection[1]),
        ];
        for v in values {
            write!(input, "{v} ").unwrap();
        }
        writeln!(input).unwrap();
        game.random = Random::new(seed);
        game.effects = crate::effects::Effects::default();
        with_items(&mut game, |_, ctx, _| {
            rolling.step(&mut position, ctx).unwrap()
        });
        let splashes = game
            .effects
            .slots
            .iter()
            .filter(|s| matches!(s, Some(crate::effects::Effect::Splash(_))))
            .count() as i32;
        expected.push(vec![
            position[0],
            position[1],
            position[2],
            i32::from(rolling.velocity[0]),
            i32::from(rolling.velocity[1]),
            i32::from(rolling.velocity[2]),
            i32::from(rolling.status),
            rolling.hint.map_or(0, |h| h + 1) as i32,
            game.random.seed as i32,
            game.random.calls as i32,
            splashes,
            queries,
            probes,
        ]);
    }
    let actual = oracle::run(&harness, &input);
    for (i, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "case {i}");
    }
    assert_eq!(actual.len(), expected.len());
}
