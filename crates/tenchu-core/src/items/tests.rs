use super::*;
use crate::{game::Game, input::UP, oracle, random::Random};
use std::fmt::Write;

fn with_items<T>(game: &mut Game, f: impl FnOnce(&mut Items, &mut Context, &mut Actors) -> T) -> T {
    f(
        &mut game.items,
        &mut Context {
            mission: &mut game.mission,
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
        &mut Actors::new(&mut game.roster),
    )
}

#[test]
#[ignore = "requires TENCHU_DISC; exercises pickup through ordinary ticks and allocation/transfer quirks"]
fn pickups_transfer_and_full_pool_replacement_preserve_source_order() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        let position = game.player().position;
        with_items(&mut game, |items, ctx, roster| {
            for id in 0..CAPACITY {
                items.dispose(id, roster, ctx).unwrap();
            }
            items.stay(3, position, roster, ctx).unwrap();
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
            items
                .drop(3, Some(player), position, [0; 3], roster, ctx)
                .unwrap();
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
                items.dispose(id, roster, ctx).unwrap();
            }
            items.cursor = 0;
            for _ in 0..CAPACITY {
                assert!(items.drop(1, None, position, [0; 3], roster, ctx).unwrap());
            }
            assert_eq!(items.cursor, 0);
            items.contact(29, 180, crate::collision::SOFT, ctx).unwrap();
            let model = items.slots[29].model;
            assert!(
                !items
                    .drop(3, None, [999000; 3], [0; 3], roster, ctx)
                    .unwrap()
            );
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

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares complete integer projectile arcs with original C"]
fn projectile_arcs_match_original_c_through_rolling_handoff() {
    let source = oracle::source("ITEM.C");
    let routines = [
        "static inline long SubFlyJitter(",
        "static void SetupFly(",
        "static void MoveFly(",
    ]
    .map(|signature| oracle::function(&source, signature).replace("long ", "s32 "))
    .join("\n");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
typedef int32_t s32;
typedef struct {{ int32_t vx,vy,vz,pad; }} VECTOR;
typedef struct {{ uintptr_t hint; int16_t vx,vy,vz; uint8_t status; }} param_korogari;
struct tag_fly {{ int32_t sx,sy,sz,vx,vy,vz,rx,ry,rz; uint8_t count,count2; }};
typedef struct {{ union {{ struct tag_fly fly; param_korogari koro; }} p; uint8_t mode; }} param_fly;
typedef struct {{ struct {{ struct {{ int t[3]; }} coord; }} locate; }} ModelType;
typedef struct {{ ModelType *locate; }} TItem;
#define FLY_MODE_ARC 0
#define FLY_MODE_ROLL 1
#define KORO_NORMAL 0
#define FIXED_ONE 4096
#define FIXED_SHIFT 12
#define FIXED_TRUNC_BIAS 4095
#define setVector(v,x,y,z) ((v)->vx=(x),(v)->vy=(y),(v)->vz=(z))
#define copyVector(to,from) setVector((to),(from)->vx,(from)->vy,(from)->vz)
static int distance,calls;
static uint32_t seed;
int rand(void) {{ seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767; }}
static int GetVectorDistance(VECTOR *a,VECTOR *b) {{ return distance; }}
static void MoveKorogari(TItem *item,param_korogari *param) {{ abort(); }}
{routines}
int main(void) {{
    int a[11];
    while(scanf("%d",a)==1) {{
        for(int i=1;i<11;i++) assert(scanf("%d",a+i)==1);
        VECTOR start={{a[0],a[1],a[2],0}},end={{a[3],a[4],a[5],0}};
        param_fly fly={{0}};seed=(uint32_t)a[10];calls=0;distance=a[9];
        SetupFly(&fly,&start,&end,a[6],a[7],a[8]);
        printf("%d %d %d %d %d %d %d",fly.p.fly.rx,fly.p.fly.ry,fly.p.fly.rz,fly.p.fly.count,fly.p.fly.count2,(int)seed,calls);
        ModelType model={{.locate.coord.t={{a[0],a[1],a[2]}}}};TItem item={{&model}};
        uint32_t digest=2166136261;
        while(fly.mode==FLY_MODE_ARC) {{
            MoveFly(&item,&fly);
            for(int i=0;i<3;i++) digest=(digest^(uint32_t)model.locate.coord.t[i])*16777619;
        }}
        printf(" %d %d %d %d %d %d %d\n",(int)digest,model.locate.coord.t[0],model.locate.coord.t[1],model.locate.coord.t[2],fly.p.koro.vx,fly.p.koro.vy,fly.p.koro.vz);
    }}
}}
"#
    );
    let mut game = oracle::game(0, 0);
    let mut cases = Random::new(0x615517);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..2048 {
        let start: [i32; 3] = std::array::from_fn(|_| (cases.next15() - 16384) * 100);
        let end = std::array::from_fn(|i| start[i].wrapping_add((cases.next15() - 16384) * 8));
        let width = [0, 1024, -1024, 32767][case % 4];
        let height = [0, 1024, 2048, -32768][(case / 4) % 4];
        let speed = [0, 1, 300, 1000, -1][case % 5];
        let seed = cases.next15() as u32;
        let distance = game
            .math
            .vector_distance(std::array::from_fn(|i| end[i].wrapping_sub(start[i])));
        for v in start
            .into_iter()
            .chain(end)
            .chain([width, height, speed, distance, seed as i32])
        {
            write!(input, "{v} ").unwrap();
        }
        writeln!(input).unwrap();
        game.random = Random::new(seed);
        with_items(&mut game, |_, ctx, _| {
            let mut flight = flight::Flight::new(start, end, width, height, speed, ctx);
            let mut row = flight.control.to_vec();
            row.extend([
                i32::from(flight.count),
                i32::from(flight.total),
                ctx.random.seed as i32,
                ctx.random.calls as i32,
            ]);
            let mut rolling = Rolling::default();
            let mut position = start;
            let mut digest = 2166136261u32;
            while !flight.rolling {
                flight.step(&mut position, &mut rolling, ctx).unwrap();
                for p in position {
                    digest = (digest ^ p as u32).wrapping_mul(16777619);
                }
            }
            row.push(digest as i32);
            row.extend(position);
            row.extend(rolling.velocity.map(i32::from));
            expected.push(row);
        });
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (c, rust)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(rust, c, "flight case {i}");
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; medicine consumption/interruption and caltrop damage through normal ticks"]
fn consumables_finish_or_drop_and_caltrops_arm_on_the_floor() {
    use crate::{ids::*, input::TRIANGLE};
    for character in 0..2 {
        for interrupted in [false, true] {
            let mut game = oracle::game(0, character);
            let player = game.player_id;
            game.roster.get_mut(player).health = 25;
            game.roster.get_mut(player).inventory[3] = 2;
            game.hud.cursor = 3;
            game.step(0).unwrap();
            game.step(TRIANGLE).unwrap();
            assert_eq!(game.player().motion.action, MOT_ITEM_DRINK);
            assert_eq!(game.player().inventory[3], 1);
            for _ in 0..15 {
                game.step(0).unwrap();
            }
            assert_eq!(game.items.icon_scale[3], 8192);
            if interrupted {
                with_items(&mut game, |_, ctx, actors| {
                    actors.get_mut(player).enter(ctx, MOT_DAMAGE, true);
                });
                game.step(0).unwrap();
                assert!(game.items.slots.iter().any(|i| i.kind == 3
                    && i.owner == Some(player)
                    && i.processor == Some(Processor::Pickup)));
            }
            let mut healed = false;
            for _ in 0..100 {
                game.step(0).unwrap();
                healed |= game.effects.sounds.iter().any(|(_, id)| *id == 0x24);
            }
            assert_eq!(healed, !interrupted);
            assert_eq!(
                game.player().health,
                if interrupted {
                    25
                } else {
                    game.player().max_health
                }
            );
            assert!(
                !game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(Processor::Medicine))
            );
        }
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        oracle::flat_ground(&mut game);
        with_items(&mut game, |_, ctx, actors| {
            for id in actors.roster.order.clone() {
                if id != player {
                    actors.roster.remove(id, ctx);
                }
            }
            let actor = actors.get_mut(player);
            actor.position = [0; 3];
            actor.rotation = [0; 3];
            actor.map =
                terrain::vector(&ctx.mission.terrain, ctx.cursor, actor.position, 500, 0).unwrap();
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
            ctx.scene.finish_tick();
        });
        game.camera.snap_pending = true;
        game.roster.get_mut(player).inventory[2] = 2;
        game.hud.cursor = 2;
        game.step(0).unwrap();
        game.step(TRIANGLE).unwrap();
        for _ in 0..30 {
            game.step(0).unwrap();
            if game
                .items
                .slots
                .iter()
                .filter(|i| i.processor == Some(Processor::Caltrops))
                .count()
                == 5
            {
                break;
            }
        }
        assert_eq!(
            game.items
                .slots
                .iter()
                .filter(|i| i.processor == Some(Processor::Caltrops))
                .count(),
            5
        );
        with_items(&mut game, |_, ctx, actors| {
            let actor = actors.get_mut(player);
            actor.position[0] = 5000;
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
            ctx.scene.finish_tick();
        });
        for _ in 0..100 {
            game.step(0).unwrap();
        }
        let traps: Vec<_> = game
            .items
            .slots
            .iter()
            .filter(|i| i.processor == Some(Processor::Caltrops))
            .collect();
        assert_eq!(traps.len(), 5);
        assert!(traps.iter().all(|i| i.mode == 1));
        let position = game.scene.models[traps[0].model].local.translation;
        with_items(&mut game, |_, ctx, actors| {
            let actor = actors.get_mut(player);
            actor.position = position;
            actor.velocity = [0; 3];
            actor.enter(ctx, MOT_NORMAL, true);
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
            ctx.scene.finish_tick();
        });
        let health = game.player().health;
        for _ in 0..3 {
            game.step(0).unwrap();
        }
        assert!(game.player().health <= health - 3);
        assert_eq!(game.player().motion.action, MOT_DAMAGE_MAKIBISHI);
        assert_eq!(game.player().inventory[2], 1);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; eviction must clear the currently executing actor's aiming flag"]
fn full_pool_cancels_an_aimed_item_owned_by_the_active_actor() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        let mut actor = actors.roster.take(player);
        let start = actor.position;
        items
            .request(1, &mut actor, start, start, player, actors.roster, ctx)
            .unwrap();
        let aim = items.cursor;
        actors.roster.restore(actor);
        for _ in 0..CAPACITY - 1 {
            items.drop(3, None, start, [0; 3], actors, ctx).unwrap();
        }
        items.cursor = (aim + 1) % CAPACITY;
        let mut actor = actors.roster.take(player);
        assert_eq!(actor.inventory[25], 1);
        items
            .request(20, &mut actor, start, start, player, actors.roster, ctx)
            .unwrap();
        assert_eq!(actor.inventory[25], 0);
        assert_eq!(items.cursor, aim);
        assert_eq!(items.slots[aim].processor, Some(Processor::Gun));
        actors.roster.restore(actor);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; dud recovery, invalid mine refunds and water extinguishing"]
fn bombs_recover_refund_and_extinguish() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    let mut intelligence = crate::ai::Intelligence::default();
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        let mut actor = actors.roster.take(player);
        actor.inventory[4] = 2;
        actor.inventory[6] = 2;
        items
            .request(
                4,
                &mut actor,
                [0, -1000, 0],
                [0; 3],
                player,
                actors.roster,
                ctx,
            )
            .unwrap();
        actors.roster.restore(actor);
        let id = items.cursor;
        items.slots[id].rolling.status = 4;
        items.slots[id].count = 1;
        let seed = (0..100)
            .find(|&seed| {
                let mut rng = Random::new(seed);
                for _ in 0..4 {
                    rng.next15();
                }
                rng.next15() % 10 < 2
            })
            .unwrap();
        *ctx.random = Random::new(seed);
        items.fire(id, actors, &mut intelligence, ctx).unwrap();
        assert!(items.slots[id].processor.is_none());
        assert!(
            items.slots.iter().any(|i| i.kind == 4
                && i.owner.is_none()
                && i.processor == Some(Processor::Pickup))
        );
        assert_eq!(ctx.random.calls, 8);
        assert!(ctx.effects.slots.iter().any(|s| matches!(
            s,
            Some(effects::Effect::Smoke(effects::Smoke { time: 10, .. }))
        )));
        assert_eq!(actors.get(player).inventory[4], 1);
        let mut actor = actors.roster.take(player);
        items
            .request(
                6,
                &mut actor,
                [999000, -1000, 999000],
                [0; 3],
                player,
                actors.roster,
                ctx,
            )
            .unwrap();
        actors.roster.restore(actor);
        let id = items.cursor;
        items.mine(id, actors, &mut intelligence, ctx).unwrap();
        assert_eq!(actors.get(player).inventory[6], 2);
        assert!(items.slots[id].processor.is_none());
    });
    oracle::flat_terrain(&mut game, 0x2004);
    with_items(&mut game, |items, ctx, actors| {
        for kind in [4, 5, 6] {
            let mut actor = actors.roster.take(player);
            actor.inventory[kind] = 2;
            items
                .request(
                    kind,
                    &mut actor,
                    [0, -50, 0],
                    [0; 3],
                    player,
                    actors.roster,
                    ctx,
                )
                .unwrap();
            actors.roster.restore(actor);
            let id = items.cursor;
            items.slots[id].rolling.velocity = [0, 100, 0];
            match kind {
                4 => items.fire(id, actors, &mut intelligence, ctx),
                5 => items.smoke(id, actors, &mut intelligence, 0, ctx),
                _ => items.mine(id, actors, &mut intelligence, ctx),
            }
            .unwrap();
            assert!(items.slots[id].processor.is_none(), "kind {kind}");
            assert_eq!(
                actors.get(player).inventory[kind],
                if kind == 6 { 2 } else { 1 }
            );
        }
    });
}
