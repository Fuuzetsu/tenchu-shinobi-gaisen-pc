use super::*;
use crate::math::Position;
use crate::oracle;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original hook timing, bends and impact RNG"]
fn flight_and_straightening_match_original_c() {
    let source = oracle::source("EFFECT.C");
    let routines = ["static void DrawFlyWire(", "int SetFlyWire("]
        .map(|signature| oracle::function(&source, signature).replace("long ", "int32_t "))
        .join("\n");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
typedef int16_t s16; typedef uint16_t u16; typedef int32_t s32;
typedef struct {{ int32_t vx,vy,vz,pad; }} VECTOR;
typedef struct {{ VECTOR start,end,center,NCenter; int16_t count,time; uint8_t mode; }} FlyWireType;
typedef struct TEffectSlot {{ void (*proc)(struct TEffectSlot *); struct {{ FlyWireType flywire; }} param; }} TEffectSlot;
static TEffectSlot slot;
static struct {{ int Owner; }} CamState;
#define FIXED_ONE 4096
#define FLYWIRE_MODE_EXTEND 0
#define FLYWIRE_MODE_STRAIGHTEN 1
#define COLOR_YELLOW 0xffff00
#define SE_PROJECTILE_IMPACT 0x31
static int root,calls,sounds,wires;
static uint32_t seed;
int rand(void) {{ seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767; }}
static int SquareRoot0(int value) {{ return root; }}
static TEffectSlot *GetFreeEffectSlot(void) {{ return &slot; }}
static void Sound(int owner,int sound) {{ assert(sound==0x31);sounds++; }}
static void SetBleeds(VECTOR *pos,int spread,int speed,int count,int time,int color) {{
    assert(spread==0 && speed==50 && count==10 && time==30 && color==0xffff00);
    for(int i=0;i<count*4;i++) rand();
}}
static void SetWire(VECTOR *a,VECTOR *b,VECTOR *c,int len) {{ wires++; }}
{routines}
static void emit(int result) {{
    FlyWireType *p=&slot.param.flywire;
    printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d\n",result,slot.proc!=0,p->time,p->count,p->mode,
        p->center.vx,p->center.vy,p->center.vz,p->NCenter.vx,p->NCenter.vy,p->NCenter.vz,(int)seed,calls,sounds,wires);
}}
int main(void) {{
    int a[8];
    while(scanf("%d",a)==1) {{
        for(int i=1;i<8;i++) assert(scanf("%d",a+i)==1);
        VECTOR start={{a[0],a[1],a[2],0}},end={{a[3],a[4],a[5],0}};
        root=a[6];seed=(uint32_t)a[7];calls=sounds=wires=0;memset(&slot,0,sizeof slot);
        int result=SetFlyWire(&start,&end);emit(result);
        while(slot.proc) {{ slot.proc(&slot);emit(result); }}
    }}
}}
"#
    );
    let game = oracle::game(0, 0);
    let mut cases = Random::new(0x83191);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..2048 {
        let start: Position = Position::from_fn(|_| (cases.next15() - 16384) * 7);
        let end: Position =
            Position::from_fn(|i| start[i] + (cases.next15() - 16384) / [1, 4, 20, 400][case % 4]);
        let delta: Position = Position::from_fn(|i| start[i] - end[i]);
        let big = delta.iter().any(|v| v.abs() > 4096);
        let root = game
            .math
            .length(if big { delta.map(|v| v / 256) } else { delta });
        let seed = cases.next15() as u32;
        for v in start.into_iter().chain(end).chain([root, seed as i32]) {
            write!(input, "{v} ").unwrap();
        }
        writeln!(input).unwrap();
        let mut effects = Effects::default();
        let mut rng = Random::new(seed);
        let result = effects.fly_wire(start, end, &game.math, &mut rng).unwrap();
        let distance = if big { root << 8 } else { root };
        // The original writes setup parameters even for a zero-time, unallocated effect.
        if result == 0 {
            let mut setup_rng = Random::new(seed);
            let straight: Position = Position::from_fn(|i| (start[i] + end[i]) / 2);
            let bend = distance / 16;
            let center: Position = Position::from_fn(|i| {
                let range = if i == 1 { bend } else { bend * 2 };
                straight[i]
                    + if range > 0 {
                        setup_rng.next15() % range
                    } else {
                        0
                    }
                    - bend
            });
            expected.push(
                [0, 0, distance / 1000, 0, 0]
                    .into_iter()
                    .chain(center)
                    .chain(straight)
                    .chain([rng.seed as i32, rng.calls as i32, 0, 0])
                    .collect::<Vec<_>>(),
            );
            continue;
        }
        let Some(Effect::FlyWire(mut wire)) = effects.slots[1].take() else {
            panic!("Missing wire")
        };
        let mut alive = true;
        loop {
            expected.push(
                [
                    i32::from(result),
                    i32::from(alive),
                    i32::from(wire.time),
                    i32::from(wire.count),
                    i32::from(wire.mode),
                ]
                .into_iter()
                .chain(wire.center)
                .chain(wire.straight)
                .chain([
                    rng.seed as i32,
                    rng.calls as i32,
                    effects.sounds.len() as i32,
                    effects.wires.len() as i32,
                ])
                .collect::<Vec<_>>(),
            );
            if !alive {
                break;
            }
            alive = effects.update_wire(
                &mut wire,
                &mut rng,
                View {
                    position: Position::ZERO,
                    matrix: Matrix::default(),
                },
                Position::ZERO,
            );
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, &expected, "wire update {i}");
    }
}

#[test]
fn cable_excludes_endpoint_and_empty_curve_has_no_hook() {
    let view = View {
        position: Position::ZERO,
        matrix: Matrix::default(),
    };
    let rotation = Matrix::default().rotation;
    let wire = sample(
        Position::new(0, 0, 3000),
        Position::new(3000, 0, 3000),
        Position::new(1500, 0, 3000),
        10,
        4096,
        rotation,
        view,
    );
    assert_eq!(wire.lines.len(), 10);
    assert_eq!(wire.hook.unwrap().0.translation, [2699, 0, 3000]);
    let empty = sample(
        Position::new(0, 0, 3000),
        Position::new(1, 0, 3000),
        Position::new(0, 0, 3000),
        0,
        4096,
        rotation,
        view,
    );
    assert!(empty.lines.is_empty());
    assert!(empty.hook.is_none());
}
