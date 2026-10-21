use super::*;
use crate::math::Position;
use crate::{oracle, random::Random};
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares original SoundEx/PlaySE with supplied SDK math results"]
fn spatial_sound_matches_original_calls() {
    let game = oracle::game(0, 0);
    let source = oracle::source("SEMNG.C");
    let audio = oracle::source("AUDIO.C");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stddef.h>
typedef int32_t s32; typedef uint32_t u32; typedef int16_t s16;
typedef struct {{s32 vx,vy,vz;}} VECTOR;
typedef struct {{s16 vx,vy,vz;}} SVECTOR;
typedef struct SoundEffect {{int VABid;}} SoundEffect;
typedef struct {{VECTOR *locate;SVECTOR *rotate;}} Human;
static SoundEffect bank,*StageSE=&bank;
static VECTOR pp; static SVECTOR rotation; static Human player={{&pp,&rotation}},*StagePlayer=&player;
static struct {{int Mode,DirectionRY;}} CamState;
static int root,angle,volume,pan; static int gSELevel=127;static s16 voice;
#define SOUND_SPATIAL_DIRECTION(value) ((value)>>8)
#define SOUND_SPATIAL_VOLUME(value) ((value)&127)
#define SOUND_PAN_ANGLE_MASK 1023
#define SOUND_PAN_ANGLE_SHIFT 4
#define SOUND_PAN_CENTER 64
#define SOUND_LEVEL_SHIFT 7
#define SOUND_VOICE_COUNT 24
#define SOUND_ID_PROGRAM(value) ((value)>>4)
#define SOUND_ID_TONE(value) ((value)&15)
#define SOUND_KEY_NOTE 36
#define SOUND_VOLUME_MAX 127
#define SOUND_SPATIAL(direction,volume) (((direction)<<8)|(volume))
#define SE_RUN_STEP 0x12
#define CMODE_DIRECTION 1
#define ANGLE_HALF 2048
#define ANGLE_FULL 4096
static int SquareRoot0(int value) {{return root;}}
static int ratan2(int y,int x) {{return angle;}}
static int SsUtKeyOnV(int voice,int bank,int prog,int tone,int note,int fine,int l,int r) {{volume=l;return 0;}}
static void SsUtAutoPan(int voice,int start,int end,int time) {{pan=end;}}
{}
{}
int main(void) {{int a[10];while(scanf("%d",&a[0])==1) {{
for(int i=1;i<10;i++)if(scanf("%d",&a[i])!=1)return 2;
VECTOR pos={{a[0],a[1],a[2]}};rotation.vy=a[3];CamState.Mode=a[4];CamState.DirectionRY=a[5];
root=a[8];angle=a[9];volume=pan=-1;voice=0;
SoundEx(a[6]?&pp:&pos,a[7]);printf("%d %d\n",volume,pan);
}}return 0;}}
"#,
        oracle::function(&audio, "short PlaySE("),
        oracle::function(&source, "short SoundEx(")
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut random = Random::new(821);
    for case in 0..4096 {
        let position = if case < 1024 {
            [
                [0, 0, 0],
                [1999, 1999, 0],
                [2000, 2000, 0],
                [17999, 0, 0],
                [18000, 0, 0],
                [0, 9999, -100],
                [0, 10000, 0],
                [0, -10000, 0],
            ][case % 8]
        } else {
            std::array::from_fn(|_| random.next15() * 2 - 32768)
        };
        let listener = Listener {
            position: Position::ZERO,
            yaw: (random.next15() - 16384) as i16,
            direction: (case % 3 == 0).then(|| (random.next15() - 16384) as i16),
        };
        let local = case % 7 == 0;
        let id = [0x12, 0x11, 0x0b, 0x56][case % 4];
        let result = listener
            .sound(&game.math, (!local).then_some(position.into()), id)
            .unwrap();
        expected.push(result.map_or(vec![-1, -1], |sound| {
            vec![i32::from(sound.volume), i32::from(sound.pan)]
        }));
        let [dx, dy, dz] = position;
        writeln!(
            input,
            "{dx} {dy} {dz} {} {} {} {} {id} {} {}",
            listener.yaw,
            u8::from(listener.direction.is_some()),
            listener.direction.unwrap_or(0),
            u8::from(local),
            game.math.length(Position::new(dx, 0, dz)),
            game.math.atan2(-dx, -dz).unwrap()
        )
        .unwrap();
    }
    assert_eq!(oracle::run(&harness, &input), expected);
}
