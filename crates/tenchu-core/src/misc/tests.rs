use super::*;
use crate::math::Position;
use crate::{
    effects::{Effect, Effects},
    game::Game,
    oracle,
    random::Random,
};
use std::fmt::Write;

fn with_misc<T>(game: &mut Game, f: impl FnOnce(&mut Misc, &mut Context) -> T) -> T {
    f(
        &mut game.misc,
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
    )
}

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original misc callbacks, with previously verified effect RNG costs stubbed"]
fn emitter_callbacks_match_original_order_rng_and_schedules() {
    let source = oracle::source("MISC.C");
    let functions = [
        "static inline void ProcMiscFire(TMisc *m, TMiscMessage msg)",
        "static void proc_misc_bonfire_(TMisc *m, TMiscMessage msg)",
        "static void proc_misc_sound_(TMisc *m, TMiscMessage msg)",
        "static inline void proc_misc_puff_(TMisc *m, TMiscMessage msg)",
        "static void ProcMiscSnowfall(TMisc *m, TMiscMessage msg)",
        "static inline void ProcMiscSprite(TMisc *m, TMiscMessage msg)",
    ]
    .map(|signature| oracle::function(&source, signature))
    .join("\n");
    let harness = [r#"
#include <stdio.h>
#include <stdint.h>
typedef int32_t s32; typedef uint8_t u8; typedef u8 misc_sprite_kind; typedef int TMiscMessage;
typedef struct {s32 vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {u8 r,g,b;} GsSPRITE;
typedef struct {struct {int t[3];} coord;} Coordinate;
typedef struct {Coordinate locate; GsSPRITE sprite;int scale;} Sprite3D;
typedef Sprite3D ModelType;
typedef struct {s32 next;int16_t min_delay,max_delay;u8 sound_index;} MiscSoundSchedule;
typedef struct {s32 w,h;SVECTOR*snows;} TSnowfall;
typedef struct {s32 x,y,z,count;u8 mode;union {
struct {s32 a,b,c;} init;
struct {s32 scale;} bonfire;
struct {s32 sound,min_delay,max_delay;} sound_init;
MiscSoundSchedule sound;
struct {s32 selector,x_radius,z_radius;} puff;
TSnowfall snowfall;
struct {u8 type;} sprite;
} param;} TMisc;
static struct {int vrx,vry,vrz;} ViewInfo;
static struct {Sprite3D*spr;int scale;} SpriteData[2];
static Sprite3D sprites[2];static GsSPRITE sprFrame[4];
static int GameClock,sounds,sound_id,sound_pos[3],counts[6],last_pos[3],last_velocity[3];
static uint32_t seed;static int calls;
static int game_rand(void) { seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767; }
#define rand game_rand
#define RGB24(r,g,b) ((r)|((g)<<8)|((b)<<16))
enum {MM_CREATE,MM_DESTROY,MM_PAUSE,MM_RESUME,MM_DO};
enum {MaxFrames=4,FIXED_ONE=4096,SNOW_SPAN=6000,SNOW_RANGE=3000,SNOW_SPRITE_DEFAULT=0,SE_FIRE=0x28,SE_BONFIRE=0x47,MISC_SOUND_ID_BASE=0x44,N_MISC_SPRITE_TYPES=2,MISC_SPRITE_FIRE1=0};
static char *msg_unknown_sprite_type;
static void AdtMessageBox(void*x,...){}
static void consume(int n) {while(n-->0)rand();}
static void SetExplosion(VECTOR*p,SVECTOR*v) {counts[3]++;counts[2]+=20;consume(141);}
static void SetHinoko(VECTOR*p,SVECTOR*v,int n) {counts[4]+=n;consume(n*6);}
static void SetSmoke(VECTOR*p,SVECTOR*v,int n,int t) {counts[0]+=n;consume(n*7);last_pos[0]=p->vx;last_pos[1]=p->vy;last_pos[2]=p->vz;}
static void SetSplash(VECTOR*p,int a,int b,int c) {counts[1]++;}
static void SetBleedsDir(VECTOR*p,SVECTOR*v,int spread,int n,int t,int c) {counts[2]+=n;consume(n*4);}
static void SetSnow(VECTOR*p,SVECTOR*v,int size,int sprite) {counts[5]++;last_pos[0]=p->vx;last_pos[1]=p->vy;last_pos[2]=p->vz;last_velocity[0]=v->vx;last_velocity[1]=v->vy;last_velocity[2]=v->vz;}
static void SoundEx(VECTOR*p,int id) {sounds++;sound_id=id;sound_pos[0]=p->vx;sound_pos[1]=p->vy;sound_pos[2]=p->vz;}
static void DrawSpriteXYZ(GsSPRITE*s,int x,int y,int z,int scale) {}
static void DrawSprite(Sprite3D*s) {}
static void UpdateCoordinate(ModelType*s) {}
"#, &functions, r#"
int main(void) {int a[8];while(scanf("%d",&a[0])==1) {
for(int i=1;i<8;i++)if(scanf("%d",&a[i])!=1)return 2;
TMisc m={0};m.x=100;m.y=-500;m.z=1000;m.mode=a[2];m.count=a[3];
seed=a[7];calls=sounds=0;sound_id=-1;GameClock=a[1];
for(int i=0;i<6;i++)counts[i]=0;
for(int i=0;i<3;i++)last_pos[i]=last_velocity[i]=sound_pos[i]=0;
SpriteData[0].spr=&sprites[0];SpriteData[1].spr=&sprites[1];
ViewInfo.vrx=100;ViewInfo.vry=-500;ViewInfo.vrz=1000;
switch(a[0]) {
case 0:ProcMiscFire(&m,MM_DO);break;
case 1:m.param.puff.x_radius=a[4];m.param.puff.z_radius=a[5];proc_misc_puff_(&m,MM_DO);break;
case 2:m.param.sound=(MiscSoundSchedule){a[3],a[4],a[5],a[6]};proc_misc_sound_(&m,MM_DO);m.count=m.param.sound.next;break;
case 3:ProcMiscSnowfall(&m,MM_DO);break;
case 4:m.param.sprite.type=a[6]&1;ProcMiscSprite(&m,MM_DO);break;
case 5:m.param.bonfire.scale=a[4];proc_misc_bonfire_(&m,MM_DO);break;
}
printf("%d %d %d %d %d",m.count,(int32_t)seed,calls,sounds,sound_id);
for(int i=0;i<6;i++)printf(" %d",counts[i]);
for(int i=0;i<3;i++)printf(" %d",sound_pos[i]);
for(int i=0;i<3;i++)printf(" %d",last_pos[i]);
for(int i=0;i<3;i++)printf(" %d",last_velocity[i]);
puts("");
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    game.camera.reference = Position::new(100, -500, 1000);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..4096 {
        let kind = case % 6;
        let clock = [0, 1, 4, 10, 16, 79, 80, 160, 237, 400, 1024, i32::MAX][case / 6 % 12];
        let mode = (case / 72 % 2) as u8;
        let count = [0, 1, 2, 10, 300, i32::MIN, i32::MAX][case / 144 % 7];
        let a = [100, 0, -1, 32767, -32768, 65536][case / 7 % 6];
        let b = [200, 0, -1, 32767, -32768, 70000][case / 11 % 6];
        let index = (case % 3) as u8;
        let seed = case as u32 * 97 + 1;
        writeln!(
            input,
            "{kind} {clock} {mode} {count} {a} {b} {index} {seed}"
        )
        .unwrap();
        game.effects = Effects::default();
        game.random = Random::new(seed);
        with_misc(&mut game, |misc, ctx| {
            misc.objects.clear();
            misc.objects.push(Object {
                position: Position::new(100, -500, 1000),
                paused: false,
                mode,
                count,
                model: None,
                angle: 0,
                speed: 0,
                kind: match kind {
                    0 => Kind::Fire,
                    1 => Kind::Puff([a, b]),
                    2 => Kind::Sound {
                        index,
                        delay: [a as i16, b as i16],
                    },
                    3 => Kind::Snowfall,
                    4 => Kind::Sprite(usize::from(index & 1)),
                    _ => Kind::Bonfire(a),
                },
            });
            misc.step(clock, ctx).unwrap();
            let mut counts = [0; 6];
            let mut last_pos = [0; 3];
            let mut last_velocity = [0; 3];
            for effect in ctx.effects.slots.iter().flatten() {
                let index = match effect {
                    Effect::Smoke(s) => {
                        last_pos = s.position.into();
                        0
                    }
                    Effect::Splash(_) => 1,
                    Effect::Bleed(_) => 2,
                    Effect::Explosion(_) => 3,
                    Effect::Ember(_) => 4,
                    Effect::Snow(s) => {
                        last_pos = s.position.into();
                        last_velocity = s.velocity.map(i32::from);
                        5
                    }
                    _ => unreachable!(),
                };
                counts[index] += 1;
            }
            let (sound_pos, sound) = ctx
                .effects
                .sounds
                .last()
                .copied()
                .unwrap_or((Position::ZERO, -1));
            let mut row = vec![
                misc.objects[0].count,
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                ctx.effects.sounds.len() as i32,
                i32::from(sound),
            ];
            row.extend(counts);
            row.extend(sound_pos);
            row.extend(last_pos);
            row.extend(last_velocity);
            expected.push(row);
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

mod hinged;
