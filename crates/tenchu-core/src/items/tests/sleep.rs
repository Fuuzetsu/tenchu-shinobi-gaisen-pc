use super::*;
use crate::math::Position;
use crate::{ai::Intelligence, effects::Effect, ids::*, scene::CONFLICT};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original sleeping-gas flight/contact callback with verified particle RNG costs and projection stubbed"]
fn gas_flight_and_contact_match_original() {
    let source = oracle::source("ITEM.C");
    let function = oracle::function(&source, "void ProcItemNemuri(TItem *item)");
    let harness=[r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int32_t s32;typedef int16_t s16;typedef int ConflictClass;
typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {struct {int t[3];} coord;int flag;} Coordinate;
typedef struct Sprite3D {Coordinate locate;int attribute;struct {uint8_t r,g,b;int rotate;} sprite;int scale,n;struct Sprite3D**object;} Sprite3D;
typedef Sprite3D ModelType;
typedef struct {int mid,count;} Motion;
typedef struct {ModelType*model;Motion*motion;int life,type,think[4],attribute;} Humanoid;
typedef struct {SVECTOR vec;uint8_t count;} param_napalm;
typedef struct Item {ModelType*locate,*model;Humanoid*owner;void(*proc)(struct Item*);uint8_t mode;struct {int size,mode,pause,ofsY;} collision;union {param_napalm napalm;} param;} TItem;
enum {ITEM_MODE_DISPOSE=255,MOT_ITEM_THROW=0xf02,MOTION_MOVE_APPLY=1,FIXED_ONE=4096,CONFLICT_SOFT=8,CONFLICT_NONE=-1,CONFLICT_OWNER_ITEM=0,MODEL_ATTR_CONFLICT=0x8000,MODEL_PART_WEAPON_HAND_1=14,AREA_LEVEL_DEFAULT=0,LEVEL_NONE=INT32_MIN,SE_SLEEP_DART_THROW=0x26,SE_SMOKE_PUFF=0x23,MOT_ACTION=0x100,PAGE_MASK=0xf0,PAGE_BOSS=0x80,WEAPON_SHEATHED=0,MOT_STATE_SHEATHE=0x80f,Think1sleep=7,ATTR_PHASE=3,ATTR_WEAPON_DRAWN=0x40,CHAR_VOICE_HURT=6};
#define RGB24(r,g,b) ((r)|((g)<<8)|((b)<<16))
#define MODEL_POSITION(m) ((VECTOR*)(m)->locate.coord.t)
#define copyVector(a,b) do {(a)->vx=(b)->vx;(a)->vy=(b)->vy;(a)->vz=(b)->vz;}while(0)
static struct {void*common;} ConflictObject[2];static Humanoid owner,target;
static uint32_t seed;static int calls,GlobalAreaMap,level,pulse,live,hit,used,bleeds,smokes,sounds,voices,drawn,brightness,rotation,scale;
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767;}
#define rand game_rand
static int rsin(int angle) {return pulse;}
static void SetNowMotion(Humanoid*h,int action,int movement) {h->motion->mid=action;}
static void SoundEx(VECTOR*p,int id) {sounds++;}
static void Sound(Humanoid*h,int id) {voices++;}
static void EquipWeapon(Humanoid*h,int kind) {h->attribute&=~ATTR_WEAPON_DRAWN;}
static VECTOR*GetAbsolutePosition(void*p,int x,int y,int z) {return 0;}
static void DeleteConflict(ModelType*m) {live=0;m->attribute&=~MODEL_ATTR_CONFLICT;}
static int InsertConflict(ModelType*m) {live=1;used=0;m->attribute&=~MODEL_ATTR_CONFLICT;return 0;}
#define SET_ITEM_COLLISION(n,sz,owner_tag,cmode) do {item->collision.size=sz;item->collision.mode=cmode;item->collision.pause=0;item->collision.ofsY=0;}while(0)
#define DISPOSE_ITEM(item) do {item->mode=255;item->proc(item);DeleteConflict(item->locate);item->owner=0;item->proc=0;}while(0)
static int GetConflictResult(ModelType*m,int ignored) {used=1;return 1;}
static int is_humanoid_on_stage_(void*h) {return h==&owner||h==&target;}
static void SetBleeds(VECTOR*p,int range,int spread,int n,int time,int color) {assert(range==300&&spread==10&&n==2&&time==10);bleeds+=n;for(int i=0;i<n*7;i++)rand();}
static void SetSmoke(VECTOR*p,SVECTOR*v,int n,int time) {assert(n==10&&time==30&&v->vy==-150);smokes+=n;for(int i=0;i<n*7;i++)rand();}
static int GetAreaMapLevel(int map,int x,int y,int z,int mode) {return level;}
static void UpdateCoordinate(ModelType*m) {m->locate.flag=0;}
static void DrawSprite(Sprite3D*s) {int z=s->locate.coord.t[2];z=z<0?0:z>65535?65535:z;int d=z>>4;if(d>1250||d<=5)return;drawn=1;brightness=s->sprite.r;rotation=s->sprite.rotate;scale=(int16_t)((s->scale>>2)*300/(d-5));}
"#,function,r#"
int main(void) {int a[16];while(scanf("%d",a)==1){for(int i=1;i<16;i++)if(scanf("%d",a+i)!=1)return 2;
ModelType locate={.locate={{{a[1],a[2],a[3]}},91},.attribute=a[8]?MODEL_ATTR_CONFLICT:0},sprite={0},human_model={.n=a[13]};
Motion motion={a[11],0};target=(Humanoid){&human_model,&motion,a[9],a[10],{0},ATTR_WEAPON_DRAWN|3};
TItem item={.locate=&locate,.model=&sprite,.owner=&owner,.proc=ProcItemNemuri,.mode=2,.param.napalm={{a[4],a[5],a[6],0},a[0]}};
seed=a[7];hit=a[8];level=a[12];pulse=a[14];ConflictObject[1].common=hit==1?(void*)&owner:hit==2?(void*)&target:(void*)1;
calls=used=bleeds=smokes=sounds=voices=drawn=brightness=rotation=scale=0;live=1;
ProcItemNemuri(&item);
printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d\n",locate.locate.coord.t[0],locate.locate.coord.t[1],locate.locate.coord.t[2],item.param.napalm.count,item.mode,item.proc!=0,(int32_t)seed,calls,live,used,bleeds,smokes,sounds,voices,target.motion->mid,target.think[0],target.attribute,target.life,locate.locate.flag,drawn,brightness,rotation,scale);
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        let enemy = actors.roster.spawn(0x10, Position::ZERO, 0, ctx).unwrap();
        let target = ctx.scene.add(None, Matrix::default());
        ctx.collision.insert(ctx.scene, target).unwrap();
        ctx.camera.view = Matrix::default();
        for case in 0..4096 {
            let count = [0, 1, 20, 31, 50, 99, 100, 101, 254, 255][case / 4 % 10];
            let position = [
                [0, -1000, 4000],
                [40000, -1000, 0],
                [i32::MAX, -1, i32::MIN],
            ][case / 40 % 3];
            let velocity =
                [[0, 0, -120], [-32768, 32767, -1], [32767, -32768, 32767]][case / 120 % 3];
            let seed = case as u32 * 1543 + 17;
            let hit = case % 4;
            let health = [-1, 0, 1, 100][case / 9 % 4];
            let kind = if case / 5 % 2 == 0 { 0x10 } else { 0x80 };
            let action = if case / 17 % 2 == 0 {
                MOT_NORMAL
            } else {
                MOT_ACTION
            };
            let actor = actors.get_mut(enemy);
            actor.kind = kind;
            actor.health = health;
            actor.attribute = ATTR_WEAPON_DRAWN | 3;
            actor.think[0] = 0;
            actor.status = 0;
            actor.enter(ctx, action, true);
            items.dispose(1, actors, ctx).unwrap();
            items
                .initialize(
                    1,
                    behavior(Processor::Sleep, Some(player), 0),
                    9,
                    position.into(),
                    ctx,
                )
                .unwrap();
            items.contact(1, 1000, crate::collision::SOFT, ctx).unwrap();
            let item = &mut items.slots[1];
            item.set_source_mode(2);
            item.napalm = crate::items::napalm::Napalm { velocity, count };
            let model = item.model;
            ctx.scene.models[model].flags = 91;
            if hit != 0 {
                ctx.scene.models[model].attribute |= CONFLICT;
            }
            let slot = ctx.scene.models[model].conflict.unwrap();
            let target_slot = ctx.scene.models[target].conflict.unwrap();
            ctx.collision.slots[target_slot].owner = Some(match hit {
                1 => crate::collision::Owner::Actor(player),
                2 => crate::collision::Owner::Actor(enemy),
                _ => crate::collision::Owner::Door,
            });
            ctx.collision.slots[slot].count = i16::from(hit != 0);
            ctx.collision.slots[slot].results[target_slot] = if hit != 0 { 0x84 } else { 0 };
            ctx.collision.other = None;
            ctx.effects.slots.fill(None);
            ctx.effects.sounds.clear();
            items.begin_tick();
            *ctx.random = Random::new(seed);
            let next = std::array::from_fn(|i| position[i].wrapping_add(i32::from(velocity[i])));
            let level = terrain::level(&ctx.mission.terrain, ctx.cursor, next.into(), 0).unwrap();
            let pulse = ctx.math.pair((i32::from(count) * 0x88) as i16)[0];
            writeln!(input,"{count} {} {} {} {} {} {} {seed} {hit} {health} {kind} {action} {level} {} {pulse} 0",position[0],position[1],position[2],velocity[0],velocity[1],velocity[2],actors.get(enemy).parts.len()).unwrap();
            let mut intelligence = Intelligence::default();
            items
                .process(1, actors, &mut intelligence, 0, 0, ctx)
                .unwrap();
            let actor = actors.get(enemy);
            let item = &items.slots[1];
            let mut row = ctx.scene.models[model]
                .local
                .translation
                .to_array()
                .to_vec();
            row.extend([
                i32::from(item.napalm.count),
                i32::from(item.source_mode()),
                i32::from(item.processor().is_some()),
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                i32::from(ctx.scene.models[model].conflict.is_some()),
                i32::from(ctx.collision.other == Some(target)),
                ctx.effects
                    .slots
                    .iter()
                    .filter(|e| matches!(e, Some(Effect::Bleed(_))))
                    .count() as i32,
                ctx.effects
                    .slots
                    .iter()
                    .filter(|e| matches!(e, Some(Effect::Smoke(_))))
                    .count() as i32,
                ctx.effects.sounds.len() as i32,
                intelligence.voices.len() as i32,
                i32::from(actor.motion.action),
                i32::from(actor.think[0]),
                i32::from(actor.attribute),
                i32::from(actor.health),
                ctx.scene.models[model].flags as i32,
                items.drawings.len() as i32,
            ]);
            row.extend(
                items
                    .drawings
                    .first()
                    .map_or([0; 3], |d| [i32::from(d.color[0]), d.rotation, d.scale[0]]),
            );
            expected.push(row);
        }
    });
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
#[ignore = "requires TENCHU_DISC; release retains coordinate caches and interruption consumes the item without a refund"]
fn hand_release_and_interruption_keep_source_side_effects() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        items
            .initialize(
                1,
                behavior(Processor::Sleep, Some(player), 0),
                9,
                Position::ZERO,
                ctx,
            )
            .unwrap();
        actors.get_mut(player).health = 0;
        items
            .process(1, actors, &mut Intelligence::default(), 0, 0, ctx)
            .unwrap();
        assert_eq!(actors.get(player).motion.action, MOT_ITEM_THROW);
        let model = items.slots[1].model;
        let cached = Matrix {
            translation: Position::new(700, 800, 900),
            ..Matrix::default()
        };
        ctx.scene.models[model].cached = cached;
        ctx.scene.models[model].flags = 91;
        actors.get_mut(player).motion.count = 3;
        let hand = ctx
            .scene
            .world(actors.get(player).parts[14])
            .unwrap()
            .translation;
        let random = *ctx.random;
        items
            .process(1, actors, &mut Intelligence::default(), 0, 0, ctx)
            .unwrap();
        assert_eq!(ctx.scene.models[model].local.translation, hand);
        assert_eq!(ctx.scene.models[model].cached, cached);
        assert_eq!(ctx.scene.models[model].flags, 91);
        assert_eq!(items.slots[1].source_mode(), 2);
        assert_eq!(*ctx.random, random);
        items.dispose(1, actors, ctx).unwrap();
        items
            .initialize(
                1,
                behavior(Processor::Sleep, Some(player), 0),
                9,
                Position::ZERO,
                ctx,
            )
            .unwrap();
        items.slots[1].set_source_mode(1);
        actors.get_mut(player).enter(ctx, MOT_DAMAGE, true);
        items
            .process(1, actors, &mut Intelligence::default(), 0, 0, ctx)
            .unwrap();
        assert!(items.slots.iter().all(|item| item.processor().is_none()));
        assert_eq!(*ctx.random, random);
    });
}
