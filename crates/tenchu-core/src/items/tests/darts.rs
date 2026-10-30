use super::*;
use crate::{actor::trail::Trail, collision, scene::CONFLICT};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares eight-shot requests with original C, using supplied terrain targets and SDK distances"]
fn dart_requests_match_original_targeting_and_rng_order() {
    let source = oracle::source("ITEM.C");
    let header = oracle::source("item.h");
    let mut routines = String::new();
    for name in ["INITIALIZE_ITEM_FROM_REQUEST", "TAKE_ITEM_SLOT_VIA_CURSOR"] {
        let start = header.find(&format!("#define {name}")).unwrap();
        routines.push_str(header[start..].split("\n\n").next().unwrap());
        routines.push('\n');
    }
    for signature in [
        "static inline long SubFlyJitter(",
        "static void SetupFly(",
        "static int ReqItemHappou(PARAM_ITEM_LAUNCH *p)\n{",
    ] {
        routines.push_str(&oracle::function(&source, signature).replace("long ", "s32 "));
        routines.push('\n');
    }
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
#include <string.h>
typedef int32_t s32;typedef uint32_t u32;typedef uint8_t u8;
typedef struct {int32_t vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {struct {struct {int t[3];}coord;void*super;int flag;}locate;SVECTOR rotate;} ModelType;
typedef struct {ModelType*model;int motion;} Humanoid;
typedef struct {ModelType*parent;SVECTOR vector1,vector2;} AfterimageType;
struct tag_fly {int32_t sx,sy,sz,vx,vy,vz,rx,ry,rz;uint8_t count,count2;};
typedef struct {union {struct tag_fly fly;}p;uint8_t mode;} param_fly;
typedef struct {param_fly fly;AfterimageType*effect;uint8_t count;} param_launch;
typedef struct Item {Humanoid*owner;void(*proc)(struct Item*);int type;uint8_t mode;ModelType*locate;struct {int size;}collision;union {param_launch launch;}param;} TItem;
typedef struct {int type;Humanoid*user;VECTOR start,end;} PARAM_ITEM_LAUNCH;
enum {MAX_ITEMS=30,ITEM_MODE_START=0,ITEM_MODE_DISPOSE=255,FLY_MODE_ARC=0,FIXED_ONE=4096,FIXED_SHIFT=12,FIXED_TRUNC_BIAS=4095,FIXED_QUARTER=1024,MOT_ITEM_THROW=0xf02,MOTION_MOVE_APPLY=1,SE_ITEM_USE=0x4c};
#define setVector(v,x,y,z) ((v)->vx=(x),(v)->vy=(y),(v)->vz=(z))
#define copyVector(to,from) setVector((to),(from)->vx,(from)->vy,(from)->vz)
static const char msg_item_dispose_fail[]="dispose";
static void DeleteConflict(ModelType*m) {assert(0);}
static void AdtMessageBox(const char*m,int type,u32 mode) {assert(0);}
static TItem items[MAX_ITEMS];static ModelType models[MAX_ITEMS];static AfterimageType trails[MAX_ITEMS];static int ic;
static uint32_t seed;static int calls,n,sound,targets[8][4];static SVECTOR aims[8];
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767;}
#define rand game_rand
static int GetVectorDistance(VECTOR*a,VECTOR*b) {return targets[n-1][3];}
static void UpdateCoordinate(ModelType*m) {m->locate.flag=0;}
static void ProcItemHappou(TItem*i) {assert(0);}
static void SetNowMotion(Humanoid*h,int motion,int move) {assert(move==1);h->motion=motion;}
static void SearchItemTarget2(Humanoid*h,SVECTOR*r,VECTOR*s,VECTOR*t) {aims[n]=*r;setVector(t,targets[n][0],targets[n][1],targets[n][2]);n++;}
static AfterimageType*SetupAfterimage(ModelType*m,int length) {assert(length==10);AfterimageType*t=&trails[m-models];t->parent=m;return t;}
static void Sound(Humanoid*h,int id) {sound=id;}
"#, &routines, r#"
int main(void) {int a[7];while(scanf("%d",a)==1) {
for(int i=1;i<7;i++)assert(scanf("%d",a+i)==1);
for(int i=0;i<8;i++)for(int j=0;j<4;j++)assert(scanf("%d",&targets[i][j])==1);
memset(items,0,sizeof(items));memset(models,0,sizeof(models));memset(trails,0,sizeof(trails));
for(int i=0;i<MAX_ITEMS;i++)items[i].locate=&models[i];
ModelType model={.rotate={a[4],a[5],a[6],0}};Humanoid human={&model,0};
PARAM_ITEM_LAUNCH p={15,&human,{a[1],a[2],a[3],0},{0}};
seed=(uint32_t)a[0];calls=n=ic=0;sound=-1;assert(ReqItemHappou(&p)==1);assert(n==8);
printf("%d %d %d %d",(int32_t)seed,calls,sound,human.motion);
for(int i=1;i<=8;i++){TItem*t=&items[i];struct tag_fly*f=&t->param.launch.fly.p.fly;AfterimageType*e=t->param.launch.effect;
printf(" %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d",aims[i-1].vx,aims[i-1].vy,aims[i-1].vz,t->locate->rotate.vx,t->locate->rotate.vy,t->locate->rotate.vz,f->rx,f->ry,f->rz,f->count,f->count2,t->param.launch.count,e->vector1.vx,e->vector2.vx,e->parent==t->locate);}
puts("");}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut targets = Vec::new();
    let mut starts = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for case in 0..128 {
            for id in 0..CAPACITY {
                items.dispose(id, actors, ctx).unwrap();
            }
            items.cursor = 0;
            let seed = case as u32 * 7919 + 17;
            *ctx.random = Random::new(seed);
            let position = actors.get(player).position;
            let start = [
                position[0] + case * 30,
                position[1] - 1200,
                position[2] + case * 15,
            ];
            let rotation = [
                (-1000 + case * 13) as i16,
                (case * 913 - 32000) as i16,
                (case * 17) as i16,
            ];
            actors.get_mut(player).rotation = rotation;
            ctx.effects.sounds.clear();
            items
                .request_darts(player, start.into(), actors, ctx)
                .unwrap();
            writeln!(
                input,
                "{seed} {} {} {} {} {} {}",
                start[0], start[1], start[2], rotation[0], rotation[1], rotation[2]
            )
            .unwrap();
            let mut row = vec![
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                ctx.effects.sounds.last().unwrap().1 as i32,
                actors.get(player).motion.action as i32,
            ];
            let mut ends = Vec::new();
            for item in &items.slots[1..=8] {
                let Flight::Arc(f) = &item.flight else {
                    panic!("new dart must start in its arc");
                };
                let distance = ctx.math.vector_distance(f.end.wrapping_sub(start.into()));
                writeln!(input, "{} {} {} {distance}", f.end[0], f.end[1], f.end[2]).unwrap();
                ends.push(f.end);
                row.extend(item.rotation.map(i32::from));
                row.extend(f.control);
                let t = item.trail.as_ref().unwrap();
                row.extend([
                    f.count as i32,
                    i32::from(f.total.get()),
                    item.count as i32,
                    i32::from(t.vectors[0][0]),
                    i32::from(t.vectors[1][0]),
                    i32::from(t.parent == item.model),
                ]);
            }
            starts.push(start);
            targets.push(ends);
            expected.push(row);
        }
    });
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    with_items(&mut game, |_, ctx, actors| {
        for (case, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
            assert_eq!(&actual[..4], &expected[..4], "case {case}");
            for shot in 0..8 {
                let a = &actual[4 + shot * 15..4 + (shot + 1) * 15];
                let e = &expected[4 + shot * 12..4 + (shot + 1) * 12];
                assert_eq!(&a[3..], e, "case {case} shot {shot}");
                let rotation = std::array::from_fn(|i| a[i] as i16);
                let end = projectile::target(player, rotation, starts[case].into(), actors, ctx)
                    .unwrap()
                    .1;
                assert_eq!(end, targets[case][shot], "case {case} shot {shot} aim");
            }
        }
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; a dart hit removes its hitbox while flight/trail continue, and ground impact disposes it"]
fn dart_contact_keeps_flying_until_ground_impact() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        let start = actors.get(player).position.map(|v| v - 1000);
        let end = [start[0], start[1], start[2] + 10000];
        items.initialize(
            1,
            behavior(Processor::Darts, Some(player), 0),
            Kind::Darts,
            start,
            ctx,
        );
        let model = items.slots[1].model;
        items.slots[1].flight = Flight::new(start, end.into(), 4096, 1024, 400, ctx);
        items.slots[1].trail = Some(Trail::new(model, [[30, 0, 0], [-30, 0, 0]]));
        items.slots[1].count = 250;
        items.contact(1, 300, collision::HIT, ctx).unwrap();
        let hit = ctx.scene.models[actors.get(player).parts[0]]
            .conflict
            .unwrap();
        let own = ctx.scene.models[model].conflict.unwrap();
        ctx.collision.slots[own].results[hit] = 0x81;
        ctx.collision.slots[own].count = 1;
        ctx.scene.models[model].attribute |= CONFLICT;
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                0,
                ctx,
            )
            .unwrap();
        assert_eq!(items.slots[1].processor(), Some(Processor::Darts));
        assert!(ctx.scene.models[model].conflict.is_none());
        assert_eq!(items.slots[1].collision.size, 300);
        assert!(ctx.effects.sounds.iter().any(|(_, sound)| *sound == 0x30));
        let position = ctx.scene.models[model].local.translation;
        let count = items.slots[1].trail.as_ref().unwrap().count;
        ctx.effects.sounds.clear();
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                0,
                ctx,
            )
            .unwrap();
        assert_ne!(ctx.scene.models[model].local.translation, position);
        assert_eq!(items.slots[1].trail.as_ref().unwrap().count, count + 1);
        assert!(ctx.effects.sounds.is_empty());
        items.slots[1].flight = Flight::Rolling;
        items.slots[1].rolling.status = RollingStatus::Settled;
        items
            .process(
                1,
                actors,
                &mut crate::ai::Intelligence::default(),
                0,
                0,
                ctx,
            )
            .unwrap();
        assert!(items.slots[1].processor().is_none());
        assert!(items.slots[1].trail.is_none());
        assert!(ctx.effects.sounds.iter().any(|(_, sound)| *sound == 0x31));
    });
}
