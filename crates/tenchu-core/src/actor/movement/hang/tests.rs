use super::*;
#[test]
fn ledge_catch_keeps_non_cardinal_multiples_of_256() {
    for (input, expected) in [
        (255, 0),
        (256, 256),
        (511, 0),
        (512, 512),
        (513, 1024),
        (4095, 4096),
        (-1, 4096),
        (-256, -256),
    ] {
        assert_eq!(catch_yaw(input), expected);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares HangCheck and ActHANG on a ledge, including repeated release probes and disguise"]
fn ledge_release_and_disguise_match_original_c() {
    use crate::{
        actor::tests::{constants, with_actor},
        oracle,
    };
    use std::fmt::Write;
    let source = oracle::source("MOTION.C");
    let check = oracle::function(&source, "hang_check_result HangCheck(void)\n{")
        .replace("long", "int32_t");
    let control = oracle::function(&source, "void ActHANG(void)\n{").replace("long", "int32_t");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
{constants}
enum {{HANG_CHECK_NONE=0,HANG_CHECK_ACTIVE=1,HANG_CHECK_STARTED=-1,
       ACTIVE_ITEM_DISGUISE=11,AREA_LEVEL_DEFAULT=0,AREA_LEVEL_STEP_DOWN=1,
       AREA_LEVEL_RETURN_DELTA=2,ANGLE_SIXTEENTH_FRACTION_MASK=255}};
#define PADLdown 0x4000
#define PADLup 0x1000
#define PADLright 0x2000
#define PADLleft 0x8000
typedef uint32_t u32; typedef int16_t s16; typedef int hang_check_result;
typedef struct {{int16_t vx,vy,vz;}} SVECTOR;
typedef struct {{int32_t vx,vy,vz;}} VECTOR;
typedef struct {{int16_t mid,count,loop;}} Motion;
typedef struct {{int16_t type,height,width,status,active_item; uint16_t attribute; struct {{int height;}} map;}} Human;
static Human human,*Me_MOTION_C=&human,*StagePlayer=&human;
static VECTOR position,*dtL=&position; static SVECTOR rotation,*dtR=&rotation,speed,*dtV=&speed;
static Motion motion,*dtM=&motion; static int16_t motID,motMODE,dtPAD;
static uint32_t *GlobalAreaMap; static int probes;
#define LEVEL_NONE INT32_MIN
#define SET_MOTION(id,mode) do {{motID=id;motMODE=mode;}} while(0)
static void GetMoveSpeed(SVECTOR*v,int16_t yaw,int16_t forward,int16_t side) {{
 assert(yaw==1024 && side==0); *v=(SVECTOR){{-forward,0,0}};
}}
static int32_t GetAreaMapLevel(uint32_t*m,int32_t x,int32_t y,int32_t z,int mode) {{
 assert(!m && ++probes<1000); int32_t floor=x/10<=0 ? -2000 : 0;
 int32_t query=y/10-((mode&1)?150:0),delta=floor-y;
 if(x/10 < -3000 || x/10>3000 || z/10 < -3000 || z/10>3000 || query>floor/10 || (delta < -1000 && !(mode&4)))return INT32_MIN;
 return mode&2 ? delta : floor;
}}
static void SetNowMotionUnlessCva(void) {{assert(0);}}
static void Sound(Human*h,int id) {{assert(0);}}
static void PadShockAR(int a,int b,int c,int d) {{assert(0);}}
static void SetCameraMode(int mode) {{assert(0);}}
static void MoveHumanoid(Human*h,int forward,int side) {{assert(0);}}
{check}
{control}
int main(void) {{int a[7];while(scanf("%d",a)==1) {{
 for(int i=1;i<7;i++)if(scanf("%d",a+i)!=1)return 2;
 human=(Human){{.type=a[0],.height=a[1],.width=a[2],.status=STAT_HANG,.active_item=a[3],.map={{a[4]}}}};
 position=(VECTOR){{a[2]/2+100,a[5],0}};rotation=(SVECTOR){{0,1024,0}};speed=(SVECTOR){{0,123,0}};
 motion=(Motion){{MOT_HANG,0,0}};motID=MOT_HANG;motMODE=0;dtPAD=PADLdown;probes=0;
 int result=-1;if(a[6])ActHANG();else result=HangCheck()!=HANG_CHECK_NONE;
 printf("%d %d %d %d %d %d %d %d\n",result,position.vx,position.vy,position.vz,rotation.vy,speed.vy,motID,motMODE);
 }}return 0;
}}
"#,
        constants = constants()
    );
    let mut game = oracle::game(0, 0);
    let mut bytes = vec![0u8; 80];
    for (i, (floor, bounds)) in [
        (-200i16, [-3000i16, -3000, 0, 3000]),
        (0, [1, -3000, 3000, 3000]),
    ]
    .into_iter()
    .enumerate()
    {
        let row = i * 16;
        let node = 48 + i * 16;
        bytes[row..row + 2].copy_from_slice(&(floor - 2).to_le_bytes());
        bytes[row + 2..row + 4].copy_from_slice(&1i16.to_le_bytes());
        bytes[row + 4..row + 8].copy_from_slice(&(node as u32).to_le_bytes());
        bytes[node..node + 2].copy_from_slice(&floor.to_le_bytes());
        for (j, value) in bounds.into_iter().enumerate() {
            bytes[row + 8 + j * 2..row + 10 + j * 2].copy_from_slice(&value.to_le_bytes());
            bytes[node + 4 + j * 2..node + 6 + j * 2].copy_from_slice(&value.to_le_bytes());
        }
        bytes[node + 12..node + 14].copy_from_slice(&0x2000u16.to_le_bytes());
        bytes[node + 14..node + 16].copy_from_slice(&u16::MAX.to_le_bytes());
    }
    game.mission.terrain = tenchu_assets::terrain::AreaMap::parse(bytes).unwrap();
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    for kind in [0, 1, 0x33, 0xa0] {
        for (height, width) in [(800, 400), (1500, 600), (2000, 1000)] {
            for offset in [100, 200, 300] {
                for active_item in [0, 11] {
                    for release in [false, true] {
                        let y = -2000 + height + offset;
                        writeln!(
                            input,
                            "{kind} {height} {width} {active_item} 100 {y} {}",
                            i32::from(release)
                        )
                        .unwrap();
                        expected.push(with_actor(&mut game, player, |actor, ctx, world| {
                            let definition = &mut ctx.mission.characters[actor.definition];
                            definition.height = height as i16;
                            definition.width = width as i16;
                            actor.kind = kind;
                            actor.status = 10;
                            actor.attribute = 0;
                            actor.active_item = active_item;
                            actor.map.height = 100;
                            actor.position = [width / 2 + 100, y, 0];
                            actor.rotation = [0, 1024, 0];
                            actor.velocity = [0, 123, 0];
                            actor.motion.action = MOT_HANG;
                            actor.motion.count = 0;
                            actor.motion.loops = 0;
                            *ctx.cursor = terrain::Cursor::new(&ctx.mission.terrain);
                            let (result, action, movement) = if release {
                                let (action, movement) =
                                    actor.hang(ctx, DOWN, world.player).unwrap().unwrap();
                                (-1, action, movement)
                            } else {
                                (i32::from(actor.hang_check(ctx).unwrap()), MOT_HANG, false)
                            };
                            vec![
                                result,
                                actor.position[0],
                                actor.position[1],
                                actor.position[2],
                                i32::from(actor.rotation[1]),
                                i32::from(actor.velocity[1]),
                                i32::from(action),
                                i32::from(movement),
                            ]
                        }));
                    }
                }
            }
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual,
            &expected,
            "case {case}: {}",
            input.lines().nth(case).unwrap()
        );
    }
}
