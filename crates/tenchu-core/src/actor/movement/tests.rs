use super::*;
use crate::{
    actor::tests::{arena, constants, with_actor},
    oracle,
};
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares the original ActSTATE fall branch"]
fn fall_transitions_damage_and_rng_match_original() {
    let source = oracle::source("MOTION.C");
    let function = oracle::function(&source, "void ActSTATE(void)\n{");
    let enumeration =
        &function[function.find("enum").unwrap()..function.find("short cleanup_guard").unwrap()];
    let branch = function
        .split_once("case MOT_STATE_FALL:")
        .unwrap()
        .1
        .split_once("case MOT_STATE_LAND:")
        .unwrap()
        .0;
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
{}
typedef struct {{int16_t vx,vy,vz;}} Vector;
typedef struct {{int16_t mid,count;}} Motion;
typedef struct {{int16_t type,life;uint16_t attribute;struct {{int height;}} map;struct {{uint16_t trig;}} pad;}} Human;
static Human human,*Me_MOTION_C=&human;static Vector speed,*dtV=&speed;static Motion motion,*dtM=&motion;
static int16_t motID,motMODE;static uint32_t seed;static int calls,sound,bars;
static int game_rand(void) {{seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767;}}
#define rand game_rand
#define PADRleft 0x80
#define SET_MOTION(id,mode) do {{motID=id;motMODE=mode;}} while(0)
#define SELECT_RETURN_MOTION() SET_MOTION((human.attribute&ATTR_WEAPON_DRAWN)?MOT_ENGAGE_STANCE:MOT_NORMAL,1)
static void Sound(Human*h,int id) {{sound=id;}}
static void ReqLifeBar(Human*h) {{bars++;}}
static void fall(void) {{ {} switch(dtM->mid) {{case MOT_STATE_FALL: {} }} }}
int main(void) {{int a[9];while(scanf("%d",&a[0])==1) {{
for(int i=1;i<9;i++)if(scanf("%d",&a[i])!=1)return 2;
human.type=a[0];human.attribute=a[1];human.map.height=a[2];human.life=a[6];human.pad.trig=a[7];
motion.mid=MOT_STATE_FALL;motion.count=a[3];speed=(Vector){{-9,a[4],13}};
seed=(uint32_t)a[5];calls=bars=0;sound=-1;motID=MOT_STATE_FALL;motMODE=0;
fall();printf("%d %d %d %d %d %d %d %d %d %d\n",motID,motMODE,motion.count,human.life,speed.vx,speed.vz,(int32_t)seed,calls,sound,bars);
}}return 0;}}
"#,
        constants(),
        enumeration,
        branch
    );
    let mut game = oracle::game(0, 0);
    let id = arena(&mut game, 0x10);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..2048 {
        let kind = [0, 1, 0x10, 0x1f, 0x20, 0x80, 0x86, 0x10][case % 8];
        let attribute = [
            0,
            ATTR_NOFLOOR,
            ATTR_WEAPON_DRAWN,
            ATTR_NOFLOOR | ATTR_WEAPON_DRAWN,
        ][case / 8 % 4];
        let height = [-1, 0, 1, 1000][case / 32 % 4];
        let count = [-56, -55, -54, -46, -41, -40, -22, -21, -20, -1, 0, 1][case % 12];
        let vy = [-10, 0, 199, 200, 201, 32767][case / 3 % 6];
        let health = [1, 9, 10, 11, 50, i16::MIN, i16::MIN + 9, i16::MAX][case / 7 % 8];
        let seed = case as u32 * 31 + 7;
        let trigger = if case % 5 == 0 { SQUARE } else { 0 };
        writeln!(
            input,
            "{kind} {attribute} {height} {count} {vy} {seed} {health} {trigger} 0"
        )
        .unwrap();
        with_actor(&mut game, id, |actor, ctx, world| {
            actor.kind = kind;
            actor.attribute = attribute;
            actor.map.height = height;
            actor.motion.action = MOT_STATE_FALL;
            actor.motion.count = count;
            actor.velocity = [-9, vy, 13];
            actor.health = health;
            actor.pad.trigger = trigger;
            *ctx.random = Random::new(seed);
            world.intelligence.voices.clear();
            world.combat.life_bars.clear();
            let (action, movement) = actor
                .state(ctx, world, 0)
                .unwrap()
                .unwrap_or((MOT_STATE_FALL, false));
            expected.push(vec![
                i32::from(action),
                i32::from(movement),
                i32::from(actor.motion.count),
                i32::from(actor.health),
                i32::from(actor.velocity[0]),
                i32::from(actor.velocity[2]),
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                world
                    .intelligence
                    .voices
                    .last()
                    .map_or(-1, |(_, id)| i32::from(*id)),
                world.combat.life_bars.len() as i32,
            ]);
        });
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual,
            expected,
            "case {case}: {}",
            input.lines().nth(case).unwrap()
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; NPC gestures bypass idle RNG and flips retain shared camera effects"]
fn npc_gesture_priority_and_flip_camera_requests() {
    let mut game = oracle::game(0, 0);
    let npc = arena(&mut game, 0);
    let seed = game.random;
    with_actor(&mut game, npc, |actor, ctx, world| {
        actor.enter(ctx, MOT_NORMAL, true);
        actor.attribute = ATTR_WEAPON_DRAWN;
        actor.pad.trigger = CROSS | SQUARE;
        for (held, action) in [
            (L2, None),
            (L2 | DOWN | RIGHT | UP, Some(MOT_ACTION_GESTURE)),
            (L2 | RIGHT | LEFT | UP, Some(MOT_ACTION_LOOP)),
            (L2 | LEFT | UP, Some(MOT_ACTION_NOTICE)),
            (L2 | UP, Some(MOT_ACTION)),
        ] {
            assert_eq!(
                actor.normal(ctx, world, held, 1, [0; 3]).unwrap(),
                action.map(|action| (action, true))
            );
            assert_eq!(*ctx.random, seed);
        }
    });
    for id in [game.player_id, npc] {
        with_actor(&mut game, id, |actor, ctx, world| {
            assert!(actor.enter(ctx, MOT_SQUAT_BACKFLIP, true));
            actor.motion.count = ctx.mission.clips[actor.motion.clip.unwrap()].duration >> 1;
            ctx.camera.snap_pending = false;
            actor.crouch(ctx, world, CIRCLE, 0).unwrap();
            assert!(ctx.camera.snap_pending);
            assert!(actor.enter(ctx, MOT_STATE_LAND_FLIP, true));
            actor.motion.count = 0;
            actor.motion.loops = 1;
            ctx.camera.snap_pending = false;
            actor.state(ctx, world, 0).unwrap();
            assert!(ctx.camera.snap_pending);
        });
    }
}
