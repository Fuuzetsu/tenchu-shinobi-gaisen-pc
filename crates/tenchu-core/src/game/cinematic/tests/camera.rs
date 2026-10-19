use super::*;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares the matching cinematic camera C with scalar view updates"]
fn cuts_and_pan_modes_match_original_c() {
    let source = oracle::source("CHRANIM.C");
    let types = oracle::source("game_types.h");
    let types = &types[types.find("typedef s16 cva_command;").unwrap()
        ..types.find("#define N_CVA_HUMANS").unwrap()];
    let game = &mut oracle::game(0, 0);
    let boss = game.roster.find(0x81).unwrap();
    let trig = game
        .math
        .tables
        .trig
        .iter()
        .map(|v| format!("{{{},{}}}", v[0], v[1]))
        .collect::<Vec<_>>()
        .join(",");
    let routines = ["void AVCameraSetup(void)", "void AVCameraControl(void)"]
        .map(|s| oracle::function(&source, s).replace("long ", "int32_t "))
        .join("\n");
    let movement = oracle::source("HUMAN.C");
    let movement = oracle::function(&movement, "void GetMoveSpeed(");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
typedef int16_t s16;typedef uint16_t u16;typedef int32_t s32;typedef uint32_t u32;
typedef s16 character_kind;typedef s16 motion_id;typedef s16 motion_loop_count;
typedef struct {{s32 vx,vy,vz;}} VECTOR;
typedef struct {{s16 vx,vy,vz,pad;}} SVECTOR;
typedef struct Humanoid {{VECTOR *locate;SVECTOR *rotate;s16 height;}} Humanoid;
{types}
static const s16 trig[4096][2]={{{trig}}};
#define ANGLE_QUADRANT 1024
#define FIXED_SHIFT 12
int rsin(int a){{return (a<0?-1:1)*trig[abs(a)&4095][0];}}
int rcos(int a){{return trig[abs(a)&4095][1];}}
{movement}
static struct {{s32 vpx,vpy,vpz,vrx,vry,vrz;}} ViewInfo;
static Humanoid actors[2],*CameraTarget;static VECTOR positions[2];static SVECTOR rotations[2];
static CVAType command,*CVAnow=&command;static s16 CameraPanMode,CameraSpeed;
static s32 length_value,direction_value,xx_value,zz_value;static int refreshed;
static Humanoid *GetHumanoid(s16 id){{return id==0?actors:id==0x81?actors+1:0;}}
static s32 SquareRoot0(s32 v){{assert(v==xx_value*xx_value+zz_value*zz_value);return length_value;}}
static s16 GetDirection(s32 x,s32 z,s16 angle){{assert(x==xx_value && z==zz_value && angle==0);return direction_value;}}
static void GsSetRefView2(void *p){{refreshed++;}}
static void Camera(void){{assert(0);}}
{routines}
int main(void){{
 int a[26];while(scanf("%d",a)==1){{for(int i=1;i<26;i++)assert(scanf("%d",a+i)==1);
 command.mode=4;command.payload.raw=(CVARawPayload){{a[0],a[1],a[2],a[3],a[4]}};
 ViewInfo.vpx=a[5];ViewInfo.vpy=a[6];ViewInfo.vpz=a[7];ViewInfo.vrx=a[8];ViewInfo.vry=a[9];ViewInfo.vrz=a[10];
 for(int i=0;i<2;i++){{positions[i]=(VECTOR){{a[11+i*5],a[12+i*5],a[13+i*5]}};rotations[i]=(SVECTOR){{0,a[14+i*5],0,0}};actors[i]=(Humanoid){{positions+i,rotations+i,a[15+i*5]}};}}
 CameraTarget=actors;refreshed=0;AVCameraSetup();
 printf("%d %d %d %d %d %d %d %d",ViewInfo.vpx,ViewInfo.vpy,ViewInfo.vpz,ViewInfo.vrx,ViewInfo.vry,ViewInfo.vrz,(int)(CameraTarget-actors),refreshed);
 CameraPanMode=a[21];CameraSpeed=a[22];length_value=a[23];direction_value=a[24];
 xx_value=ViewInfo.vpx-ViewInfo.vrx;zz_value=ViewInfo.vpz-ViewInfo.vrz;
 positions[CameraTarget-actors].vx+=a[25];AVCameraControl();
 printf(" %d %d %d %d %d %d %d\n",ViewInfo.vpx,ViewInfo.vpy,ViewInfo.vpz,ViewInfo.vrx,ViewInfo.vry,ViewInfo.vrz,refreshed);
 }}
}}
"#
    );
    let mut random = crate::random::Random::new(29764);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..512 {
        let command = tenchu_assets::script::Command {
            operation: 4,
            id: (case % 7) as i16,
            position: std::array::from_fn(|_| (random.next15() % 80 - 40) as i16),
            parameter: if case % 7 == 5 {
                if case % 2 == 0 { 0x81 } else { 99 }
            } else if case % 5 == 0 {
                0
            } else {
                (random.next15() % 20000 - 10000) as i16
            },
        };
        game.camera.position = std::array::from_fn(|_| random.next15() % 10000 - 5000);
        game.camera.reference = std::array::from_fn(|_| random.next15() % 10000 - 5000);
        let mut row = vec![i32::from(command.id)];
        row.extend(command.position.map(i32::from));
        row.push(i32::from(command.parameter));
        row.extend(game.camera.position);
        row.extend(game.camera.reference);
        for id in [game.player_id, boss] {
            let actor = game.roster.get_mut(id);
            actor.position = std::array::from_fn(|_| random.next15() % 20000 - 10000);
            actor.rotation[1] = random.next15() as i16;
            row.extend(actor.position);
            row.push(i32::from(actor.rotation[1]));
            row.push(i32::from(game.mission.characters[actor.definition].height));
        }
        game.cinematic.target = game.player_id;
        game.cinematic_camera_command(command).unwrap();
        let mut state = game
            .camera
            .position
            .into_iter()
            .chain(game.camera.reference)
            .collect::<Vec<_>>();
        let refreshed = i32::from(!(command.id == 5 && command.parameter == 99));
        state.extend([i32::from(game.cinematic.target == boss), refreshed]);
        let x = game.camera.position[0] - game.camera.reference[0];
        let z = game.camera.position[2] - game.camera.reference[2];
        game.cinematic.pan = [0, 2, 3, 4, 5, 6, 7, 8, 20][case % 9];
        game.cinematic.speed = (random.next15() % 300 - 150) as i16;
        row.extend([
            i32::from(game.cinematic.pan),
            i32::from(game.cinematic.speed),
            game.math.length([x, 0, z]),
            i32::from(game.math.direction(x, z, 0).unwrap()),
            193,
        ]);
        for value in row {
            write!(input, "{value} ").unwrap();
        }
        input.push('\n');
        game.roster.get_mut(game.cinematic.target).position[0] += 193;
        game.cinematic_camera(0).unwrap();
        state.extend(game.camera.position);
        state.extend(game.camera.reference);
        state.push(refreshed + i32::from(game.cinematic.pan != 0));
        expected.push(state);
    }
    assert_eq!(expected, oracle::run(&harness, &input));
}
