use super::*;
use crate::math::Position;
use crate::oracle;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DECOMP; compares original snow motion, unsigned wrapping and terrain expiry"]
fn snow_state_matches_original_c() {
    let source = oracle::source("EFFECT.C");
    let function = oracle::function(&source, "void DrawSnow(TEffectSlot *ef)");
    let harness=[r#"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
typedef int32_t s32;typedef int16_t s16;typedef uint16_t u16;typedef uint32_t u32;
typedef struct {s16 vx,vy,vz,pad;} SVECTOR;
typedef struct {s32 x,y,z,ground,sample_y,size;s16 velocity[3];uint8_t sprite;} SnowParticleType;
typedef struct {s32 scalex,scaley;s16 x,y;} GsSPRITE;
typedef struct {GsSPRITE sprite;} Sprite3D;
typedef struct TEffectSlot {void(*proc)(struct TEffectSlot*);struct {SnowParticleType snow;} param;} TEffectSlot;
static struct {int vrx,vry,vrz;} ViewInfo;static Sprite3D model,*SpriteSnow[]={&model};static int GlobalAreaMap,OTablePt;
enum {SNOW_RANGE=3000,SNOW_SPAN=6000,NEAR_DEPTH=36,PROJECTION_DISTANCE=300,AREA_LEVEL_FIRST_HIT=8};
#define CLAMP_SORT_DEPTH(pri,z) pri=(z)<0?0:((z)>=1250?1249:(z))
static int GetAreaMapLevel(int map,int x,int y,int z,int mode){return 0;}
static void GetScreenPosition(int x,int y,int z,SVECTOR*s){*s=(SVECTOR){0};}
static void GsSortSprite(GsSPRITE*s,int table,int priority){}
"#,function,r#"
int main(void){int a[10];while(scanf("%d",a)==1){for(int i=1;i<10;i++)if(scanf("%d",a+i)!=1)return 2;
TEffectSlot e={0};e.proc=DrawSnow;SnowParticleType*p=&e.param.snow;
*p=(SnowParticleType){a[0],a[1],a[2],a[6],-8000,4096,{a[3],a[4],a[5]},0};ViewInfo.vrx=a[7];ViewInfo.vry=a[8];ViewInfo.vrz=a[9];
for(int frame=0;frame<24;frame++){DrawSnow(&e);printf("%d %d %d %d %d\n",e.proc!=0,p->x,p->y,p->z,p->ground);if(!e.proc)break;}
}return 0;}
"#].concat();
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut effects = Effects::default();
    let view = View {
        position: Position::ZERO,
        matrix: Matrix::default(),
    };
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..2048 {
        let position = [
            [-3001, -6000, 3001],
            [3000, -1, -3000],
            [0, -100, 0],
            [9999, -3000, -9999],
        ][case % 4];
        let velocity = [
            [-10, 50, 10],
            [0, 99, 0],
            [32767, 0, -32768],
            [-50, -50, 50],
        ][case / 4 % 4];
        let reference = [[0; 3], [1000, -500, 1000], [-1000, -6000, -1000]][case / 16 % 3];
        let ground = [0, -2000, 3000, i32::MIN][case / 48 % 4];
        for v in position
            .into_iter()
            .chain(velocity.map(i32::from))
            .chain([ground])
            .chain(reference)
        {
            write!(input, "{v} ").unwrap();
        }
        writeln!(input).unwrap();
        let mut snow = Snow {
            position: position.into(),
            velocity,
            ground,
            sample_y: -8000,
            size: 4096,
        };
        let mut env = Environment {
            terrain: &map,
            cursor: &mut cursor,
            clock: 0,
            owner_position: Position::ZERO,
            reference: reference.into(),
        };
        for _ in 0..24 {
            let alive = effects.update_snow(&mut snow, view, &mut env).unwrap();
            let mut row = vec![i32::from(alive)];
            row.extend(snow.position);
            row.push(snow.ground);
            expected.push(row);
            if !alive {
                break;
            }
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(actual, expected, "row {i}");
    }
}

#[test]
fn full_snow_pool_keeps_terrain_queries_and_does_not_tick_the_dummy() {
    let map = flat_map();
    let mut cursor = Cursor::new(&map);
    let mut effects = Effects::default();
    for _ in 0..201 {
        effects
            .snow(
                Position::new(0, -200, 300),
                [0, 50, 0],
                4096,
                &map,
                &mut cursor,
            )
            .unwrap();
    }
    assert_eq!(effects.slots.iter().flatten().count(), 201);
    assert_eq!(cursor.last_y, -820);
    let view = View {
        position: Position::ZERO,
        matrix: Matrix::default(),
    };
    effects
        .step(
            &mut Random::new(1),
            view,
            &mut Scene::default(),
            &mut Environment {
                terrain: &map,
                cursor: &mut cursor,
                clock: 1,
                owner_position: Position::ZERO,
                reference: Position::ZERO,
            },
        )
        .unwrap();
    let Some(Effect::Snow(snow)) = &effects.slots[200] else {
        panic!("Missing overflow snow");
    };
    assert_eq!(snow.position, [0, -200, 300]);
    let Some(Effect::Snow(snow)) = &effects.slots[0] else {
        panic!("Missing live snow");
    };
    assert_eq!(snow.position, [0, -150, 300]);
    assert_eq!(effects.drawings.len(), 200);
}
