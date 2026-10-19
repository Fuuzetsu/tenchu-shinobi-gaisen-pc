use super::*;
use crate::{effects::Effect, ids::MOT_DAMAGE};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original charm timer, attached pulses and unconditional active-item cleanup"]
fn charm_timer_and_cleanup_match_original_c() {
    let source = oracle::source("ITEM.C");
    let function = oracle::function(&source, "void ProcItemGosin(TItem *item)");
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int TItemType; typedef int16_t s16; typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {int mid,count,loop;} MotionManager;
typedef struct Model {int locate;struct Model*object[2];} Model;
typedef struct {MotionManager*motion;Model*model;int active_item;} Humanoid;
typedef struct Item {Humanoid*owner;void(*proc)(struct Item*);int type;uint8_t mode;Model*locate;union {struct {uint16_t count;}gosin;}param;} TItem;
typedef struct {int type;Humanoid*user;VECTOR start,end;} PARAM_ITEM_LAUNCH;
enum {ITEM_MODE_DISPOSE=255,ACTIVE_ITEM_NONE=0,MOT_ITEM_KAENGEKI=0xf04,MOTION_MOVE_APPLY=1,SE_ITEM_USE=0x4c,MODEL_PART_TORSO=1,GOSIN_DURATION=450,FIXED_ONE=4096,COLOR_GRAY=0x808080,IMPACT_SPRITE_GOSIN=4};
#define RGB24(r,g,b) ((r)|((g)<<8)|((b)<<16))
#define copyVector(to,from) (*(to)=*(from))
#define DISPOSE_ITEM(item) do {item->mode=255;item->proc(item);item->owner=0;item->proc=0;}while(0)
VECTOR vec_y_n1200_z_400={0,-1200,400,0};
static uint32_t seed;static int calls,pulse[15],n;static int*parent;
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767;}
#define rand game_rand
static void SetNowMotion(Humanoid*h,int m,int move) {assert(0);}
static void Sound(Humanoid*h,int s) {assert(0);}
static VECTOR*GetAbsolutePosition(Model*m,int x,int y,int z) {assert(0);return 0;}
static void ReqItemDrop(PARAM_ITEM_LAUNCH*p) {assert(0);}
static void NowReturnNormal(Humanoid*h) {assert(0);}
static void SetBleeds(VECTOR*v,int spread,int speed,int count,int time,int color) {assert(0);}
static void set_impact_ex_(VECTOR*v,int*p,int s1,int s2,int c1,int c2,s16 r,int speed,int time,int sprite) {
int a[]={p==parent,v->vx,v->vy,v->vz,s1,s2,c1&255,(c1>>8)&255,(c1>>16)&255,c2&255,(c2>>8)&255,(c2>>16)&255,r,speed,time};
for(int i=0;i<15;i++)pulse[i]=a[i];n=sprite;}
"#, function, r#"
int main(void) {int mode,count,active,initial;while(scanf("%d %d %d %d",&mode,&count,&active,&initial)==4) {
Model model={0};MotionManager motion={0};Humanoid human={&motion,&model,active};
TItem item={.owner=&human,.proc=ProcItemGosin,.type=12,.mode=mode,.param.gosin={count}};
seed=(uint32_t)initial;calls=n=0;parent=&model.locate;
if(mode==255) {DISPOSE_ITEM((&item));}else ProcItemGosin(&item);
printf("%d %d %d %d %d %d %d",item.mode,item.param.gosin.count,item.proc!=0,human.active_item,(int32_t)seed,calls,n);
if(n)for(int i=0;i<15;i++)printf(" %d",pulse[i]);puts("");}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for count in [0, 1, 2, 64, 65, 128, 129, 449, 450, 32768, 32769, 65535] {
            for mode in [2, 255] {
                for active in [0, 12, 18] {
                    items.dispose(1, actors, ctx).unwrap();
                    items
                        .initialize(1, Processor::Charm, 12, Some(player), [0; 3], ctx)
                        .unwrap();
                    items.slots[1].mode = mode;
                    items.slots[1].charm_remaining = count;
                    let owner = actors.get_mut(player);
                    owner.active_item = active;
                    let root = owner.root_model;
                    let seed = u32::from(count) * 73 + 7;
                    *ctx.random = Random::new(seed);
                    *ctx.effects = effects::Effects::default();
                    writeln!(input, "{mode} {count} {active} {seed}").unwrap();
                    if mode == 255 {
                        items.dispose(1, actors, ctx).unwrap();
                    } else {
                        items.charm(1, actors, player, ctx).unwrap();
                    }
                    let item = &items.slots[1];
                    let pulse = ctx.effects.slots.iter().find_map(|e| {
                        if let Some(Effect::Impact(p)) = e {
                            Some(p)
                        } else {
                            None
                        }
                    });
                    let mut row = vec![
                        item.mode as i32,
                        item.charm_remaining as i32,
                        i32::from(item.processor.is_some()),
                        actors.get(player).active_item as i32,
                        ctx.random.seed as i32,
                        ctx.random.calls as i32,
                        pulse.map_or(0, |p| p.sprite as i32),
                    ];
                    if let Some(p) = pulse {
                        row.push(i32::from(p.parent == Some(root)));
                        row.extend(p.position);
                        row.extend(p.size.map(i32::from));
                        row.extend(p.color.into_iter().flatten().map(i32::from));
                        row.extend([p.rotation as i32, p.speed as i32, p.time as i32]);
                    }
                    expected.push(row);
                }
            }
        }
    });
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (a, e)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(a, e, "case {case}: {}", input.lines().nth(case).unwrap());
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; interrupted charm casting refunds a pickup and clears an unrelated active item"]
fn interrupted_charm_refunds_and_clears_active_item() {
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        items
            .initialize(1, Processor::Charm, 12, Some(player), [1000, -1200, 0], ctx)
            .unwrap();
        items.slots[1].mode = 1;
        actors.get_mut(player).enter(ctx, MOT_DAMAGE, true);
        actors.get_mut(player).active_item = 18;
        let mut random = *ctx.random;
        items.charm(1, actors, player, ctx).unwrap();
        assert!(items.slots[1].processor.is_none());
        assert_eq!(actors.get(player).active_item, 0);
        let drops: Vec<_> = items
            .slots
            .iter()
            .filter(|i| i.processor == Some(Processor::Pickup) && i.kind == 12)
            .collect();
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].owner, Some(player));
        assert_eq!(
            ctx.scene.models[drops[0].model].local.translation,
            [1000, -1200, 0]
        );
        assert_eq!(
            drops[0].rolling.velocity,
            [
                (random.next15() % 200 - 100) as i16,
                (random.next15() % 100 - 200) as i16,
                (random.next15() % 200 - 100) as i16
            ]
        );
        assert_eq!(*ctx.random, random);
    });
}
