use super::*;
use crate::{ai::Intelligence, effects::Effect};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original lure/resurrection callbacks including forced eviction and all emitted particle fields"]
fn support_items_match_original_callbacks() {
    let source = oracle::source("ITEM.C");
    let functions = [
        "static void ProcItemManebue(TItem *item)\n{",
        "void ProcItemKawarimi(TItem *item)",
    ]
    .map(|signature| oracle::function(&source, signature))
    .join("\n");
    let harness=[r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int32_t s32;typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {struct {struct {int t[3];}coord;}locate;} Model;
typedef struct {Model*model;int active_item;} Human;
typedef struct {uint8_t count;} param_drop;
typedef struct Item {Human*owner;void(*proc)(struct Item*);int type;uint8_t mode;union {param_drop drop;}param;} TItem;
enum {ITEM_MODE_DISPOSE=255,MANEBUE_DURATION=30,SE_LURE_FLUTE=0x43,ACTIVE_ITEM_NONE=0};
#define RGB24(r,g,b) ((r)|((g)<<8)|((b)<<16))
#define DISPOSE_ITEM(item) do {item->mode=255;item->proc(item);item->owner=0;item->proc=0;}while(0)
static uint32_t seed;static int calls,EmergencyNotice,sound,particles[20][10],n;
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767;}
#define rand game_rand
static void SoundEx(void*p,int id) {assert(!p);sound=id;}
static void SetBleed(VECTOR*p,SVECTOR*v,int t,int color) {int*q=particles[n++];q[0]=p->vx;q[1]=p->vy;q[2]=p->vz;q[3]=v->vx;q[4]=v->vy;q[5]=v->vz;q[6]=t;q[7]=color&255;q[8]=(color>>8)&255;q[9]=(color>>16)&255;}
"#,&functions,r#"
int main(void) {int a[9];while(scanf("%d",a)==1){for(int i=1;i<9;i++)if(scanf("%d",a+i)!=1)return 2;
Model model={.locate.coord.t={a[4],a[5],a[6]}};Human human={&model,a[7]};TItem item={.owner=&human,.proc=a[0]?ProcItemKawarimi:ProcItemManebue,.type=a[0]?10:18,.mode=a[1],.param.drop={a[2]}};
seed=(uint32_t)a[3];EmergencyNotice=a[8];calls=n=0;sound=-1;
if(item.mode==255){DISPOSE_ITEM((&item));}else item.proc(&item);
printf("%d %d %d %d %d %d %d %d %d",item.mode,item.param.drop.count,item.proc!=0,human.active_item,EmergencyNotice,(int32_t)seed,calls,sound,n);
for(int i=0;i<n;i++)for(int j=0;j<10;j++)printf(" %d",particles[i][j]);puts("");
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for case in 0..512 {
            let resurrection = case % 2 != 0;
            let mode = if resurrection {
                [0, 1, 2, 255][case / 2 % 4]
            } else {
                [0, 1, 1, 255][case / 2 % 4]
            };
            let count = [0, 1, 2, 29, 30, 31, 254, 255][case / 8 % 8];
            let seed = case as u32 * 73 + 7;
            let position = if case % 3 == 0 {
                [i32::MAX, i32::MIN, -7]
            } else {
                [100, -900, 550]
            };
            let active = if case % 5 == 0 { 12 } else { 18 };
            items.dispose(1, actors, ctx).unwrap();
            items
                .initialize(
                    1,
                    if resurrection {
                        Processor::Resurrection
                    } else {
                        Processor::Flute
                    },
                    if resurrection { 10 } else { 18 },
                    Some(player),
                    [0; 3],
                    ctx,
                )
                .unwrap();
            items.slots[1].mode = mode;
            items.slots[1].count = count;
            let owner = actors.get_mut(player);
            owner.active_item = active;
            ctx.scene.set_translation(owner.root_model, position);
            *ctx.random = Random::new(seed);
            *ctx.effects = effects::Effects::default();
            let mut intelligence = Intelligence::default();
            intelligence.alert = 500;
            writeln!(
                input,
                "{} {mode} {count} {seed} {} {} {} {active} 500",
                i32::from(resurrection),
                position[0],
                position[1],
                position[2]
            )
            .unwrap();
            if mode == 255 {
                items.dispose(1, actors, ctx).unwrap();
            } else if resurrection {
                items.resurrection(1, actors, ctx).unwrap();
            } else {
                items.flute(1, actors, &mut intelligence, ctx).unwrap();
            }
            let item = &items.slots[1];
            let particles = ctx
                .effects
                .slots
                .iter()
                .filter_map(|e| {
                    if let Some(Effect::Bleed(p)) = e {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let mut row = vec![
                i32::from(item.mode),
                i32::from(item.count),
                i32::from(item.processor.is_some()),
                i32::from(actors.get(player).active_item),
                intelligence.alert,
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                ctx.effects
                    .sounds
                    .last()
                    .map_or(-1, |(_, id)| i32::from(*id)),
                particles.len() as i32,
            ];
            for p in particles {
                row.extend(p.position);
                row.extend(p.velocity.map(i32::from));
                row.push(i32::from(p.time));
                row.extend(p.color.map(i32::from));
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
#[ignore = "requires TENCHU_DISC; flute lasts thirty updates, plays at full local volume and forced disposal preserves its active flag"]
fn flute_duration_and_forced_eviction_follow_source() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    game.roster.get_mut(player).inventory[18] = 255;
    game.hud.cursor = 18;
    game.intelligence.alert = 500;
    game.step(0).unwrap();
    game.step(crate::input::TRIANGLE).unwrap();
    assert_eq!(game.intelligence.alert, 0);
    assert_eq!(game.player().active_item, 18);
    assert_eq!(game.player().inventory[18], 255);
    assert!(
        game.sounds()
            .unwrap()
            .iter()
            .any(|sound| sound.id == 0x43 && sound.volume == 126 && sound.pan == 64)
    );
    for _ in 0..29 {
        game.step(0).unwrap();
        assert_eq!(game.player().active_item, 18);
    }
    game.step(0).unwrap();
    assert_eq!(game.player().active_item, 0);
    game.step(crate::input::TRIANGLE).unwrap();
    assert_eq!(game.player().active_item, 18);
    with_items(&mut game, |items, ctx, actors| {
        let id = items
            .slots
            .iter()
            .position(|item| item.processor == Some(Processor::Flute))
            .unwrap();
        items.dispose(id, actors, ctx).unwrap();
    });
    for _ in 0..40 {
        game.step(0).unwrap();
    }
    assert_eq!(game.player().active_item, 18);
}
