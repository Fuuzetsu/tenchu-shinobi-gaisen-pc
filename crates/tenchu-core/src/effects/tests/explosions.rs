use super::*;
use crate::oracle;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DECOMP; compares every smoke/explosion/ember state through expiry with original C"]
fn particle_lifetimes_match_original_c() {
    let source = oracle::source("EFFECT.C");
    let routines = [
        "static void DrawSmoke(",
        "static void DrawExplosion(",
        "static void DrawHinoko(",
    ]
    .map(|signature| oracle::function(&source, signature).replace("long ", "int32_t "))
    .join("\n");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
typedef int32_t s32; typedef uint8_t u8;
typedef struct {{ int16_t vx,vy,vz,pad; }} SVECTOR;
typedef struct {{ int32_t vx,vy,vz,pad; }} VECTOR;
typedef struct {{ SVECTOR vec; VECTOR pos; int32_t rotate,scale; uint8_t time,evtime,sprite; }} SmokeType;
typedef struct {{ SVECTOR vec; VECTOR pos; int32_t rotate,scale; uint8_t time,mode; }} ExplosionType;
typedef struct TEffectSlot {{ void (*proc)(struct TEffectSlot *); union {{ SmokeType smoke; ExplosionType explosion,hinoko; }} param; }} TEffectSlot;
typedef struct {{ struct {{ struct {{ int32_t t[3]; }} coord; }} locate; int32_t scale; struct {{ uint8_t r,g,b; int32_t rotate; }} sprite; }} Sprite3D;
typedef Sprite3D ModelType;
static Sprite3D sprites[5];
static Sprite3D *sprSmoke[]={{sprites,sprites+1}};
static Sprite3D *sprBomb[]={{sprites+2,sprites+3,sprites+4}};
#define FIXED_ONE 4096
#define BOMB_SPRITE_FLASH 0
#define BOMB_SPRITE_EXPANDED 1
#define BOMB_SPRITE_HINOKO 2
#define EXPLOSION_MODE_FLASH 0
#define EXPLOSION_MODE_EXPAND 1
#define EXPLOSION_MODE_FADE 2
static uint32_t seed;
static int calls,image,color;
int rand(void) {{ seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767; }}
static void UpdateCoordinate(ModelType *model) {{}}
static void DrawSprite(Sprite3D *sprite) {{ image=sprite-sprites;color=sprite->sprite.r; }}
{routines}
int main(void) {{
    int a[13];
    while(scanf("%d",a)==1) {{
        for(int i=1;i<13;i++) assert(scanf("%d",a+i)==1);
        TEffectSlot slot={{0}};SmokeType *p=&slot.param.smoke;
        p->pos=(VECTOR){{a[1],a[2],a[3],0}};p->vec=(SVECTOR){{a[4],a[5],a[6],0}};
        p->scale=a[7];p->rotate=a[8];p->time=a[9];p->evtime=a[10];p->sprite=a[11];seed=(uint32_t)a[12];calls=0;
        slot.proc=a[0]==0?DrawSmoke:a[0]==1?DrawExplosion:DrawHinoko;
        for(int frame=0;slot.proc;frame++) {{
            assert(frame<256);slot.proc(&slot);
            printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d\n",slot.proc!=0,p->pos.vx,p->pos.vy,p->pos.vz,p->vec.vx,p->vec.vy,p->vec.vz,p->scale,p->rotate,p->time,p->evtime,image,color,(int)seed,calls);
        }}
    }}
}}
"#
    );
    let mut random = Random::new(0x836735);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..384 {
        let kind = case % 3;
        let position = [random.next15() % 1000, -1000, 6000];
        let velocity = [
            (random.next15() % 30 - 15) as i16,
            (random.next15() % 150 - 100) as i16,
            (random.next15() % 30 - 15) as i16,
        ];
        let scale = random.next15() % 8192 + 4096;
        let rotation = random.next15().wrapping_mul(random.next15());
        let time = (random.next15() % 40) as u8;
        let mode = if kind == 0 {
            (random.next15() % 30) as u8
        } else {
            (case / 3 % if kind == 1 { 3 } else { 2 }) as u8
        };
        let sprite = (case / 3 % 2) as u8;
        let seed = random.next15() as u32;
        for v in [kind]
            .into_iter()
            .chain(position)
            .chain(velocity.map(i32::from))
            .chain([
                scale,
                rotation,
                i32::from(time),
                i32::from(mode),
                i32::from(sprite),
                seed as i32,
            ])
        {
            write!(input, "{v} ").unwrap();
        }
        writeln!(input).unwrap();
        let mut smoke = Smoke {
            position,
            velocity,
            scale,
            rotation,
            time,
            change_at: mode,
            sprite,
        };
        let mut explosion = Explosion {
            position,
            velocity,
            scale,
            rotation,
            time,
            mode,
        };
        let mut effects = Effects::default();
        let mut rng = Random::new(seed);
        let view = View {
            position: [0; 3],
            matrix: Matrix::default(),
        };
        loop {
            effects.drawings.clear();
            let alive = if kind == 0 {
                effects.update_smoke(&mut smoke, &mut rng, view)
            } else {
                effects.update_explosion(&mut explosion, kind == 2, view)
            };
            let drawing = effects.drawings.last().unwrap();
            let image = match drawing.graphic {
                Graphic::Smoke(i) => i,
                Graphic::Explosion(i) => i + 2,
                _ => unreachable!(),
            };
            let state = if kind == 0 {
                (
                    smoke.position,
                    smoke.velocity,
                    smoke.scale,
                    smoke.rotation,
                    smoke.time,
                    smoke.change_at,
                )
            } else {
                (
                    explosion.position,
                    explosion.velocity,
                    explosion.scale,
                    explosion.rotation,
                    explosion.time,
                    explosion.mode,
                )
            };
            expected.push(
                [i32::from(alive)]
                    .into_iter()
                    .chain(state.0)
                    .chain(state.1.map(i32::from))
                    .chain([
                        state.2,
                        state.3,
                        i32::from(state.4),
                        i32::from(state.5),
                        i32::from(image),
                        i32::from(drawing.color[0]),
                        rng.seed as i32,
                        rng.calls as i32,
                    ])
                    .collect::<Vec<_>>(),
            );
            if !alive {
                break;
            }
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, &expected, "particle update {i}");
    }
}
