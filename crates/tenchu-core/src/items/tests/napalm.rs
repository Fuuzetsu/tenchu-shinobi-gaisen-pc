use super::*;
use crate::math::Position;
use crate::{effects::Effect, scene::CONFLICT};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original napalm C with recorded terrain responses and projection stubbed"]
fn expansion_collision_and_terrain_exit_match_original() {
    let source = oracle::source("ITEM.C");
    let function = oracle::function(&source, "void ProcItemNapalm(TItem *item)");
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int32_t s32;typedef uint8_t u8;
typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {struct {int t[3];} coord;int flag;} Coordinate;
typedef struct {Coordinate locate;int attribute;struct {u8 r,g,b;int rotate;} sprite;int scale;} Sprite3D;
typedef Sprite3D ModelType;
typedef struct {SVECTOR vec;u8 count;} param_napalm;
typedef struct Item {ModelType*locate,*model;void*owner;void(*proc)(struct Item*);u8 mode;
struct {int size,mode,pause,ofsY;} collision;union {param_napalm napalm;} param;} TItem;
typedef struct {struct {ModelType**object;int n;}*model;} Humanoid;
static struct {void*common;} ConflictObject[2];
static Sprite3D primary,secondary,*sprNapalm2=&secondary;
static ModelType parts[40],*objects[40];static Humanoid human;
static int GlobalAreaMap,level,hit,live,used,flashes,part,local[3],queries,draws,brightness[2],rotation[2],scale[2];
static uint32_t seed;static int calls;
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767;}
#define rand game_rand
enum {ITEM_MODE_DISPOSE=255,FIXED_ONE=4096,CONFLICT_NONE=-1,CONFLICT_OWNER_ITEM=0,CONFLICT_HIT=1,MODEL_ATTR_CONFLICT=0x8000,AREA_LEVEL_DEFAULT=0,LEVEL_NONE=INT32_MIN};
static void DeleteConflict(ModelType*m) {live=0;m->attribute&=~MODEL_ATTR_CONFLICT;}
static int InsertConflict(ModelType*m) {live=1;used=0;m->attribute&=~MODEL_ATTR_CONFLICT;return 0;}
#define SET_ITEM_COLLISION(n,sz,owner_tag,cmode) do {item->collision.size=sz;item->collision.mode=cmode;item->collision.pause=0;item->collision.ofsY=0;}while(0)
#define DISPOSE_ITEM(item) do {item->mode=255;item->proc(item);DeleteConflict(item->locate);item->owner=0;item->proc=0;}while(0)
static int GetConflictResult(ModelType*m,int ignored) {if(hit==0)return -1;used=1;return 1;}
static int is_humanoid_on_stage_(void*h) {return h==&human;}
static void SetFrame(VECTOR*p,int size,int count,Coordinate*c) {assert(size==12288&&count==60);flashes++;for(int i=0;i<human.model->n;i++)if(c==&parts[i].locate)part=i;local[0]=p->vx;local[1]=p->vy;local[2]=p->vz;}
static int GetAreaMapLevel(int map,int x,int y,int z,int mode) {queries++;return level;}
static void UpdateCoordinate(ModelType*m) {m->locate.flag=0;}
static void DrawSprite(Sprite3D*s) {
    int z=s->locate.coord.t[2];z=z<0?0:z>65535?65535:z;
    int d=z>>4;if(d>1250||d<=5)return;
    brightness[draws]=s->sprite.r;rotation[draws]=s->sprite.rotate;scale[draws]=(int16_t)((s->scale>>2)*300/(d-5));draws++;
}
"#, function, r#"
int main(void) {int a[13];while(scanf("%d",a)==1) {
for(int i=1;i<13;i++)if(scanf("%d",a+i)!=1)return 2;
ModelType locate={.locate={{ {a[2],a[3],a[4]} },91},.attribute=MODEL_ATTR_CONFLICT};
TItem item={.locate=&locate,.model=&primary,.owner=&human,.proc=ProcItemNapalm,.mode=a[0],.collision={333,7,1,11},.param.napalm={{a[5],a[6],a[7],0},a[1]}};
struct {ModelType**object;int n;} archive={objects,a[11]};human.model=(void*)&archive;
for(int i=0;i<a[11];i++)objects[i]=&parts[i];
seed=(uint32_t)a[8];hit=a[9];level=a[10];ConflictObject[1].common=hit==1?(void*)&human:(void*)1;
calls=used=flashes=queries=draws=0;part=-1;live=1;for(int i=0;i<3;i++)local[i]=0;
ProcItemNapalm(&item);
printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d",locate.locate.coord.t[0],locate.locate.coord.t[1],locate.locate.coord.t[2],item.param.napalm.count,item.mode,item.proc!=0,(int32_t)seed,calls,live,item.collision.size,item.collision.mode,item.collision.pause,item.collision.ofsY,used,flashes,part,local[0],local[1],local[2],locate.locate.flag,draws);
for(int i=0;i<draws;i++)printf(" %d %d %d",brightness[i],rotation[i],scale[i]);puts("");
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    game.roster.get_mut(player).health = 0;
    game.camera.view = Matrix::default();
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        let target = ctx.scene.add(None, Matrix::default());
        ctx.collision.insert(ctx.scene, target).unwrap();
        for case in 0..4096 {
            let mode = if case % 31 == 0 {
                (case / 31 % 2) * 2
            } else {
                1
            };
            let count = [0, 1, 9, 10, 11, 19, 20, 21, 127, 255][case / 3 % 10];
            let position = [
                [0, -1000, 4000],
                [40000, -500, 0],
                [i32::MAX, -100, i32::MIN],
            ][case / 30 % 3];
            let velocity =
                [[99, -15, -99], [-32768, 32767, -300], [0; 3], [1, -1, 1]][case / 90 % 4];
            let seed = case as u32 * 7919 + 17;
            let hit = case % 3;
            let id = 1;
            items.dispose(id, actors, ctx).unwrap();
            items
                .initialize(
                    id,
                    Processor::Napalm,
                    22,
                    Some(player),
                    position.into(),
                    ctx,
                )
                .unwrap();
            items.contact(id, 333, 7, ctx).unwrap();
            let item = &mut items.slots[id];
            item.mode = mode as u8;
            item.napalm = crate::items::napalm::Napalm { velocity, count };
            item.collision.paused = true;
            item.collision.offset_y = 11;
            let model = item.model;
            ctx.scene.models[model].attribute |= CONFLICT;
            ctx.scene.models[model].flags = 91;
            let slot = ctx.scene.models[model].conflict.unwrap();
            let target_slot = ctx.scene.models[target].conflict.unwrap();
            ctx.collision.slots[target_slot].owner = Some(if hit == 1 {
                player
            } else {
                crate::collision::DOOR_OWNER
            });
            ctx.collision.slots[slot].count = i16::from(hit != 0);
            ctx.collision.slots[slot].results[target_slot] = if hit != 0 { 0x84 } else { 0 };
            ctx.collision.other = None;
            *ctx.random = Random::new(seed);
            ctx.effects.slots.fill(None);
            items.begin_tick();
            let next = std::array::from_fn(|i| {
                position[i].wrapping_add(i32::from(velocity[i]) * i32::from(count).pow(2) / 100)
            });
            let level = terrain::level(&ctx.mission.terrain, ctx.cursor, next.into(), 0).unwrap();
            writeln!(
                input,
                "{mode} {count} {} {} {} {} {} {} {seed} {hit} {level} {} 0",
                position[0],
                position[1],
                position[2],
                velocity[0],
                velocity[1],
                velocity[2],
                actors.get(player).parts.len()
            )
            .unwrap();
            items.napalm(id, actors, ctx).unwrap();
            let item = &items.slots[id];
            let flash = ctx.effects.slots.iter().find_map(|effect| {
                if let Some(Effect::Flash(f)) = effect {
                    Some(f)
                } else {
                    None
                }
            });
            let mut row = ctx.scene.models[model]
                .local
                .translation
                .to_array()
                .to_vec();
            row.extend([
                i32::from(item.napalm.count),
                i32::from(item.mode),
                i32::from(item.processor.is_some()),
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                i32::from(ctx.scene.models[model].conflict.is_some()),
                i32::from(item.collision.size),
                i32::from(item.collision.class),
                i32::from(item.collision.paused),
                i32::from(item.collision.offset_y),
                i32::from(ctx.collision.other == Some(target)),
                i32::from(flash.is_some()),
                flash.map_or(-1, |f| {
                    actors
                        .get(player)
                        .parts
                        .iter()
                        .position(|p| Some(*p) == f.parent)
                        .unwrap() as i32
                }),
            ]);
            row.extend(flash.map_or([0; 3], |f| f.position.into()));
            row.extend([
                ctx.scene.models[model].flags as i32,
                items.drawings.len() as i32,
            ]);
            for drawing in &items.drawings {
                row.extend([
                    i32::from(drawing.color[0]),
                    drawing.rotation,
                    drawing.scale[0],
                ]);
            }
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
#[ignore = "requires TENCHU_DISC; an even-tick request still evicts a full slot and clears aimed-item ownership"]
fn rejected_request_preserves_allocator_side_effects() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
            items
                .initialize(
                    id,
                    Processor::SightShot,
                    1,
                    Some(player),
                    Position::ZERO,
                    ctx,
                )
                .unwrap();
        }
        items.cursor = 0;
        actors.get_mut(player).inventory[25] = 1;
        actors.get_mut(player).inventory[22] = 3;
        items
            .request_napalm(
                player,
                Position::ZERO,
                Position::new(99, -15, -99),
                2,
                actors,
                ctx,
            )
            .unwrap();
        assert_eq!(items.cursor, 29);
        assert!(items.slots[29].processor.is_none());
        assert_eq!(actors.get(player).inventory[25], 0);
        assert_eq!(actors.get(player).inventory[22], 2);
        items
            .request_napalm(
                player,
                Position::ZERO,
                Position::new(65537, -32769, -99),
                3,
                actors,
                ctx,
            )
            .unwrap();
        assert_eq!(items.cursor, 28);
        assert_eq!(items.slots[28].processor, Some(Processor::Napalm));
        assert_eq!(items.slots[28].napalm.velocity, [1, 32767, -99]);
    });
}
